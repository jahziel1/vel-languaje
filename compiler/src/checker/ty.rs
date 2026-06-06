use crate::parser::ast;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Text,
    Number,
    Bool,
    None,
    #[allow(dead_code)]
    Block,
    List(Box<Ty>),
    Map(Box<Ty>, Box<Ty>),
    Optional(Box<Ty>),
    Result(Box<Ty>),
    Fn(Vec<Ty>, Box<Ty>),
    Named(String),
    Unknown,
}

impl Ty {
    pub fn from_ast(ty: &ast::Type) -> Self {
        match ty {
            ast::Type::Text => Ty::Text,
            ast::Type::Number => Ty::Number,
            ast::Type::Bool => Ty::Bool,
            ast::Type::None => Ty::None,
            ast::Type::List(t) => Ty::List(Box::new(Ty::from_ast(t))),
            ast::Type::Map(k, v) => Ty::Map(Box::new(Ty::from_ast(k)), Box::new(Ty::from_ast(v))),
            ast::Type::Optional(t) => Ty::Optional(Box::new(Ty::from_ast(t))),
            ast::Type::Result(t) => Ty::Result(Box::new(Ty::from_ast(t))),
            ast::Type::Fn(params, ret) => Ty::Fn(
                params.iter().map(Ty::from_ast).collect(),
                Box::new(Ty::from_ast(ret)),
            ),
            ast::Type::Named(n) => Ty::Named(n.clone()),
        }
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Text => write!(f, "Text"),
            Ty::Number => write!(f, "Number"),
            Ty::Bool => write!(f, "Bool"),
            Ty::None => write!(f, "none"),
            Ty::Block => write!(f, "Block"),
            Ty::List(t) => write!(f, "List<{}>", t),
            Ty::Map(k, v) => write!(f, "Map<{},{}>", k, v),
            Ty::Optional(t) => write!(f, "{}?", t),
            Ty::Result(t) => write!(f, "Result<{}>", t),
            Ty::Fn(ps, r) => write!(
                f,
                "({}) -> {}",
                ps.iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                r
            ),
            Ty::Named(n) => write!(f, "{}", n),
            Ty::Unknown => write!(f, "unknown"),
        }
    }
}
