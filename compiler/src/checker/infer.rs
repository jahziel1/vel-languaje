use super::scope::Scope;
use super::suggest::suggest_for_undefined;
use super::ty::Ty;
use super::{Checker, NO_SPAN};
use crate::parser::ast::{self, BinOp, Expr, Stmt, StringPart, UnOp};

impl Checker {
    pub(super) fn infer_expr(&mut self, expr: &Expr, scope: &Scope) -> Ty {
        match expr {
            Expr::Number(_) => Ty::Number,
            Expr::Bool(_) => Ty::Bool,
            Expr::None => Ty::None,
            Expr::Color(_) => Ty::Text,
            Expr::Str(parts) => {
                for part in parts {
                    if let StringPart::Interp(e) = part {
                        self.infer_expr(e, scope);
                    }
                }
                Ty::Text
            }
            Expr::Ident(name) => {
                if let Some(ty) = scope.lookup(name) {
                    ty.clone()
                } else {
                    match suggest_for_undefined(name) {
                        Some(hint) => {
                            self.error_help(format!("undefined variable `{}`", name), NO_SPAN, hint)
                        }
                        None => self.error(format!("undefined variable `{}`", name), NO_SPAN),
                    }
                    Ty::Unknown
                }
            }
            Expr::Field(obj, field) => {
                // theme.section.key — compile-time constants
                if let Expr::Field(inner, section) = obj.as_ref()
                    && let Expr::Ident(name) = inner.as_ref()
                    && name == "theme"
                {
                    return self.theme_token_ty(section, field);
                }
                let obj_ty = self.infer_expr(obj, scope);
                self.field_ty(&obj_ty, field)
            }
            Expr::OptField(obj, field) => {
                let obj_ty = self.infer_expr(obj, scope);
                // Unwrap Optional before field lookup so chaining works:
                // user?.address?.city — each `?.` peels one Optional layer.
                let base = match obj_ty {
                    Ty::Optional(t) => *t,
                    other => other,
                };
                let field_t = self.field_ty(&base, field);
                // Flatten Optional(Optional(T)) -> Optional(T) for fields typed as T?
                match field_t {
                    Ty::Optional(_) => field_t,
                    t => Ty::Optional(Box::new(t)),
                }
            }
            Expr::BinOp(lhs, op, rhs) => {
                let lt = self.infer_expr(lhs, scope);
                let rt = self.infer_expr(rhs, scope);
                self.infer_binop(op, &lt, &rt)
            }
            Expr::UnOp(op, operand) => {
                let ty = self.infer_expr(operand, scope);
                match op {
                    UnOp::Not => Ty::Bool,
                    UnOp::Neg => {
                        if !matches!(ty, Ty::Number | Ty::Unknown) {
                            self.error(format!("cannot negate `{}`", ty), NO_SPAN);
                        }
                        Ty::Number
                    }
                }
            }
            Expr::NullCoal(lhs, rhs) => {
                let lt = self.infer_expr(lhs, scope);
                let rt = self.infer_expr(rhs, scope);
                match lt {
                    Ty::Optional(inner) => {
                        // fallback must match the unwrapped type
                        self.check_assignable(&rt, &inner, NO_SPAN);
                        *inner
                    }
                    Ty::None => rt,
                    Ty::Unknown => Ty::Unknown,
                    other => other,
                }
            }
            Expr::Ternary(cond, then_e, else_e) => {
                self.infer_expr(cond, scope);
                let tt = self.infer_expr(then_e, scope);
                self.infer_expr(else_e, scope);
                tt
            }
            Expr::List(items) => {
                let elem_ty = items
                    .first()
                    .map(|e| self.infer_expr(e, scope))
                    .unwrap_or(Ty::Unknown);
                for item in items.iter().skip(1) {
                    self.infer_expr(item, scope);
                }
                Ty::List(Box::new(elem_ty))
            }
            Expr::Object(entries) => {
                for entry in entries {
                    match entry {
                        ast::ObjectEntry::Field(_, e) => {
                            self.infer_expr(e, scope);
                        }
                        ast::ObjectEntry::Spread(e) => {
                            self.infer_expr(e, scope);
                        }
                    }
                }
                Ty::Unknown
            }
            Expr::Call(callee, args, block) => {
                // Special case: list method calls — data.filter(lambda) or data.sum(lambda)
                if let Expr::Field(obj, method) = callee.as_ref() {
                    let obj_ty = self.infer_expr(obj, scope);
                    if let Ty::List(elem_ty) = obj_ty {
                        for a in args {
                            if let Expr::Lambda(params, body) = &a.value {
                                let mut lscope = Scope::child(scope);
                                for p in params {
                                    lscope.define(p.clone(), (*elem_ty).clone());
                                }
                                self.infer_expr(body, &lscope);
                            } else {
                                self.infer_expr(&a.value, scope);
                            }
                        }
                        if let Some(block_stmts) = block {
                            for s in block_stmts {
                                self.check_stmt(s, &mut Scope::child(scope));
                            }
                        }
                        return match method.as_str() {
                            "filter" => Ty::List(elem_ty),
                            "sum" => Ty::Number,
                            _ => Ty::Unknown,
                        };
                    }
                }

                self.infer_expr(callee, scope);
                // Infer first arg separately to propagate its type to list() ForEach
                let first_arg_ty = args.first().map(|a| self.infer_expr(&a.value, scope));
                for arg in args.iter().skip(1) {
                    self.infer_expr(&arg.value, scope);
                }
                if let Some(block_stmts) = block {
                    let mut block_scope = Scope::child(scope);
                    let list_elem_ty = first_arg_ty.and_then(|ty| match ty {
                        Ty::List(elem) => Some(*elem),
                        _ => None,
                    });
                    for stmt in block_stmts {
                        if let Stmt::ForEach(param, body) = stmt {
                            let item_ty = list_elem_ty.clone().unwrap_or(Ty::Unknown);
                            let mut child = Scope::child(&block_scope);
                            child.define(param.clone(), item_ty);
                            self.check_stmts(body, &mut child);
                        } else {
                            self.check_stmt(stmt, &mut block_scope);
                        }
                    }
                }
                Ty::Unknown
            }
            Expr::Lambda(params, body) => {
                let mut lambda_scope = Scope::child(scope);
                for p in params {
                    lambda_scope.define(p.clone(), Ty::Unknown);
                }
                let ret_ty = self.infer_expr(body, &lambda_scope);
                Ty::Fn(
                    params.iter().map(|_| Ty::Unknown).collect(),
                    Box::new(ret_ty),
                )
            }
            Expr::Try(inner) => {
                let inner_ty = self.infer_expr(inner, scope);
                match inner_ty {
                    Ty::Result(t) => Ty::Result(t),
                    Ty::Unknown => Ty::Result(Box::new(Ty::Unknown)),
                    t => Ty::Result(Box::new(t)),
                }
            }
        }
    }

