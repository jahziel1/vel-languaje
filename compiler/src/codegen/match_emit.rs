use wasm_encoder::{BlockType, Instruction, ValType};

use crate::parser::ast::{Expr, MatchArm, MatchBody, MatchStmt, Pattern};

use super::CodeGen;
use super::emit::Ctx;

// ── Known variant discriminants ───────────────────────────────────────────────

fn result_discriminant(name: &str) -> Option<i32> {
    match name {
        "Loading" => Some(0),
        "Success" => Some(1),
        "Error" => Some(2),
        _ => None,
    }
}

fn is_known_variant(name: &str) -> bool {
    matches!(name, "true" | "false" | "Loading" | "Success" | "Error")
}

/// Returns true for patterns that always match (wildcard or variable binding).
fn always_matches(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Wildcard => true,
        Pattern::Ident(n) => !is_known_variant(n),
        Pattern::Variant(_, _) => false,
    }
}

// ── impl CodeGen ──────────────────────────────────────────────────────────────

impl CodeGen {
    /// Emit a match statement as nested WASM if/else blocks.
    pub(super) fn emit_match_stmt(
        &mut self,
        m: &MatchStmt,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        if m.arms.is_empty() {
            return;
        }
        self.emit_arm_chain(&m.value, &m.arms, ctx, out);
    }

    fn emit_arm_chain(
        &mut self,
        value: &Expr,
        arms: &[MatchArm],
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        let Some(arm) = arms.first() else { return };

        if always_matches(&arm.pattern) {
            self.emit_arm_body(&arm.body, &arm.pattern, value, ctx, out);
            return;
        }

        self.emit_arm_cond(value, &arm.pattern, ctx, out);
        out.push(Instruction::If(BlockType::Empty));
        self.emit_arm_body(&arm.body, &arm.pattern, value, ctx, out);

        if arms.len() > 1 {
            out.push(Instruction::Else);
            self.emit_arm_chain(value, &arms[1..], ctx, out);
        }

        out.push(Instruction::End);
    }

    /// Push the condition for one arm onto the WASM stack (leaves an i32).
    fn emit_arm_cond(
        &mut self,
        value: &Expr,
        pattern: &Pattern,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        match pattern {
            Pattern::Wildcard => out.push(Instruction::I32Const(1)),

            Pattern::Ident(name) => match name.as_str() {
                "true" => self.emit_value_as_i32(value, ctx, out),
                "false" => {
                    self.emit_value_as_i32(value, ctx, out);
                    out.push(Instruction::I32Eqz);
                }
                known if result_discriminant(known).is_some() => {
                    let disc = result_discriminant(known).unwrap();
                    self.emit_value_as_i32(value, ctx, out);
                    out.push(Instruction::I32Const(disc));
                    out.push(Instruction::I32Eq);
                }
                _ => out.push(Instruction::I32Const(1)), // variable binding
            },

            Pattern::Variant(name, _) => {
                let disc = result_discriminant(name).unwrap_or(0);
                self.emit_value_as_i32(value, ctx, out);
                out.push(Instruction::I32Const(disc));
                out.push(Instruction::I32Eq);
            }
        }
    }

    /// Emit the match value as i32 (truncates f64 globals).
    fn emit_value_as_i32(
        &mut self,
        expr: &Expr,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        if let Expr::Ident(name) = expr {
            let gkey = format!("{}/{}", ctx.page, name);
            if let Some(&gidx) = self.global_map.get(&gkey) {
                let vtype = self.globals[gidx as usize].val_type;
                out.push(Instruction::GlobalGet(gidx));
                if vtype == ValType::F64 {
                    out.push(Instruction::I32TruncF64S);
                }
                return;
            }
        }
        self.emit_expr_i32(expr, ctx, out);
    }

    fn emit_arm_body(
        &mut self,
        body: &MatchBody,
        pattern: &Pattern,
        value: &Expr,
        ctx: &mut Ctx,
        out: &mut Vec<Instruction<'static>>,
    ) {
        // Set up local bindings for variant arms (Success(data) / Error(msg))
        if let Pattern::Variant(variant_name, bindings) = pattern
            && !bindings.is_empty()
            && let Expr::Ident(var_name) = value
        {
            let reqid_key = format!("{}/{}__reqid", ctx.page, var_name);
            if let Some(&reqid_gidx) = self.global_map.get(&reqid_key) {
                for binding in bindings {
                    if let Some(&local_idx) = ctx.locals.get(binding.as_str()) {
                        out.push(Instruction::GlobalGet(reqid_gidx));
                        out.push(Instruction::F64ConvertI32U);
                        out.push(Instruction::LocalSet(local_idx));
                        // Success bindings get api_field access; Error bindings are strings (future)
                        if variant_name == "Success" {
                            ctx.api_bindings.insert(binding.clone(), local_idx);
                        }
                    }
                }
            }
        }

        match body {
            MatchBody::Expr(e) => {
                self.emit_expr_push(e, ctx, out);
                out.push(Instruction::Drop);
            }
            MatchBody::Block(stmts) => {
                for s in stmts {
                    self.emit_stmt(s, ctx, out);
                }
            }
        }

        // Cleanup: remove Success bindings so they don't leak into subsequent arms
        if let Pattern::Variant(variant_name, bindings) = pattern
            && variant_name == "Success"
        {
            for binding in bindings {
                ctx.api_bindings.remove(binding.as_str());
            }
        }
    }
}
