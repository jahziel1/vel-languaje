mod keywords;
mod string;
#[cfg(test)]
mod tests;
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
            let is_eof = tok.token == Token::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    // ── Cursor helpers ────────────────────────────────────────────────────────

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

    // ── Whitespace and comments ───────────────────────────────────────────────

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current() {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => {
                    self.advance();
                }
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
        if self.in_string {
            return self.lex_string_content();
        }

        self.skip_whitespace_and_comments();

        let span = self.span();
        let ch = match self.current() {
            None => return self.tok(Token::Eof, span),
            Some(c) => c,
        };

        if ch == '#' {
            return self.lex_color(span);
        }
        if ch == '"' {
            self.advance();
            self.in_string = true;
            return self.tok(Token::StringStart, span);
        }
        if ch.is_ascii_digit() {
            return self.lex_number(span);
        }
        if ch.is_alphabetic() || ch == '_' {
            return self.lex_ident_or_keyword(span);
        }

        // Multi-character operators — must check before single-char
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

        // '}' closing a string interpolation
        if ch == '}' && self.interp_depth > 0 {
            self.advance();
            self.interp_depth -= 1;
            if self.interp_depth == 0 {
                self.in_string = true;
                return self.tok(Token::InterpolationEnd, span);
            }
            return self.tok(Token::RBrace, span);
        }

        // '{' deepens interpolation nesting
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
                self.advance();
                return self.next_token();
            }
        };

        self.advance();
        self.tok(token, span)
    }

    // ── Literal sub-lexers ────────────────────────────────────────────────────

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
        let token = keywords::keyword_or_ident(ident);
        self.tok(token, span)
    }
}
