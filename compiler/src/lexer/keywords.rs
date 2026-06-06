use super::token::Token;

/// Maps an identifier string to its keyword token, or returns `Token::Ident`.
pub fn keyword_or_ident(ident: String) -> Token {
    match ident.as_str() {
        // Structure
        "page" => Token::Page,
        "component" => Token::Component,
        "layout" => Token::Layout,
        "store" => Token::Store,
        "export" => Token::Export,
        "import" => Token::Import,
        "from" => Token::From,
        "type" => Token::Type,
        "enum" => Token::Enum,
        "theme" => Token::Theme,
        // State
        "state" => Token::State,
        "derived" => Token::Derived,
        "persist" => Token::Persist,
        "outlet" => Token::Outlet,
        // Events
        "on" => Token::On,
        "change" => Token::Change,
        "mount" => Token::Mount,
        "unmount" => Token::Unmount,
        "key" => Token::Key,
        // Control flow
        "guard" => Token::Guard,
        "fn" => Token::Fn,
        "if" => Token::If,
        "else" => Token::Else,
        "match" => Token::Match,
        "try" => Token::Try,
        // Values
        "true" => Token::True,
        "false" => Token::False,
        "none" => Token::None,
        // Word operators
        "and" => Token::And,
        "or" => Token::Or,
        "not" => Token::Not,
        // Identifier
        _ => Token::Ident(ident),
    }
}
