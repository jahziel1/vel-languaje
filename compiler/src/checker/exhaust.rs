use super::Checker;
use super::ty::Ty;
use crate::lexer::token::Span;
use crate::parser::ast::{MatchStmt, Pattern};

impl Checker {
    pub(super) fn check_assignable(&mut self, from: &Ty, to: &Ty, span: Span) {
        if matches!(from, Ty::None) && matches!(to, Ty::Optional(_)) {
            return;
        }
        if !Self::ty_compatible(from, to) {
            self.error(
                format!("type mismatch: expected `{}`, got `{}`", to, from),
                span,
            );
        }
    }

    /// Structural compatibility check — Unknown matches anything at any depth.
    pub(super) fn ty_compatible(from: &Ty, to: &Ty) -> bool {
        if matches!(from, Ty::Unknown) || matches!(to, Ty::Unknown) {
            return true;
        }
        match (from, to) {
            (Ty::Result(f), Ty::Result(t)) => Self::ty_compatible(f, t),
            (Ty::List(f), Ty::List(t)) => Self::ty_compatible(f, t),
            (Ty::Optional(f), Ty::Optional(t)) => Self::ty_compatible(f, t),
            _ => from == to,
        }
    }

    pub(super) fn check_match_exhaustive(
        &mut self,
        match_stmt: &MatchStmt,
        subject_ty: &Ty,
        span: Span,
    ) {
        let enum_name = match subject_ty {
            Ty::Named(name) => name.clone(),
            Ty::Result(_) => "Result".to_string(),
            _ => return,
        };

        let variants = match self.enum_defs.get(&enum_name) {
            Some(v) => v.clone(),
            None => return,
        };

        let has_wildcard = match_stmt
            .arms
            .iter()
            .any(|arm| matches!(arm.pattern, Pattern::Wildcard));

        if has_wildcard {
            return;
        }

        let covered: std::collections::HashSet<String> = match_stmt
            .arms
            .iter()
            .filter_map(|arm| match &arm.pattern {
                Pattern::Ident(name) | Pattern::Variant(name, _) => Some(name.clone()),
                Pattern::Wildcard => None,
            })
            .collect();

        for variant in &variants {
            if !covered.contains(&variant.name) {
                self.error(
                    format!(
                        "non-exhaustive match: missing case `{}.{}`",
                        enum_name, variant.name
                    ),
                    span,
                );
            }
        }
    }
}
