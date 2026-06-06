use super::ty::Ty;
use super::{Checker, NO_SPAN};

impl Checker {
    /// Resolve type for `theme.section.key` — color → Text, number → Number.
    pub(super) fn theme_token_ty(&mut self, section: &str, key: &str) -> Ty {
        match self.theme_defs.get(section).and_then(|m| m.get(key)) {
            Some(true) => Ty::Text,
            Some(false) => Ty::Number,
            None => {
                self.error(
                    format!("unknown theme token `{}.{}`", section, key),
                    NO_SPAN,
                );
                Ty::Unknown
            }
        }
    }

    /// Resolve the type of `obj.field` where `obj` has type `ty`.
    pub(super) fn field_ty(&mut self, ty: &Ty, field: &str) -> Ty {
        if matches!(ty, Ty::List(_)) {
            return match field {
                "length" => Ty::Number,
                "isEmpty" => Ty::Bool,
                _ => Ty::Unknown,
            };
        }
        let type_name = match ty {
            Ty::Named(n) => n.clone(),
            Ty::Unknown => return Ty::Unknown,
            other => {
                self.error(
                    format!(
                        "cannot access field `{}` on type `{}` — field access requires a named type",
                        field, other
                    ),
                    NO_SPAN,
                );
                return Ty::Unknown;
            }
        };
        let fields = match self.type_defs.get(&type_name).cloned() {
            Some(f) => f,
            None => return Ty::Unknown,
        };
        match fields.iter().find(|f| f.name == field) {
            Some(f) => Ty::from_ast(&f.ty),
            None => {
                self.error(
                    format!("type `{}` has no field `{}`", type_name, field),
                    NO_SPAN,
                );
                Ty::Unknown
            }
        }
    }
}
