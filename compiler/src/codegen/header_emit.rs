use wasm_encoder::Instruction;

use crate::parser::ast::Expr;

use super::api_emit::extract_static_str;
use super::{
    CodeGen, RT_HEADER_BEGIN, RT_HEADER_DONE, RT_HEADER_FIELD, RT_HEADER_FIELD_STR, RT_STR_BEGIN,
    RT_STR_LIT, RT_STR_NUM, RT_TEXT_STATE_GET,
};

impl CodeGen {
    /// Emit header_begin / (store headers) / (inline headers) / header_done.
    /// Called before every api_fetch — skipped entirely if no headers to send.
    pub(super) fn emit_all_headers(
        &mut self,
        inline: Option<&Expr>,
        out: &mut Vec<Instruction<'static>>,
    ) {
        // Snapshot store headers to avoid borrowing self mutably twice.
        let store_headers: Vec<(String, Vec<(String, Expr)>)> = self
            .store_api_headers
            .iter()
            .map(|(sname, entries)| (sname.clone(), entries.clone()))
            .collect();

        let has_store = store_headers.iter().any(|(_, e)| !e.is_empty());
        let has_inline = inline
            .and_then(|e| {
                if let Expr::Object(entries) = e {
                    Some(!entries.is_empty())
                } else {
                    None
                }
            })
            .unwrap_or(false);

        if !has_store && !has_inline {
            return;
        }

        out.push(Instruction::Call(RT_HEADER_BEGIN));

        // Store-level headers (support dynamic values via str_builder).
        for (store_name, entries) in &store_headers {
            for (key, val_expr) in entries {
                let (key_ptr, key_len) = self.intern(key);
                if let Some(s) = extract_static_str(val_expr) {
                    let (val_ptr, val_len) = self.intern(&s);
                    out.push(Instruction::I32Const(key_ptr as i32));
                    out.push(Instruction::I32Const(key_len as i32));
                    out.push(Instruction::I32Const(val_ptr as i32));
                    out.push(Instruction::I32Const(val_len as i32));
                    out.push(Instruction::Call(RT_HEADER_FIELD));
                } else if let Expr::Str(parts) = val_expr {
                    // Interpolated — build via str_begin/parts, then header_field_str.
                    out.push(Instruction::Call(RT_STR_BEGIN));
                    for part in parts {
                        match part {
                            crate::parser::ast::StringPart::Lit(s) => {
                                let (ptr, len) = self.intern(s);
                                out.push(Instruction::I32Const(ptr as i32));
                                out.push(Instruction::I32Const(len as i32));
                                out.push(Instruction::Call(RT_STR_LIT));
                            }
                            crate::parser::ast::StringPart::Interp(expr) => {
                                if let Expr::Ident(var) = expr.as_ref() {
                                    let ts_key = format!("{}/{}", store_name, var);
                                    if self.text_state.contains(&ts_key) {
                                        let (kp, kl) = self.intern(&ts_key);
                                        out.push(Instruction::I32Const(kp as i32));
                                        out.push(Instruction::I32Const(kl as i32));
                                        out.push(Instruction::Call(RT_TEXT_STATE_GET));
                                    } else if let Some(&gidx) = self.global_map.get(&ts_key) {
                                        out.push(Instruction::GlobalGet(gidx));
                                        out.push(Instruction::Call(RT_STR_NUM));
                                    }
                                }
                            }
                        }
                    }
                    out.push(Instruction::I32Const(key_ptr as i32));
                    out.push(Instruction::I32Const(key_len as i32));
                    out.push(Instruction::Call(RT_HEADER_FIELD_STR));
                }
            }
        }

        // Inline headers from the call site — static strings only.
        if let Some(Expr::Object(entries)) = inline {
            for entry in entries {
                let crate::parser::ast::ObjectEntry::Field(key, val) = entry else {
                    continue;
                };
                if let Some(val_str) = extract_static_str(val) {
                    let (key_ptr, key_len) = self.intern(key);
                    let (val_ptr, val_len) = self.intern(&val_str);
                    out.push(Instruction::I32Const(key_ptr as i32));
                    out.push(Instruction::I32Const(key_len as i32));
                    out.push(Instruction::I32Const(val_ptr as i32));
                    out.push(Instruction::I32Const(val_len as i32));
                    out.push(Instruction::Call(RT_HEADER_FIELD));
                }
            }
        }

        out.push(Instruction::Call(RT_HEADER_DONE));
    }
}
