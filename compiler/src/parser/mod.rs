pub mod ast;
mod expr;
mod stmt;
#[cfg(test)]
mod tests;
mod toplevel;

use crate::lexer::token::{Span, Token, TokenWithSpan};
use ast::Program;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

// ── Parser struct ─────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<TokenWithSpan>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<TokenWithSpan>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> ParseResult<Program> {
        let mut items = Vec::new();
        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    // ── Token stream helpers ──────────────────────────────────────────────────

    pub(super) fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    pub(super) fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span { line: 0, col: 0 })
    }

    pub(super) fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos + 1)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    pub(super) fn advance(&mut self) -> &Token {
        let tok = self
            .tokens
            .get(self.pos)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    pub(super) fn is_at_end(&self) -> bool {
        matches!(self.current(), Token::Eof)
    }

    /// Consume token if it matches, else return error.
    pub(super) fn expect(&mut self, expected: &Token) -> ParseResult<Span> {
        if std::mem::discriminant(self.current()) == std::mem::discriminant(expected) {
            let span = self.current_span();
            self.advance();
            Ok(span)
        } else {
            Err(ParseError::new(
                format!("expected {:?}, found {:?}", expected, self.current()),
                self.current_span(),
            ))
        }
    }

    /// Consume `{` — used everywhere a block starts.
    pub(super) fn expect_lbrace(&mut self) -> ParseResult<Span> {
        self.expect(&Token::LBrace)
    }

    /// Consume `}` — used everywhere a block ends.
    pub(super) fn expect_rbrace(&mut self) -> ParseResult<Span> {
        self.expect(&Token::RBrace)
    }

    /// Consume `(`.
    pub(super) fn expect_lparen(&mut self) -> ParseResult<Span> {
        self.expect(&Token::LParen)
    }

    /// Consume `)`.
    pub(super) fn expect_rparen(&mut self) -> ParseResult<Span> {
        self.expect(&Token::RParen)
    }

    /// Consume an identifier and return its name.
    pub(super) fn expect_ident(&mut self) -> ParseResult<String> {
        match self.current().clone() {
            Token::Ident(name) => {
                self.advance();
                Ok(name)
            }
            other => Err(ParseError::new(
                format!("expected identifier, found {:?}", other),
                self.current_span(),
            )),
        }
    }

    /// Check if current token matches (without consuming).
    pub(super) fn check(&self, tok: &Token) -> bool {
        std::mem::discriminant(self.current()) == std::mem::discriminant(tok)
    }

    /// Consume current token if it matches, return true if it did.
    pub(super) fn eat(&mut self, tok: &Token) -> bool {
        if self.check(tok) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Parse a comma-separated list until `end` token, consuming the `end`.
    pub(super) fn parse_comma_list<T>(
        &mut self,
        end: &Token,
        mut parse_one: impl FnMut(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<Vec<T>> {
        let mut items = Vec::new();
        while !self.check(end) && !self.is_at_end() {
            items.push(parse_one(self)?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect(end)?;
        Ok(items)
    }
}
