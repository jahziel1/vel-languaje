use super::ast::*;
use super::{ParseResult, Parser};
use crate::lexer::token::Token;

impl Parser {
    /// Parse `(arg, name: arg, ...)` with optional trailing `{ block }`.
    pub(super) fn parse_call_args(&mut self) -> ParseResult<(Vec<Arg>, Option<Vec<Stmt>>)> {
        self.expect_lparen()?;
        let mut args = Vec::new();

        while !self.check(&Token::RParen) && !self.is_at_end() {
            let arg = self.parse_arg()?;
            args.push(arg);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect_rparen()?;

        // Optional UI block: `column(padding: 16) { ... }` — props or children
        let block = if self.check(&Token::LBrace) {
            self.advance();
            let (block_args, stmts) = self.parse_element_block_body()?;
            args.extend(block_args);
            if stmts.is_empty() { None } else { Some(stmts) }
        } else {
            None
        };

        Ok((args, block))
    }

    /// Parse `{ [ident: expr | stmt]* }` in an element context.
    /// Lines of the form `ident: expr` become extra Args (props added to the call).
    /// `ident -> stmts }` becomes Stmt::ForEach (iteration pattern for list()).
    /// All other lines become child Stmts.
    /// Assumes the opening `{` has already been consumed.
    pub(super) fn parse_element_block_body(&mut self) -> ParseResult<(Vec<Arg>, Vec<Stmt>)> {
        let mut extra_args = Vec::new();
        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if matches!(self.current(), Token::Ident(_)) && matches!(self.peek(), Token::Colon) {
                let name = self.expect_ident()?;
                self.advance(); // consume ':'
                let value = self.parse_expr(0)?;
                extra_args.push(Arg {
                    name: Some(name),
                    value,
                });
            } else if matches!(self.current(), Token::Ident(_))
                && matches!(self.peek(), Token::Arrow)
            {
                let param = self.expect_ident()?;
                self.advance(); // consume '->'
                let body = self.parse_stmts_until_rbrace()?; // consumes '}'
                stmts.push(Stmt::ForEach(param, body));
                return Ok((extra_args, stmts)); // '}' already consumed
            } else {
                stmts.push(self.parse_stmt()?);
            }
        }
        self.expect_rbrace()?;
        Ok((extra_args, stmts))
    }

    pub(super) fn parse_arg(&mut self) -> ParseResult<Arg> {
        // Named arg: `name: expr`
        if matches!(self.current(), Token::Ident(_)) && matches!(self.peek(), Token::Colon) {
            let name = self.expect_ident()?;
            self.advance(); // consume ':'
            let value = self.parse_expr(0)?;
            return Ok(Arg {
                name: Some(name),
                value,
            });
        }
        // Lambda arg: `param -> expr`  (used in filter/sum/etc.)
        if matches!(self.current(), Token::Ident(_)) && matches!(self.peek(), Token::Arrow) {
            let param = self.expect_ident()?;
            self.advance(); // consume '->'
            let body = self.parse_expr(0)?;
            return Ok(Arg {
                name: None,
                value: Expr::Lambda(vec![param], Box::new(body)),
            });
        }
        // Positional
        let value = self.parse_expr(0)?;
        Ok(Arg { name: None, value })
    }
}
