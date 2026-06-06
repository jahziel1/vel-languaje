use wasm_encoder::{BlockType, Instruction, ValType};

use crate::parser::ast::{Expr, UnOp};

use super::emit::Ctx;
use super::{CodeGen, RT_API_FIELD_NUM, RT_LIST_COUNT};

impl CodeGen {
    /// Emit an expression — always pushes exactly one value (f64 or i32).
    pub(super) fn emit_expr_push(
        &mut self,
        expr: &Expr,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        match expr {
            Expr::Number(n) => out.push(Instruction::F64Const((*n).into())),
            Expr::Bool(b) => out.push(Instruction::I32Const(if *b { 1 } else { 0 })),
            Expr::None => out.push(Instruction::I32Const(0)),
            Expr::Color(_) | Expr::Str(_) => out.push(Instruction::I32Const(0)),

            Expr::Ident(name) => {
                let gkey = format!("{}/{}", ctx.page, name);
                if let Some(&gidx) = self.global_map.get(&gkey) {
                    out.push(Instruction::GlobalGet(gidx));
                } else if let Some(&pidx) = ctx.params.get(name.as_str()) {
                    out.push(Instruction::LocalGet(pidx));
                } else if let Some(&lidx) = ctx.locals.get(name.as_str()) {
                    out.push(Instruction::LocalGet(lidx));
                } else {
                    out.push(Instruction::I32Const(0));
                }
            }

            Expr::Field(obj, field) | Expr::OptField(obj, field) => {
                if let Expr::Ident(var_name) = obj.as_ref() {
                    // data.length → f64(list_count(req))
                    if field == "length"
                        && let Some(&api_local) = ctx.api_bindings.get(var_name.as_str())
                    {
                        let req_local = ctx.alloc_i32_local();
                        out.push(Instruction::LocalGet(api_local));
                        out.push(Instruction::I32TruncF64S);
                        out.push(Instruction::LocalSet(req_local));
                        out.push(Instruction::LocalGet(req_local));
                        out.push(Instruction::Call(RT_LIST_COUNT));
                        out.push(Instruction::F64ConvertI32S);
                        return;
                    }
                    if let Some(&(req_l, idx_l)) = ctx.list_bindings.get(var_name.as_str()) {
                        self.emit_list_item_num(req_l, idx_l, field, out);
                        return;
                    }
                    if let Some(&local_idx) = ctx.api_bindings.get(var_name.as_str()) {
                        let (fptr, flen) = self.intern(field);
                        out.push(Instruction::LocalGet(local_idx));
                        out.push(Instruction::I32TruncF64S);
                        out.push(Instruction::I32Const(fptr as i32));
                        out.push(Instruction::I32Const(flen as i32));
                        out.push(Instruction::Call(RT_API_FIELD_NUM));
                        return;
                    }
                    // Store field access: counter.count → GlobalGet("counter/count")
                    // or store derived: counter.doubled → inline expr with ctx.page=store
                    if self.store_names.contains(var_name.as_str()) {
                        let store_key = format!("{}/{}", var_name, field);
                        if let Some(&gidx) = self.global_map.get(&store_key) {
                            out.push(Instruction::GlobalGet(gidx));
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
                            self.emit_expr_push(&expr, ctx, out);
                            ctx.page = saved_page;
                            return;
                        }
                        out.push(Instruction::F64Const(0.0f64.into()));
                        return;
                    }
                }
                self.emit_expr_push(obj, ctx, out);
                out.push(Instruction::Drop);
                out.push(Instruction::F64Const(0.0f64.into()));
            }

            Expr::BinOp(lhs, op, rhs) => {
                self.emit_binop_expr(lhs, op, rhs, ctx, out);
            }

            Expr::UnOp(op, operand) => match op {
                UnOp::Neg => {
                    self.emit_expr_f64(operand, ctx, out);
                    out.push(Instruction::F64Neg);
                }
                UnOp::Not => {
                    self.emit_expr_i32(operand, ctx, out);
                    out.push(Instruction::I32Eqz);
                }
            },

            Expr::NullCoal(lhs, _rhs) => {
                self.emit_expr_push(lhs, ctx, out);
            }

            Expr::Ternary(cond, then_e, else_e) => {
                self.emit_expr_i32(cond, ctx, out);
                out.push(Instruction::If(BlockType::Result(ValType::F64)));
                self.emit_expr_f64(then_e, ctx, out);
                out.push(Instruction::Else);
                self.emit_expr_f64(else_e, ctx, out);
                out.push(Instruction::End);
            }

            Expr::List(_) | Expr::Object(_) => out.push(Instruction::I32Const(0)),

            Expr::Lambda(params, body) => {
                let mut lambda_ctx = ctx.clone();
                let base = lambda_ctx.next_local;
                for (i, p) in params.iter().enumerate() {
                    lambda_ctx.locals.insert(p.clone(), base + i as u32);
                    lambda_ctx.next_local += 1;
                }
                self.emit_expr_push(body, &mut lambda_ctx, out);
            }

            Expr::Try(inner) => self.emit_expr_push(inner, ctx, out),

            Expr::Call(callee, args, block) => {
                self.emit_call_expr(callee, args, block, ctx, out);
            }
        }
    }

    pub(super) fn emit_expr_f64(
        &mut self,
        expr: &Expr,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        // theme.text.X or theme.radius.X → compile-time f64 constant
        if let Expr::Field(obj, key) = expr
            && let Expr::Field(inner, section) = obj.as_ref()
            && let Expr::Ident(name) = inner.as_ref()
            && name == "theme"
        {
            let map_key = format!("{}/{}", section, key);
            if let Some(&n) = self.theme_numbers.get(&map_key) {
                out.push(Instruction::F64Const(n.into()));
                return;
            }
        }
        match expr {
            Expr::Number(n) => {
                out.push(Instruction::F64Const((*n).into()));
            }
            Expr::Bool(b) => {
                out.push(Instruction::F64Const(
                    (if *b { 1.0_f64 } else { 0.0_f64 }).into(),
                ));
            }
            // Guarantee f64: globals may be i32 (Bool state), unknown idents fall back to 0.0.
            Expr::Ident(name) => {
                let gkey = format!("{}/{}", ctx.page, name);
                if let Some(&gidx) = self.global_map.get(&gkey) {
                    out.push(Instruction::GlobalGet(gidx));
                    if self.globals[gidx as usize].val_type == ValType::I32 {
                        out.push(Instruction::F64ConvertI32S);
                    }
                } else if let Some(&pidx) = ctx.params.get(name.as_str()) {
                    out.push(Instruction::LocalGet(pidx));
                } else if let Some(&lidx) = ctx.locals.get(name.as_str()) {
                    out.push(Instruction::LocalGet(lidx));
                } else {
                    out.push(Instruction::F64Const(0.0f64.into()));
                }
            }
            _ => {
                self.emit_expr_push(expr, ctx, out);
            }
        }
    }
}
