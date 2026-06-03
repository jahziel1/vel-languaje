#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // --- Keywords ---
    Page,
    Component,
    Layout,
    Store,
    Export,
    Import,
    From,
    Type,
    Enum,
    State,
    Derived,
    On,
    Guard,
    Fn,
    If,
    Else,
    Match,
    Try,
    Persist,
    Outlet,
    Change,  // "on change"
    Mount,   // "on mount"
    Unmount, // "on unmount"
    Key,     // "on key"

    // --- Value keywords ---
    True,
    False,
    None,

    // --- Word operators ---
    And,
    Or,
    Not,

    // --- Identifiers & Literals ---
    Ident(String),
    Number(f64),
    Color(String), // #RRGGBB hex

    // --- String tokens (supports interpolation) ---
    StringStart,           // opening "
    StringLiteral(String), // literal text segment inside string
    InterpolationStart,    // { inside a string
    InterpolationEnd,      // } closing an interpolation
    StringEnd,             // closing "

    // --- Delimiters ---
    LBrace,   // {
    RBrace,   // }
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]

    // --- Punctuation ---
    Comma, // ,
    Colon, // :
    Dot,   // .
    Pipe,  // |

    // --- Operators ---
    Arrow,  // ->
    Assign, // =
    Plus,   // +
    Minus,  // -
    Star,   // *
    Slash,  // /

    Question, // ?
    OptChain, // ?.
    NullCoal, // ??
    Spread,   // ...

    Eq,    // ==
    NotEq, // !=
    Lt,    // <
    Gt,    // >
    LtEq,  // <=
    GtEq,  // >=

    // --- Special ---
    Eof,
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct TokenWithSpan {
    pub token: Token,
    pub span: Span,
}

impl TokenWithSpan {
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}
