use wasm_encoder::Instruction;

use crate::parser::ast::{Expr, StringPart};

use super::emit::Ctx;
use super::{CodeGen, RT_STR_BEGIN, RT_TEXT_STATE_SET, RT_TEXT_STATE_SET_BUILT};

impl CodeGen {
    /// Emit a write to a Text state variable.
    /// Static string literals use text_state_set directly.
    /// Interpolated strings build via str_begin/parts/text_state_set_built.
    pub(super) fn emit_text_state_write(
        &mut self,
        key: &str,
        value_expr: &Expr,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let (kptr, klen) = self.intern(key);
        if let Some(s) = extract_static_text(value_expr) {
            let (vptr, vlen) = self.intern(&s);
            out.push(Instruction::I32Const(kptr as i32));
            out.push(Instruction::I32Const(klen as i32));
            out.push(Instruction::I32Const(vptr as i32));
            out.push(Instruction::I32Const(vlen as i32));
            out.push(Instruction::Call(RT_TEXT_STATE_SET));
        } else if let Expr::Str(parts) = value_expr {
            out.push(Instruction::Call(RT_STR_BEGIN));
            for part in parts {
                match part {
                    StringPart::Lit(s) => {
                        let (ptr, len) = self.intern(s);
                        out.push(Instruction::I32Const(ptr as i32));
                        out.push(Instruction::I32Const(len as i32));
                        out.push(Instruction::Call(super::RT_STR_LIT));
                    }
                    StringPart::Interp(e) => {
                        self.emit_expr_f64(e, ctx, out);
                        out.push(Instruction::Call(super::RT_STR_NUM));
                    }
                }
            }
            out.push(Instruction::I32Const(kptr as i32));
            out.push(Instruction::I32Const(klen as i32));
            out.push(Instruction::Call(RT_TEXT_STATE_SET_BUILT));
        }
    }
}

fn extract_static_text(expr: &Expr) -> Option<String> {
    let Expr::Str(parts) = expr else { return None };
    let mut s = String::new();
    for part in parts {
        match part {
            StringPart::Lit(lit) => s.push_str(lit),
            StringPart::Interp(_) => return None,
        }
    }
    Some(s)
}
