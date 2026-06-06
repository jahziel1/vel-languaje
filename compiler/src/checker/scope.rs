use super::ty::Ty;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Scope {
    vars: HashMap<String, Ty>,
    parent: Option<Box<Scope>>,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Scope {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            parent: None,
        }
    }

    pub fn child(parent: &Scope) -> Self {
        Self {
            vars: HashMap::new(),
            parent: Some(Box::new(parent.clone())),
        }
    }

    pub fn define(&mut self, name: impl Into<String>, ty: Ty) {
        self.vars.insert(name.into(), ty);
    }

    pub fn lookup(&self, name: &str) -> Option<&Ty> {
        self.vars
            .get(name)
            .or_else(|| self.parent.as_ref()?.lookup(name))
    }
}
