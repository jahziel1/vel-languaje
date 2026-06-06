use wasm_encoder::{Instruction, ValType};

use crate::parser::ast::{BinOp, Expr, UnOp};

use super::emit::Ctx;
use super::{CodeGen, RT_API_FIELD_BOOL, RT_LIST_COUNT, RT_TEXT_STATE_BOOL};

impl CodeGen {
    pub(super) fn emit_expr_i32(
        &mut self,
        expr: &Expr,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        match expr {
            Expr::Bool(b) => out.push(Instruction::I32Const(if *b { 1 } else { 0 })),
            Expr::None => out.push(Instruction::I32Const(0)),

            // Comparisons produce i32 directly — no f64 conversion needed here
            Expr::BinOp(lhs, op, rhs) => match op {
                BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                    self.emit_expr_f64(lhs, ctx, out);
                    self.emit_expr_f64(rhs, ctx, out);
                    out.push(match op {
                        BinOp::Eq => Instruction::F64Eq,
                        BinOp::NotEq => Instruction::F64Ne,
                        BinOp::Lt => Instruction::F64Lt,
                        BinOp::Gt => Instruction::F64Gt,
                        BinOp::LtEq => Instruction::F64Le,
                        BinOp::GtEq => Instruction::F64Ge,
                        _ => unreachable!(),
                    });
                }
                BinOp::And => {
                    self.emit_expr_i32(lhs, ctx, out);
                    self.emit_expr_i32(rhs, ctx, out);
                    out.push(Instruction::I32And);
                }
                BinOp::Or => {
                    self.emit_expr_i32(lhs, ctx, out);
                    self.emit_expr_i32(rhs, ctx, out);
                    out.push(Instruction::I32Or);
                }
                // Arithmetic in a boolean context: truthy if nonzero
                _ => {
                    self.emit_expr_f64(lhs, ctx, out);
                    self.emit_expr_f64(rhs, ctx, out);
                    out.push(match op {
                        BinOp::Add => Instruction::F64Add,
                        BinOp::Sub => Instruction::F64Sub,
                        BinOp::Mul => Instruction::F64Mul,
                        BinOp::Div => Instruction::F64Div,
                        _ => unreachable!(),
                    });
                    out.push(Instruction::F64Const(0.0f64.into()));
                    out.push(Instruction::F64Ne);
                }
            },

            // `not x`
            Expr::UnOp(UnOp::Not, operand) => {
                self.emit_expr_i32(operand, ctx, out);
                out.push(Instruction::I32Eqz);
            }

            // Variables: read global/local and convert f64 → i32 if needed
            Expr::Ident(name) => {
                let gkey = format!("{}/{}", ctx.page, name);
                if self.is_text_state(&gkey) {
                    let (kptr, klen) = self.intern(&gkey);
                    out.push(Instruction::I32Const(kptr as i32));
                    out.push(Instruction::I32Const(klen as i32));
                    out.push(Instruction::Call(RT_TEXT_STATE_BOOL));
                } else if let Some(&gidx) = self.global_map.get(&gkey) {
                    out.push(Instruction::GlobalGet(gidx));
                    if self.globals[gidx as usize].val_type == ValType::F64 {
                        out.push(Instruction::F64Const(0.0f64.into()));
                        out.push(Instruction::F64Ne);
                    }
                } else if let Some(&pidx) = ctx.params.get(name.as_str()) {
                    out.push(Instruction::LocalGet(pidx));
                    out.push(Instruction::F64Const(0.0f64.into()));
                    out.push(Instruction::F64Ne);
                } else if let Some(&lidx) = ctx.locals.get(name.as_str()) {
                    out.push(Instruction::LocalGet(lidx));
                    out.push(Instruction::F64Const(0.0f64.into()));
                    out.push(Instruction::F64Ne);
                } else {
                    out.push(Instruction::I32Const(0));
                }
            }

            Expr::Field(obj, field) | Expr::OptField(obj, field) => {
                if let Expr::Ident(var_name) = obj.as_ref() {
                    // data.isEmpty → (list_count(req) == 0)
                    if field == "isEmpty"
                        && let Some(&api_local) = ctx.api_bindings.get(var_name.as_str())
                    {
                        let req_local = ctx.alloc_i32_local();
                        out.push(Instruction::LocalGet(api_local));
                        out.push(Instruction::I32TruncF64S);
                        out.push(Instruction::LocalSet(req_local));
                        out.push(Instruction::LocalGet(req_local));
                        out.push(Instruction::Call(RT_LIST_COUNT));
                        out.push(Instruction::I32Eqz);
                        return;
                    }
                    if let Some(&(req_l, idx_l)) = ctx.list_bindings.get(var_name.as_str()) {
                        self.emit_list_item_bool(req_l, idx_l, field, out);
                        return;
                    }
                    if let Some(&local_idx) = ctx.api_bindings.get(var_name.as_str()) {
                        let (fptr, flen) = self.intern(field);
                        out.push(Instruction::LocalGet(local_idx));
                        out.push(Instruction::I32TruncF64S);
                        out.push(Instruction::I32Const(fptr as i32));
                        out.push(Instruction::I32Const(flen as i32));
                        out.push(Instruction::Call(RT_API_FIELD_BOOL));
                        return;
                    }
                    // Store field in bool context: with type conversion if needed
                    if self.store_names.contains(var_name.as_str()) {
                        let store_key = format!("{}/{}", var_name, field);
                        if let Some(&gidx) = self.global_map.get(&store_key) {
                            out.push(Instruction::GlobalGet(gidx));
                            if self.globals[gidx as usize].val_type == ValType::F64 {
                                out.push(Instruction::F64Const(0.0f64.into()));
                                out.push(Instruction::F64Ne);
                            }
                            return;
                        }
                        let derived =
                            self.store_derived
                                .get(var_name.as_str())
                                .and_then(|entries| {
                                    entries
                                        .iter()
                                        .find(|(n, _)| n == field)
                                        .map(|(_, e)| e.clone())
                                });
                        if let Some(expr) = derived {
                            let saved_page = ctx.page.clone();
                            ctx.page = var_name.clone();
                            self.emit_expr_i32(&expr, ctx, out);
                            ctx.page = saved_page;
                            return;
                        }
                        out.push(Instruction::I32Const(0));
                        return;
                    }
                }
                self.emit_expr_push(expr, ctx, out);
            }

            // Fallback: emit as-is (Call stubs return I32Const(0), which is fine)
            _ => {
                self.emit_expr_push(expr, ctx, out);
            }
        }
    }

    pub(super) fn emit_binop_expr(
        &mut self,
        lhs: &Expr,
        op: &BinOp,
        rhs: &Expr,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                self.emit_expr_f64(lhs, ctx, out);
                self.emit_expr_f64(rhs, ctx, out);
                out.push(match op {
                    BinOp::Add => Instruction::F64Add,
                    BinOp::Sub => Instruction::F64Sub,
                    BinOp::Mul => Instruction::F64Mul,
                    BinOp::Div => Instruction::F64Div,
                    _ => unreachable!(),
                });
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                self.emit_expr_f64(lhs, ctx, out);
                self.emit_expr_f64(rhs, ctx, out);
                out.push(match op {
                    BinOp::Eq => Instruction::F64Eq,
                    BinOp::NotEq => Instruction::F64Ne,
                    BinOp::Lt => Instruction::F64Lt,
                    BinOp::Gt => Instruction::F64Gt,
                    BinOp::LtEq => Instruction::F64Le,
                    BinOp::GtEq => Instruction::F64Ge,
                    _ => unreachable!(),
                });
                out.push(Instruction::F64ConvertI32U);
            }
            BinOp::And => {
                self.emit_expr_i32(lhs, ctx, out);
                self.emit_expr_i32(rhs, ctx, out);
                out.push(Instruction::I32And);
                out.push(Instruction::F64ConvertI32U);
            }
            BinOp::Or => {
                self.emit_expr_i32(lhs, ctx, out);
                self.emit_expr_i32(rhs, ctx, out);
                out.push(Instruction::I32Or);
                out.push(Instruction::F64ConvertI32U);
            }
        }
    }
}
