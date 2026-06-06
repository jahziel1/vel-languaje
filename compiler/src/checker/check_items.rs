use crate::lexer::token::Span;
use crate::parser::ast::{self, Item, Program};

use super::Checker;
use super::ty::Ty;

impl Checker {
    pub(super) fn collect_definitions(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::TypeDef(td) => {
                    self.type_defs.insert(td.name.clone(), td.fields.clone());
                }
                Item::EnumDef(ed) => {
                    self.enum_defs.insert(ed.name.clone(), ed.variants.clone());
                }
                Item::Store(store) => {
                    self.store_names.insert(store.name.clone());
                }
                Item::ThemeDef(td) => {
                    self.has_theme = true;
                    for section in &td.sections {
                        let section_map = self.theme_defs.entry(section.name.clone()).or_default();
                        for (key, val) in &section.entries {
                            let is_color =
                                matches!(val, crate::parser::ast::ThemeTokenValue::Color(_));
                            section_map.insert(key.clone(), is_color);
                        }
                    }
                }
                _ => {}
            }
        }
        self.enum_defs
            .entry("Result".to_string())
            .or_insert_with(|| {
                vec![
                    ast::Variant {
                        name: "Loading".to_string(),
                        fields: vec![],
                    },
                    ast::Variant {
                        name: "Success".to_string(),
                        fields: vec!["value".to_string()],
                    },
                    ast::Variant {
                        name: "Error".to_string(),
                        fields: vec!["message".to_string()],
                    },
                ]
            });
    }

    pub(super) fn check_item(&mut self, item: &Item) {
        match item {
            Item::Page(page) => {
                let mut scope = self.builtin_scope();
                for param in &page.params {
                    self.check_named_type_exists(&param.ty, page.span);
                    scope.define(param.name.clone(), Ty::from_ast(&param.ty));
                }
                self.check_stmts(&page.body, &mut scope);
            }
            Item::Component(comp) => {
                let mut scope = self.builtin_scope();
                for param in &comp.params {
                    self.check_named_type_exists(&param.ty, comp.span);
                    scope.define(param.name.clone(), Ty::from_ast(&param.ty));
                }
                self.check_stmts(&comp.body, &mut scope);
            }
            Item::Layout(layout) => {
                let mut scope = self.builtin_scope();
                self.check_stmts(&layout.body, &mut scope);
            }
            Item::Store(store) => {
                let mut scope = self.builtin_scope();
                if let Some(state) = &store.state {
                    for entry in &state.entries {
                        let inferred = self.infer_expr(&entry.value, &scope);
                        let ty = entry.ty.as_ref().map(Ty::from_ast).unwrap_or(inferred);
                        scope.define(entry.name.clone(), ty);
                    }
                }
                if let Some(derived) = &store.derived {
                    for (name, expr) in &derived.entries {
                        let ty = self.infer_expr(expr, &scope);
                        scope.define(name.clone(), ty);
                    }
                }
                for fn_def in &store.fns {
                    self.check_fn(fn_def, &scope);
                }
            }
            Item::TypeDef(_) | Item::EnumDef(_) | Item::Import(_) | Item::ThemeDef(_) => {}
        }
    }

    pub(super) fn check_named_type_exists(&mut self, ty: &ast::Type, span: Span) {
        match ty {
            ast::Type::Named(name) => {
                if !self.type_defs.contains_key(name) && !self.enum_defs.contains_key(name) {
                    self.error(format!("undefined type `{}`", name), span);
                }
            }
            ast::Type::List(t) | ast::Type::Optional(t) | ast::Type::Result(t) => {
                self.check_named_type_exists(t, span);
            }
            ast::Type::Map(k, v) => {
                self.check_named_type_exists(k, span);
                self.check_named_type_exists(v, span);
            }
            ast::Type::Fn(params, ret) => {
                for p in params {
                    self.check_named_type_exists(p, span);
                }
                self.check_named_type_exists(ret, span);
            }
            _ => {}
        }
    }

    pub(super) fn variant_binding_ty(&self, subject_ty: &Ty, variant: &str, index: usize) -> Ty {
        match (subject_ty, variant, index) {
            (Ty::Result(inner), "Success", 0) => *inner.clone(),
            (Ty::Result(_), "Error", 0) => Ty::Text,
            _ => Ty::Unknown,
        }
    }
}
