mod builtins;
mod check_items;
mod exhaust;
mod infer;
mod infer_field;
pub mod scope;
mod suggest;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_api_types;
#[cfg(test)]
mod tests_optional;
#[cfg(test)]
mod tests_types;
pub mod ty;

use std::collections::{HashMap, HashSet};

use crate::lexer::token::Span;
use crate::parser::ast::{self, FnDef, MatchBody, OnEvent, Pattern, Program, Stmt};
use scope::Scope;
pub use ty::Ty;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
    pub help: Option<String>,
}

impl TypeError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            help: None,
        }
    }
}

const NO_SPAN: Span = Span { line: 0, col: 0 };

// ── Checker ───────────────────────────────────────────────────────────────────

pub struct Checker {
    pub(super) type_defs: HashMap<String, Vec<ast::Field>>,
    pub(super) enum_defs: HashMap<String, Vec<ast::Variant>>,
    pub(super) store_names: HashSet<String>,
    /// section_name → token_name → is_color
    pub(super) theme_defs: HashMap<String, HashMap<String, bool>>,
    pub(super) has_theme: bool,
    pub errors: Vec<TypeError>,
}

impl Default for Checker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker {
    pub fn new() -> Self {
        Self {
            type_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            store_names: HashSet::new(),
            theme_defs: HashMap::new(),
            has_theme: false,
            errors: Vec::new(),
        }
    }

    pub(super) fn error(&mut self, msg: impl Into<String>, span: Span) {
        self.errors.push(TypeError::new(msg, span));
    }

    pub(super) fn error_help(
        &mut self,
        msg: impl Into<String>,
        span: Span,
        help: impl Into<String>,
    ) {
        let mut e = TypeError::new(msg, span);
        e.help = Some(help.into());
        self.errors.push(e);
    }

    // ── Public entry ──────────────────────────────────────────────────────────

    pub fn check(&mut self, program: &Program) -> &[TypeError] {
        self.collect_definitions(program);
        for item in &program.items {
            self.check_item(item);
        }
        &self.errors
    }

    // ── Statements ────────────────────────────────────────────────────────────

    pub(super) fn check_stmts(&mut self, stmts: &[Stmt], scope: &mut Scope) {
        for stmt in stmts {
            self.check_stmt(stmt, scope);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt, scope: &mut Scope) {
        match stmt {
            Stmt::State(state) => {
                for entry in &state.entries {
                    let inferred = self.infer_expr(&entry.value, scope);
                    let ty = if let Some(declared) = &entry.ty {
                        self.check_named_type_exists(declared, NO_SPAN);
                        let declared_ty = Ty::from_ast(declared);
                        self.check_assignable(&inferred, &declared_ty, NO_SPAN);
                        declared_ty
                    } else {
                        inferred
                    };
                    scope.define(entry.name.clone(), ty);
                }
            }
            Stmt::Derived(derived) => {
                for (name, expr) in &derived.entries {
                    let ty = self.infer_expr(expr, scope);
                    scope.define(name.clone(), ty);
                }
            }
            Stmt::Let(let_stmt) => {
                let ty = self.infer_expr(&let_stmt.value, scope);
                scope.define(let_stmt.name.clone(), ty);
            }
            Stmt::Fn(fn_def) => {
                let ret_ty = fn_def
                    .return_ty
                    .as_ref()
                    .map(Ty::from_ast)
                    .unwrap_or(Ty::Unknown);
                let param_tys: Vec<Ty> =
                    fn_def.params.iter().map(|p| Ty::from_ast(&p.ty)).collect();
                scope.define(fn_def.name.clone(), Ty::Fn(param_tys, Box::new(ret_ty)));
                self.check_fn(fn_def, scope);
            }
            Stmt::Guard(guard) => {
                self.infer_expr(&guard.condition, scope);
                self.infer_expr(&guard.action, scope);
            }
            Stmt::On(on_stmt) => {
                if let OnEvent::Change(var) = &on_stmt.event
                    && scope.lookup(var).is_none()
                {
                    self.error(format!("undefined variable `{}`", var), NO_SPAN);
                }
                let mut child = Scope::child(scope);
                if let Some(param) = &on_stmt.param {
                    child.define(param.clone(), Ty::Unknown);
                }
                self.check_stmts(&on_stmt.body, &mut child);
            }
            Stmt::If(if_stmt) => {
                self.infer_expr(&if_stmt.condition, scope);
                let mut then_scope = Scope::child(scope);
                self.check_stmts(&if_stmt.then_body, &mut then_scope);
                if let Some(else_body) = &if_stmt.else_body {
                    let mut else_scope = Scope::child(scope);
                    self.check_stmts(else_body, &mut else_scope);
                }
            }
            Stmt::Match(match_stmt) => {
                let subject_ty = self.infer_expr(&match_stmt.value, scope);
                self.check_match_exhaustive(match_stmt, &subject_ty, NO_SPAN);
                for arm in &match_stmt.arms {
                    let mut arm_scope = Scope::child(scope);
                    if let Pattern::Variant(variant_name, fields) = &arm.pattern {
                        for (i, field) in fields.iter().enumerate() {
                            let ty = self.variant_binding_ty(&subject_ty, variant_name, i);
                            arm_scope.define(field.clone(), ty);
                        }
                    }
                    match &arm.body {
                        MatchBody::Expr(expr) => {
                            self.infer_expr(expr, &arm_scope);
                        }
                        MatchBody::Block(stmts) => {
                            self.check_stmts(stmts, &mut arm_scope);
                        }
                    }
                }
            }
            Stmt::ForEach(param, body) => {
                let mut child = Scope::child(scope);
                child.define(param.clone(), Ty::Unknown);
                self.check_stmts(body, &mut child);
            }
            Stmt::Expr(expr) => {
                self.infer_expr(expr, scope);
            }
        }
    }

    pub(super) fn check_fn(&mut self, fn_def: &FnDef, parent_scope: &Scope) {
        let mut scope = Scope::child(parent_scope);
        for param in &fn_def.params {
            self.check_named_type_exists(&param.ty, NO_SPAN);
            scope.define(param.name.clone(), Ty::from_ast(&param.ty));
        }
        self.check_stmts(&fn_def.body, &mut scope);
    }
}
