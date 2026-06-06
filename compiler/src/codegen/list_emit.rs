use wasm_encoder::{BlockType, Instruction};

use crate::parser::ast::{Arg, Expr, Stmt};

use super::emit::Ctx;
use super::{
    CodeGen, RT_LIST_COUNT, RT_LIST_ITEM_BOOL, RT_LIST_ITEM_NUM, RT_LIST_ITEM_STR, RT_LIST_SUM,
};

impl CodeGen {
    /// Emit `list(data) { item -> body }` or `list(data.filter(item -> pred)) { item -> body }`.
    /// `data` must be in `ctx.api_bindings` (inside a `Success(data)` arm).
    pub(super) fn emit_list_call(
        &mut self,
        args: &[Arg],
        block: &Option<Vec<Stmt>>,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let Some(first_arg) = args.first() else {
            out.push(Instruction::I32Const(0));
            return;
        };

        // Accept either `list(data)` or `list(data.filter(item -> pred))`
        let (var_name, filter_info): (String, Option<(String, &Expr)>) = match &first_arg.value {
            Expr::Ident(name) => (name.clone(), None),
            Expr::Call(callee, filter_args, None) => match (callee.as_ref(), filter_args.first()) {
                (
                    Expr::Field(obj, method),
                    Some(Arg {
                        value: Expr::Lambda(params, pred),
                        ..
                    }),
                ) if method == "filter" => match obj.as_ref() {
                    Expr::Ident(var) => {
                        let param = params.first().cloned().unwrap_or_default();
                        (var.clone(), Some((param, pred.as_ref())))
                    }
                    _ => {
                        out.push(Instruction::I32Const(0));
                        return;
                    }
                },
                _ => {
                    out.push(Instruction::I32Const(0));
                    return;
                }
            },
            _ => {
                out.push(Instruction::I32Const(0));
                return;
            }
        };

        let Some(&api_local) = ctx.api_bindings.get(var_name.as_str()) else {
            out.push(Instruction::I32Const(0));
            return;
        };

        // req_local (i32) = truncate api_local (f64 holding reqid)
        let req_local = ctx.alloc_i32_local();
        out.push(Instruction::LocalGet(api_local));
        out.push(Instruction::I32TruncF64S);
        out.push(Instruction::LocalSet(req_local));

        // count_local = list_count(req)
        let count_local = ctx.alloc_i32_local();
        out.push(Instruction::LocalGet(req_local));
        out.push(Instruction::Call(RT_LIST_COUNT));
        out.push(Instruction::LocalSet(count_local));

        // index_local = 0
        let index_local = ctx.alloc_i32_local();
        out.push(Instruction::I32Const(0));
        out.push(Instruction::LocalSet(index_local));

        out.push(Instruction::Block(BlockType::Empty));
        out.push(Instruction::Loop(BlockType::Empty));

        // if index >= count: break out of block
        out.push(Instruction::LocalGet(index_local));
        out.push(Instruction::LocalGet(count_local));
        out.push(Instruction::I32GeS);
        out.push(Instruction::BrIf(1));

        if let Some((filter_param, filter_pred)) = filter_info {
            // Wrap body in an inner block; skip if predicate is false.
            out.push(Instruction::Block(BlockType::Empty));
            ctx.list_bindings
                .insert(filter_param.clone(), (req_local, index_local));
            self.emit_expr_i32(filter_pred, ctx, out);
            ctx.list_bindings.remove(&filter_param);
            out.push(Instruction::I32Eqz);
            out.push(Instruction::BrIf(0)); // skip body
            if let Some(stmts) = block {
                for stmt in stmts {
                    if let Stmt::ForEach(param, body) = stmt {
                        ctx.list_bindings
                            .insert(param.clone(), (req_local, index_local));
                        for body_stmt in body {
                            self.emit_stmt(body_stmt, ctx, out);
                        }
                        ctx.list_bindings.remove(param.as_str());
                    } else {
                        self.emit_stmt(stmt, ctx, out);
                    }
                }
            }
            out.push(Instruction::End); // end inner block
        } else {
            // No filter: emit body directly.
            if let Some(stmts) = block {
                for stmt in stmts {
                    if let Stmt::ForEach(param, body) = stmt {
                        ctx.list_bindings
                            .insert(param.clone(), (req_local, index_local));
                        for body_stmt in body {
                            self.emit_stmt(body_stmt, ctx, out);
                        }
                        ctx.list_bindings.remove(param.as_str());
                    } else {
                        self.emit_stmt(stmt, ctx, out);
                    }
                }
            }
        }

        // index++
        out.push(Instruction::LocalGet(index_local));
        out.push(Instruction::I32Const(1));
        out.push(Instruction::I32Add);
        out.push(Instruction::LocalSet(index_local));

        out.push(Instruction::Br(0)); // continue loop
        out.push(Instruction::End); // end loop
        out.push(Instruction::End); // end block

        out.push(Instruction::I32Const(0));
    }

