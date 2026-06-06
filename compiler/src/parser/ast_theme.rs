use crate::lexer::token::Span;

#[derive(Debug, Clone)]
pub struct ThemeDef {
    pub sections: Vec<ThemeSection>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ThemeSection {
    pub name: String,
    pub entries: Vec<(String, ThemeTokenValue)>,
}

#[derive(Debug, Clone)]
pub enum ThemeTokenValue {
    Color(String), // hex string without #, e.g. "3B82F6"
    Number(f64),
}
