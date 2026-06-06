use wasm_encoder::{BlockType, Instruction, ValType};

use crate::parser::ast::{Expr, ObjectEntry, Stmt, StringPart};

use super::emit::Ctx;
use super::{
    CodeGen, RT_API_FETCH, RT_API_FETCH_DYN, RT_API_POLL, RT_API_URL_CHANGED, RT_BODY_BEGIN,
    RT_BODY_DONE, RT_BODY_FIELD_BOOL, RT_BODY_FIELD_NUM, RT_BODY_FIELD_STR, RT_URL_BEGIN,
    RT_URL_DONE, RT_URL_LIT, RT_URL_NUM, is_async_state,
};

// ── URL kind ──────────────────────────────────────────────────────────────────

pub(super) enum ApiUrl {
    /// Compile-time constant — interned into the data section.
    Static(String),
    /// Contains interpolated variables — must be built at runtime.
    Dynamic(Vec<StringPart>),
}

impl CodeGen {
    /// Emit fetch+poll calls for every async state entry in `stmts`.
    /// Must run before the main statement loop so state is ready on first render.
    pub(super) fn emit_api_inits(
        &mut self,
        owner: &str,
        stmts: &[Stmt],
        _ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        for stmt in stmts {
            if let Stmt::State(state) = stmt {
                for entry in &state.entries {
                    if !is_async_state(&entry.value) {
                        continue;
                    }
                    let Some((method, url, body, headers)) = extract_api_call(&entry.value) else {
                        continue;
                    };
                    let reqid_key = format!("{}/{}__reqid", owner, entry.name);
                    let status_key = format!("{}/{}", owner, entry.name);
                    let (Some(&reqid_gidx), Some(&status_gidx)) = (
                        self.global_map.get(&reqid_key),
                        self.global_map.get(&status_key),
                    ) else {
                        continue;
                    };
                    let (method_ptr, method_len) = self.intern(&method);

                    match url {
                        ApiUrl::Static(url_str) => {
                            let (url_ptr, url_len) = self.intern(&url_str);

                            // if reqid == 0: fire the request
                            out.push(Instruction::GlobalGet(reqid_gidx));
                            out.push(Instruction::I32Const(0));
                            out.push(Instruction::I32Eq);
                            out.push(Instruction::If(BlockType::Empty));
                            self.emit_all_headers(headers.as_ref(), out);
                            if let Some(body_expr) = body {
                                self.emit_body_build(owner, &body_expr, out);
                            }
                            out.push(Instruction::I32Const(method_ptr as i32));
                            out.push(Instruction::I32Const(method_len as i32));
                            out.push(Instruction::I32Const(url_ptr as i32));
                            out.push(Instruction::I32Const(url_len as i32));
                            out.push(Instruction::Call(RT_API_FETCH));
                            out.push(Instruction::GlobalSet(reqid_gidx));
                            out.push(Instruction::End);
                        }
                        ApiUrl::Dynamic(parts) => {
                            // Always build the current URL into pending_url
                            self.emit_url_build(owner, &parts, out);

                            // If URL changed for this reqid → reset reqid to 0
                            out.push(Instruction::GlobalGet(reqid_gidx));
                            out.push(Instruction::Call(RT_API_URL_CHANGED));
                            out.push(Instruction::If(BlockType::Empty));
                            out.push(Instruction::I32Const(0));
                            out.push(Instruction::GlobalSet(reqid_gidx));
                            out.push(Instruction::End);

                            // if reqid == 0: fire the request using pending_url
                            out.push(Instruction::GlobalGet(reqid_gidx));
                            out.push(Instruction::I32Const(0));
                            out.push(Instruction::I32Eq);
                            out.push(Instruction::If(BlockType::Empty));
                            self.emit_all_headers(headers.as_ref(), out);
                            if let Some(body_expr) = body {
                                self.emit_body_build(owner, &body_expr, out);
                            }
                            out.push(Instruction::I32Const(method_ptr as i32));
                            out.push(Instruction::I32Const(method_len as i32));
                            out.push(Instruction::Call(RT_API_FETCH_DYN));
                            out.push(Instruction::GlobalSet(reqid_gidx));
                            out.push(Instruction::End);
                        }
                    }

                    // update status = api_poll(reqid) on every render
                    out.push(Instruction::GlobalGet(reqid_gidx));
                    out.push(Instruction::Call(RT_API_POLL));
                    out.push(Instruction::GlobalSet(status_gidx));
                }
            }
        }
    }