    /// Emit `data.sum(item -> item.field)` — calls host `list_sum(req, fptr, flen) -> f64`.
    /// Only supports simple field access in the lambda body.
    pub(super) fn emit_list_sum(
        &mut self,
        api_local: u32,
        args: &[Arg],
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let result = (|| {
            let first_arg = args.first()?;
            let Expr::Lambda(params, body_expr) = &first_arg.value else {
                return None;
            };
            let param_name = params.first()?;
            // Only handle `item -> item.field` (simple field access)
            if let Expr::Field(obj, field) | Expr::OptField(obj, field) = body_expr.as_ref()
                && let Expr::Ident(obj_name) = obj.as_ref()
                && obj_name == param_name
            {
                return Some(field.clone());
            }
            None
        })();

        if let Some(field) = result {
            let req_local = ctx.alloc_i32_local();
            out.push(Instruction::LocalGet(api_local));
            out.push(Instruction::I32TruncF64S);
            out.push(Instruction::LocalSet(req_local));
            let (fptr, flen) = self.intern(&field);
            out.push(Instruction::LocalGet(req_local));
            out.push(Instruction::I32Const(fptr as i32));
            out.push(Instruction::I32Const(flen as i32));
            out.push(Instruction::Call(RT_LIST_SUM));
        } else {
            out.push(Instruction::F64Const(0.0f64.into()));
        }
    }

    /// Emit a list item string field access inside the str_* builder protocol.
    /// Pushes `list_item_str(req, index, field_ptr, field_len)`.
    pub(super) fn emit_list_item_str(
        &mut self,
        req_local: u32,
        idx_local: u32,
        field: &str,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let (fptr, flen) = self.intern(field);
        out.push(Instruction::LocalGet(req_local));
        out.push(Instruction::LocalGet(idx_local));
        out.push(Instruction::I32Const(fptr as i32));
        out.push(Instruction::I32Const(flen as i32));
        out.push(Instruction::Call(RT_LIST_ITEM_STR));
    }

    /// Emit a list item numeric field access (returns f64 on stack).
    pub(super) fn emit_list_item_num(
        &mut self,
        req_local: u32,
        idx_local: u32,
        field: &str,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let (fptr, flen) = self.intern(field);
        out.push(Instruction::LocalGet(req_local));
        out.push(Instruction::LocalGet(idx_local));
        out.push(Instruction::I32Const(fptr as i32));
        out.push(Instruction::I32Const(flen as i32));
        out.push(Instruction::Call(RT_LIST_ITEM_NUM));
    }

    /// Emit a list item boolean field access (returns i32 on stack).
    pub(super) fn emit_list_item_bool(
        &mut self,
        req_local: u32,
        idx_local: u32,
        field: &str,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let (fptr, flen) = self.intern(field);
        out.push(Instruction::LocalGet(req_local));
        out.push(Instruction::LocalGet(idx_local));
        out.push(Instruction::I32Const(fptr as i32));
        out.push(Instruction::I32Const(flen as i32));
        out.push(Instruction::Call(RT_LIST_ITEM_BOOL));
    }
}
