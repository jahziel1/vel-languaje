pub mod token;

use token::{Span, Token, TokenWithSpan};

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    // String interpolation state machine
    in_string: bool,
    interp_depth: usize, // nesting depth inside { } within a string
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            in_string: false,
            interp_depth: 0,
        }
    }

    pub fn tokenize(&mut self) -> Vec<TokenWithSpan> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.token == Token::EOF;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn current(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.source.get(self.pos + 2).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.source.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn span(&self) -> Span {
        Span {
            line: self.line,
            col: self.col,
        }
    }

    fn tok(&self, token: Token, span: Span) -> TokenWithSpan {
        TokenWithSpan::new(token, span)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current() {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => {
                    self.advance();
                }
                // line comment //
                Some('/') if self.peek() == Some('/') => {
                    while self.current().is_some_and(|c| c != '\n') {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    // ── Main dispatch ─────────────────────────────────────────────────────────

    fn next_token(&mut self) -> TokenWithSpan {
        // String content mode — lex inside a string literal
        if self.in_string {
            return self.lex_string_content();
        }

        self.skip_whitespace_and_comments();

        let span = self.span();
        let ch = match self.current() {
            None => return self.tok(Token::EOF, span),
            Some(c) => c,
        };

        // Color literal: #RRGGBB
        if ch == '#' {
            return self.lex_color(span);
        }

        // String start
        if ch == '"' {
            self.advance();
            self.in_string = true;
            return self.tok(Token::StringStart, span);
        }

        // Number
        if ch.is_ascii_digit() {
            return self.lex_number(span);
        }

        // Identifier or keyword
        if ch.is_alphabetic() || ch == '_' {
            return self.lex_ident_or_keyword(span);
        }

        // Multi-character operators (must check before single-char)
        match (ch, self.peek(), self.peek2()) {
            ('-', Some('>'), _) => {
                self.advance();
                self.advance();
                return self.tok(Token::Arrow, span);
            }
            ('.', Some('.'), Some('.')) => {
                self.advance();
                self.advance();
                self.advance();
                return self.tok(Token::Spread, span);
            }
            ('?', Some('?'), _) => {
                self.advance();
                self.advance();
                return self.tok(Token::NullCoal, span);
            }
            ('?', Some('.'), _) => {
                self.advance();
                self.advance();
                return self.tok(Token::OptChain, span);
            }
            ('=', Some('='), _) => {
                self.advance();
                self.advance();
                return self.tok(Token::Eq, span);
            }
            ('!', Some('='), _) => {
                self.advance();
                self.advance();
                return self.tok(Token::NotEq, span);
            }
            ('<', Some('='), _) => {
                self.advance();
                self.advance();
                return self.tok(Token::LtEq, span);
            }
            ('>', Some('='), _) => {
                self.advance();
                self.advance();
                return self.tok(Token::GtEq, span);
            }
            _ => {}
        }

        // Single-character tokens
        // '}' needs special handling when closing a string interpolation
        if ch == '}' && self.interp_depth > 0 {
            self.advance();
            self.interp_depth -= 1;
            if self.interp_depth == 0 {
                self.in_string = true;
                return self.tok(Token::InterpolationEnd, span);
            }
            return self.tok(Token::RBrace, span);
        }

        // '{' inside an interpolation increases nesting depth
        if ch == '{' && self.interp_depth > 0 {
            self.advance();
            self.interp_depth += 1;
            return self.tok(Token::LBrace, span);
        }

        let token = match ch {
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ',' => Token::Comma,
            ':' => Token::Colon,
            '.' => Token::Dot,
            '|' => Token::Pipe,
            '=' => Token::Assign,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '<' => Token::Lt,
            '>' => Token::Gt,
            '?' => Token::Question,
            _ => {
                // Unknown character — skip silently
                self.advance();
                return self.next_token();
            }
        };

        self.advance();
        self.tok(token, span)
    }

    // ── String content lexer ──────────────────────────────────────────────────

    fn lex_string_content(&mut self) -> TokenWithSpan {
        let span = self.span();

        match self.current() {
            // End of string
            Some('"') => {
                self.advance();
                self.in_string = false;
                self.tok(Token::StringEnd, span)
            }
            // Interpolation start: { expr }
            Some('{') => {
                self.advance();
                self.in_string = false;
                self.interp_depth = 1;
                self.tok(Token::InterpolationStart, span)
            }
            // Literal text content
            _ => {
                let mut content = String::new();
                loop {
                    match self.current() {
                        None | Some('"') | Some('{') => break,
                        Some('\\') => {
                            self.advance();
                            match self.advance() {
                                Some('n') => content.push('\n'),
                                Some('t') => content.push('\t'),
                                Some('"') => content.push('"'),
                                Some('{') => content.push('{'),
                                Some('\\') => content.push('\\'),
                                Some(c) => {
                                    content.push('\\');
                                    content.push(c);
                                }
                                None => break,
                            }
                        }
                        Some(c) => {
                            content.push(c);
                            self.advance();
                        }
                    }
                }
                self.tok(Token::StringLiteral(content), span)
            }
        }
    }

    // ── Sub-lexers ────────────────────────────────────────────────────────────

    fn lex_number(&mut self, span: Span) -> TokenWithSpan {
        let mut num = String::new();
        let mut has_dot = false;

        while let Some(c) = self.current() {
            if c.is_ascii_digit() {
                num.push(c);
                self.advance();
            } else if c == '.' && !has_dot && self.peek().is_some_and(|p| p.is_ascii_digit()) {
                has_dot = true;
                num.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let value: f64 = num.parse().unwrap_or(0.0);
        self.tok(Token::Number(value), span)
    }

    fn lex_color(&mut self, span: Span) -> TokenWithSpan {
        self.advance(); // skip #
        let mut hex = String::new();
        while let Some(c) = self.current() {
            if c.is_ascii_hexdigit() {
                hex.push(c);
                self.advance();
            } else {
                break;
            }
        }
        self.tok(Token::Color(hex), span)
    }

    fn lex_ident_or_keyword(&mut self, span: Span) -> TokenWithSpan {
        let mut ident = String::new();
        while let Some(c) = self.current() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let token = match ident.as_str() {
            "page"      => Token::Page,
            "component" => Token::Component,
            "layout"    => Token::Layout,
            "store"     => Token::Store,
            "export"    => Token::Export,
            "import"    => Token::Import,
            "from"      => Token::From,
            "type"      => Token::Type,
            "enum"      => Token::Enum,
            "state"     => Token::State,
            "derived"   => Token::Derived,
            "on"        => Token::On,
            "guard"     => Token::Guard,
            "fn"        => Token::Fn,
            "if"        => Token::If,
            "else"      => Token::Else,
            "match"     => Token::Match,
            "try"       => Token::Try,
            "persist"   => Token::Persist,
            "outlet"    => Token::Outlet,
            "change"    => Token::Change,
            "mount"     => Token::Mount,
            "unmount"   => Token::Unmount,
            "key"       => Token::Key,
            "true"      => Token::True,
            "false"     => Token::False,
            "none"      => Token::None,
            "and"       => Token::And,
            "or"        => Token::Or,
            "not"       => Token::Not,
            _           => Token::Ident(ident),
        };

        self.tok(token, span)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use token::Token;

    fn lex(src: &str) -> Vec<Token> {
        Lexer::new(src).tokenize().into_iter().map(|t| t.token).collect()
    }

    #[test]
    fn test_hello_world() {
        let tokens = lex(r#"page Home { text("Hello world") }"#);
        assert_eq!(tokens[0], Token::Page);
        assert_eq!(tokens[1], Token::Ident("Home".to_string()));
        assert_eq!(tokens[2], Token::LBrace);
        assert_eq!(tokens[3], Token::Ident("text".to_string()));
        assert_eq!(tokens[4], Token::LParen);
        assert_eq!(tokens[5], Token::StringStart);
        assert_eq!(tokens[6], Token::StringLiteral("Hello world".to_string()));
        assert_eq!(tokens[7], Token::StringEnd);
        assert_eq!(tokens[8], Token::RParen);
        assert_eq!(tokens[9], Token::RBrace);
    }

    #[test]
    fn test_keywords() {
        let tokens = lex("page component export import type enum state derived");
        assert_eq!(tokens[0], Token::Page);
        assert_eq!(tokens[1], Token::Component);
        assert_eq!(tokens[2], Token::Export);
        assert_eq!(tokens[3], Token::Import);
        assert_eq!(tokens[4], Token::Type);
        assert_eq!(tokens[5], Token::Enum);
        assert_eq!(tokens[6], Token::State);
        assert_eq!(tokens[7], Token::Derived);
    }

    #[test]
    fn test_operators() {
        let tokens = lex("-> == != <= >= ?? ?.");
        assert_eq!(tokens[0], Token::Arrow);
        assert_eq!(tokens[1], Token::Eq);
        assert_eq!(tokens[2], Token::NotEq);
        assert_eq!(tokens[3], Token::LtEq);
        assert_eq!(tokens[4], Token::GtEq);
        assert_eq!(tokens[5], Token::NullCoal);
        assert_eq!(tokens[6], Token::OptChain);
    }

    #[test]
    fn test_number() {
        let tokens = lex("42 3.14 0");
        assert_eq!(tokens[0], Token::Number(42.0));
        assert_eq!(tokens[1], Token::Number(3.14));
        assert_eq!(tokens[2], Token::Number(0.0));
    }

    #[test]
    fn test_color() {
        let tokens = lex("#3B82F6");
        assert_eq!(tokens[0], Token::Color("3B82F6".to_string()));
    }

    #[test]
    fn test_string_interpolation() {
        let tokens = lex(r#""Hello {name}!""#);
        assert_eq!(tokens[0], Token::StringStart);
        assert_eq!(tokens[1], Token::StringLiteral("Hello ".to_string()));
        assert_eq!(tokens[2], Token::InterpolationStart);
        assert_eq!(tokens[3], Token::Ident("name".to_string()));
        assert_eq!(tokens[4], Token::InterpolationEnd);
        assert_eq!(tokens[5], Token::StringLiteral("!".to_string()));
        assert_eq!(tokens[6], Token::StringEnd);
    }

    #[test]
    fn test_line_comment() {
        let tokens = lex("page // this is a comment\nHome");
        assert_eq!(tokens[0], Token::Page);
        assert_eq!(tokens[1], Token::Ident("Home".to_string()));
    }

    #[test]
    fn test_word_operators() {
        let tokens = lex("and or not");
        assert_eq!(tokens[0], Token::And);
        assert_eq!(tokens[1], Token::Or);
        assert_eq!(tokens[2], Token::Not);
    }

    #[test]
    fn test_spread() {
        let tokens = lex("...");
        assert_eq!(tokens[0], Token::Spread);
    }

    #[test]
    fn test_component_syntax() {
        let src = "export component Button(label: Text, onClick: () -> none)";
        let tokens = lex(src);
        assert_eq!(tokens[0], Token::Export);
        assert_eq!(tokens[1], Token::Component);
        assert_eq!(tokens[2], Token::Ident("Button".to_string()));
        assert_eq!(tokens[3], Token::LParen);
        assert_eq!(tokens[4], Token::Ident("label".to_string()));
        assert_eq!(tokens[5], Token::Colon);
        assert_eq!(tokens[6], Token::Ident("Text".to_string()));
    }

    #[test]
    fn test_span_tracking() {
        let tokens = Lexer::new("page\nHome").tokenize();
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.col, 1);
        assert_eq!(tokens[1].span.line, 2);
        assert_eq!(tokens[1].span.col, 1);
    }
}