    pub(super) fn infer_binop(&mut self, op: &BinOp, lt: &Ty, rt: &Ty) -> Ty {
        if matches!(lt, Ty::Unknown) || matches!(rt, Ty::Unknown) {
            return match op {
                BinOp::Eq
                | BinOp::NotEq
                | BinOp::Lt
                | BinOp::Gt
                | BinOp::LtEq
                | BinOp::GtEq
                | BinOp::And
                | BinOp::Or => Ty::Bool,
                _ => Ty::Unknown,
            };
        }
        match op {
            BinOp::Add => match (lt, rt) {
                (Ty::Number, Ty::Number) => Ty::Number,
                (Ty::Text, Ty::Text) => Ty::Text,
                _ => {
                    self.error(
                        format!(
                            "cannot add `{}` and `{}` — use string interpolation for mixed types",
                            lt, rt
                        ),
                        NO_SPAN,
                    );
                    Ty::Unknown
                }
            },
            BinOp::Sub | BinOp::Mul | BinOp::Div => {
                if !matches!((lt, rt), (Ty::Number, Ty::Number)) {
                    self.error(
                        format!("arithmetic requires Number, got `{}` and `{}`", lt, rt),
                        NO_SPAN,
                    );
                }
                Ty::Number
            }
            BinOp::Eq | BinOp::NotEq => Ty::Bool,
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                if !matches!((lt, rt), (Ty::Number, Ty::Number)) {
                    self.error(
                        format!("comparison requires Number, got `{}` and `{}`", lt, rt),
                        NO_SPAN,
                    );
                }
                Ty::Bool
            }
            BinOp::And | BinOp::Or => Ty::Bool,
        }
    }
}