    /// Emit url_begin / url_lit / url_num / url_done for an interpolated URL string.
    fn emit_url_build(
        &mut self,
        owner: &str,
        parts: &[StringPart],
        out: &mut Vec<Instruction<'static>>,
    ) {
        out.push(Instruction::Call(RT_URL_BEGIN));
        for part in parts {
            match part {
                StringPart::Lit(s) => {
                    let (ptr, len) = self.intern(s);
                    out.push(Instruction::I32Const(ptr as i32));
                    out.push(Instruction::I32Const(len as i32));
                    out.push(Instruction::Call(RT_URL_LIT));
                }
                StringPart::Interp(expr) => {
                    if let Expr::Ident(var) = expr.as_ref() {
                        let gkey = format!("{}/{}", owner, var);
                        if let Some(&gidx) = self.global_map.get(&gkey)
                            && self.globals[gidx as usize].val_type == ValType::F64
                        {
                            out.push(Instruction::GlobalGet(gidx));
                            out.push(Instruction::Call(RT_URL_NUM));
                        }
                    }
                }
            }
        }
        out.push(Instruction::Call(RT_URL_DONE));
    }

    /// Emit body_begin / body_field_* / body_done for an Object literal.
    /// Supports: Number literals, Bool literals, static String literals, and Ident globals.
    fn emit_body_build(
        &mut self,
        owner: &str,
        body_expr: &Expr,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let Expr::Object(entries) = body_expr else {
            return;
        };
        out.push(Instruction::Call(RT_BODY_BEGIN));
        for entry in entries {
            let ObjectEntry::Field(key, val) = entry else {
                continue;
            };
            let (key_ptr, key_len) = self.intern(key);
            match val {
                Expr::Number(n) => {
                    out.push(Instruction::I32Const(key_ptr as i32));
                    out.push(Instruction::I32Const(key_len as i32));
                    out.push(Instruction::F64Const((*n).into()));
                    out.push(Instruction::Call(RT_BODY_FIELD_NUM));
                }
                Expr::Bool(b) => {
                    out.push(Instruction::I32Const(key_ptr as i32));
                    out.push(Instruction::I32Const(key_len as i32));
                    out.push(Instruction::I32Const(if *b { 1 } else { 0 }));
                    out.push(Instruction::Call(RT_BODY_FIELD_BOOL));
                }
                Expr::Str(_) => {
                    if let Some(s) = extract_static_str(val) {
                        let (val_ptr, val_len) = self.intern(&s);
                        out.push(Instruction::I32Const(key_ptr as i32));
                        out.push(Instruction::I32Const(key_len as i32));
                        out.push(Instruction::I32Const(val_ptr as i32));
                        out.push(Instruction::I32Const(val_len as i32));
                        out.push(Instruction::Call(RT_BODY_FIELD_STR));
                    }
                }
                Expr::Ident(var) => {
                    let gkey = format!("{}/{}", owner, var);
                    if let Some(&gidx) = self.global_map.get(&gkey) {
                        if self.globals[gidx as usize].val_type == ValType::I32 {
                            out.push(Instruction::I32Const(key_ptr as i32));
                            out.push(Instruction::I32Const(key_len as i32));
                            out.push(Instruction::GlobalGet(gidx));
                            out.push(Instruction::Call(RT_BODY_FIELD_BOOL));
                        } else {
                            out.push(Instruction::I32Const(key_ptr as i32));
                            out.push(Instruction::I32Const(key_len as i32));
                            out.push(Instruction::GlobalGet(gidx));
                            out.push(Instruction::Call(RT_BODY_FIELD_NUM));
                        }
                    }
                }
                _ => {}
            }
        }
        out.push(Instruction::Call(RT_BODY_DONE));
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract (method, ApiUrl, body, headers) from `try api.method(url[, body][, headers: {...}])`.
pub(super) fn extract_api_call(
    expr: &Expr,
) -> Option<(String, ApiUrl, Option<Expr>, Option<Expr>)> {
    let Expr::Try(inner) = expr else {
        return None;
    };
    let Expr::Call(callee, args, _) = inner.as_ref() else {
        return None;
    };
    let Expr::Field(obj, method) = callee.as_ref() else {
        return None;
    };
    let Expr::Ident(api_name) = obj.as_ref() else {
        return None;
    };
    if api_name != "api" {
        return None;
    }
    let url_arg = &args.first()?.value;
    let Expr::Str(parts) = url_arg else {
        return None;
    };
    let has_interp = parts.iter().any(|p| matches!(p, StringPart::Interp(_)));
    let url = if has_interp {
        ApiUrl::Dynamic(parts.clone())
    } else {
        let s: String = parts
            .iter()
            .map(|p| match p {
                StringPart::Lit(s) => s.as_str(),
                StringPart::Interp(_) => "",
            })
            .collect();
        ApiUrl::Static(s)
    };
    // Positional arg after URL = body; named arg "headers" = headers.
    let body = args
        .iter()
        .skip(1)
        .find(|a| a.name.is_none())
        .map(|a| a.value.clone());
    let headers = args
        .iter()
        .find(|a| a.name.as_deref() == Some("headers"))
        .map(|a| a.value.clone());
    Some((method.clone(), url, body, headers))
}

pub(super) fn extract_static_str(expr: &Expr) -> Option<String> {
    let Expr::Str(parts) = expr else {
        return None;
    };
    let mut s = String::new();
    for part in parts {
        match part {
            StringPart::Lit(lit) => s.push_str(lit),
            StringPart::Interp(_) => return None,
        }
    }
    Some(s)
}
