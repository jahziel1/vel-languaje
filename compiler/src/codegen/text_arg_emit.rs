use wasm_encoder::Instruction;

use crate::parser::ast::{Expr, StringPart};

use super::emit;
use super::{
    CodeGen, RT_API_FIELD_STR, RT_LIST_COUNT, RT_STR_BEGIN, RT_STR_DONE, RT_STR_LIT, RT_STR_NUM,
    RT_TEXT_CONTENT, RT_TEXT_STATE_GET,
};

impl CodeGen {
    /// Emit a text argument: static strings use `text_content`, interpolated use the str_* builder.
    pub(super) fn emit_text_arg(
        &mut self,
        expr: &Expr,
        ctx: &mut emit::Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        // Text state variable: text(name) → str_begin + text_state_get + str_done
        if let Expr::Ident(var_name) = expr {
            let gkey = format!("{}/{}", ctx.page, var_name);
            if self.is_text_state(&gkey) {
                let (kptr, klen) = self.intern(&gkey);
                out.push(Instruction::Call(RT_STR_BEGIN));
                out.push(Instruction::I32Const(kptr as i32));
                out.push(Instruction::I32Const(klen as i32));
                out.push(Instruction::Call(RT_TEXT_STATE_GET));
                out.push(Instruction::Call(RT_STR_DONE));
                return;
            }
        }

        // List item or API field: text(item.name) / text(data.name)
        if let Expr::Field(obj, field) | Expr::OptField(obj, field) = expr
            && let Expr::Ident(var_name) = obj.as_ref()
        {
            if let Some(&(req_local, idx_local)) = ctx.list_bindings.get(var_name.as_str()) {
                out.push(Instruction::Call(RT_STR_BEGIN));
                self.emit_list_item_str(req_local, idx_local, field, out);
                out.push(Instruction::Call(RT_STR_DONE));
                return;
            }
            if let Some(&local_idx) = ctx.api_bindings.get(var_name.as_str()) {
                let (fptr, flen) = self.intern(field);
                out.push(Instruction::Call(RT_STR_BEGIN));
                out.push(Instruction::LocalGet(local_idx));
                out.push(Instruction::I32TruncF64S);
                out.push(Instruction::I32Const(fptr as i32));
                out.push(Instruction::I32Const(flen as i32));
                out.push(Instruction::Call(RT_API_FIELD_STR));
                out.push(Instruction::Call(RT_STR_DONE));
                return;
            }
        }

        if let Expr::Str(parts) = expr {
            let has_interp = parts.iter().any(|p| matches!(p, StringPart::Interp(_)));
            if has_interp {
                out.push(Instruction::Call(RT_STR_BEGIN));
                for part in parts {
                    match part {
                        StringPart::Lit(s) => {
                            let (ptr, len) = self.intern(s);
                            out.push(Instruction::I32Const(ptr as i32));
                            out.push(Instruction::I32Const(len as i32));
                            out.push(Instruction::Call(RT_STR_LIT));
                        }
                        StringPart::Interp(e) => {
                            // Text state ident in interpolation: append via text_state_get
                            if let Expr::Ident(var_name) = e.as_ref() {
                                let gkey = format!("{}/{}", ctx.page, var_name);
                                if self.is_text_state(&gkey) {
                                    let (kptr, klen) = self.intern(&gkey);
                                    out.push(Instruction::I32Const(kptr as i32));
                                    out.push(Instruction::I32Const(klen as i32));
                                    out.push(Instruction::Call(RT_TEXT_STATE_GET));
                                    continue;
                                }
                            }
                            // List item or API string field in interpolation
                            let handled = if let Expr::Field(obj, field)
                            | Expr::OptField(obj, field) = e.as_ref()
                                && let Expr::Ident(var_name) = obj.as_ref()
                            {
                                if let Some(&(req_l, idx_l)) =
                                    ctx.list_bindings.get(var_name.as_str())
                                {
                                    self.emit_list_item_str(req_l, idx_l, field, out);
                                    true
                                } else if let Some(&api_local) =
                                    ctx.api_bindings.get(var_name.as_str())
                                {
                                    if field == "length" {
                                        // data.length in interpolation: emit as number
                                        let req_local = ctx.alloc_i32_local();
                                        out.push(Instruction::LocalGet(api_local));
                                        out.push(Instruction::I32TruncF64S);
                                        out.push(Instruction::LocalSet(req_local));
                                        out.push(Instruction::LocalGet(req_local));
                                        out.push(Instruction::Call(RT_LIST_COUNT));
                                        out.push(Instruction::F64ConvertI32S);
                                        out.push(Instruction::Call(RT_STR_NUM));
                                        true
                                    } else {
                                        let (fptr, flen) = self.intern(field);
                                        out.push(Instruction::LocalGet(api_local));
                                        out.push(Instruction::I32TruncF64S);
                                        out.push(Instruction::I32Const(fptr as i32));
                                        out.push(Instruction::I32Const(flen as i32));
                                        out.push(Instruction::Call(RT_API_FIELD_STR));
                                        true
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            };
                            if !handled {
                                self.emit_expr_f64(e, ctx, out);
                                out.push(Instruction::Call(RT_STR_NUM));
                            }
                        }
                    }
                }
                out.push(Instruction::Call(RT_STR_DONE));
                return;
            }
        }
        // Static string fast path
        let (ptr, len) = extract_static_string(expr)
            .map(|s| {
                let (p, l) = self.intern(&s);
                (p as i32, l as i32)
            })
            .unwrap_or((0, 0));
        out.push(Instruction::I32Const(ptr));
        out.push(Instruction::I32Const(len));
        out.push(Instruction::Call(RT_TEXT_CONTENT));
    }
}

impl CodeGen {
    /// Emit only the parts of an interpolated Expr::Str — no str_begin, no str_done.
    /// Caller wraps with str_begin + [str_done | print_str_built].
    pub(super) fn emit_text_arg_parts(
        &mut self,
        expr: &Expr,
        ctx: &mut emit::Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let Expr::Str(parts) = expr else { return };
        for part in parts {
            match part {
                StringPart::Lit(s) => {
                    let (ptr, len) = self.intern(s);
                    out.push(Instruction::I32Const(ptr as i32));
                    out.push(Instruction::I32Const(len as i32));
                    out.push(Instruction::Call(RT_STR_LIT));
                }
                StringPart::Interp(e) => {
                    if let Expr::Ident(var_name) = e.as_ref() {
                        let gkey = format!("{}/{}", ctx.page, var_name);
                        if self.is_text_state(&gkey) {
                            let (kptr, klen) = self.intern(&gkey);
                            out.push(Instruction::I32Const(kptr as i32));
                            out.push(Instruction::I32Const(klen as i32));
                            out.push(Instruction::Call(RT_TEXT_STATE_GET));
                            continue;
                        }
                    }
                    self.emit_expr_f64(e, ctx, out);
                    out.push(Instruction::Call(RT_STR_NUM));
                }
            }
        }
    }
}

pub(super) fn extract_static_string(expr: &Expr) -> Option<String> {
    if let Expr::Str(parts) = expr {
        let mut s = String::new();
        for part in parts {
            match part {
                StringPart::Lit(lit) => s.push_str(lit),
                StringPart::Interp(_) => return None,
            }
        }
        Some(s)
    } else {
        None
    }
}
