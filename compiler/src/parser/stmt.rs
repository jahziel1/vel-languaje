use super::ast::*;
use super::{ParseError, ParseResult, Parser};
use crate::lexer::token::Token;

fn breakpoint_from_str(s: &str) -> Option<Breakpoint> {
    match s {
        "phone" => Some(Breakpoint::Phone),
        "mobile" => Some(Breakpoint::Mobile),
        "tablet" => Some(Breakpoint::Tablet),
        "desktop" => Some(Breakpoint::Desktop),
        "wide" => Some(Breakpoint::Wide),
        _ => None,
    }
}

impl Parser {
    pub(super) fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        match self.current() {
            Token::State => self.parse_state_block().map(Stmt::State),
            Token::Derived => self.parse_derived_block().map(Stmt::Derived),
            Token::Guard => self.parse_guard().map(Stmt::Guard),
            Token::On => self.parse_on().map(Stmt::On),
            Token::Fn => self.parse_fn().map(Stmt::Fn),
            Token::If => self.parse_if().map(Stmt::If),
            Token::Match => self.parse_match().map(Stmt::Match),
            // assignment: `name = expr` — ident followed by `=`
            Token::Ident(_) if matches!(self.peek(), Token::Assign) => {
                self.parse_let().map(Stmt::Let)
            }
            // block-only element call: `column { ... }` (no parens)
            Token::Ident(_) if matches!(self.peek(), Token::LBrace) => {
                let ident = self.expect_ident()?;
                self.advance(); // consume '{'
                let (block_args, stmts) = self.parse_element_block_body()?;
                let block = if stmts.is_empty() { None } else { Some(stmts) };
                Ok(Stmt::Expr(Expr::Call(
                    Box::new(Expr::Ident(ident)),
                    block_args,
                    block,
                )))
            }
            _ => self.parse_expr(0).map(Stmt::Expr),
        }
    }

    // ── State block ───────────────────────────────────────────────────────────

    pub(super) fn parse_state_block(&mut self) -> ParseResult<StateBlock> {
        self.advance(); // consume 'state'
        self.expect_lbrace()?;
        let mut entries = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            entries.push(self.parse_state_entry()?);
        }
        self.expect_rbrace()?;
        Ok(StateBlock { entries })
    }

    fn parse_state_entry(&mut self) -> ParseResult<StateEntry> {
        let name = self.expect_ident()?;

        let ty = if self.check(&Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&Token::Assign)?;
        let value = self.parse_expr(0)?;

        // optional `, persist` flag
        let persist = if self.check(&Token::Comma) {
            let saved = self.pos;
            self.advance();
            if self.check(&Token::Persist) {
                self.advance();
                true
            } else {
                self.pos = saved; // backtrack
                false
            }
        } else {
            false
        };

        Ok(StateEntry {
            name,
            ty,
            value,
            persist,
        })
    }

    // ── Derived block ─────────────────────────────────────────────────────────

    pub(super) fn parse_derived_block(&mut self) -> ParseResult<DerivedBlock> {
        self.advance(); // consume 'derived'
        self.expect_lbrace()?;
        let mut entries = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let name = self.expect_ident()?;
            self.expect(&Token::Assign)?;
            let value = self.parse_expr(0)?;
            entries.push((name, value));
        }
        self.expect_rbrace()?;
        Ok(DerivedBlock { entries })
    }

    // ── Guard ─────────────────────────────────────────────────────────────────

    fn parse_guard(&mut self) -> ParseResult<GuardStmt> {
        self.advance(); // consume 'guard'
        let condition = self.parse_expr(0)?;
        self.expect(&Token::Arrow)?;
        let action = self.parse_expr(0)?;
        Ok(GuardStmt { condition, action })
    }

    // ── On statement ──────────────────────────────────────────────────────────

    fn parse_on(&mut self) -> ParseResult<OnStmt> {
        self.advance(); // consume 'on'
        let (event, param) = self.parse_on_event()?;
        self.expect_lbrace()?;

        // Check for lambda param: `{ msg -> stmts }`
        let (param, body) =
            if matches!(self.current(), Token::Ident(_)) && matches!(self.peek(), Token::Arrow) {
                let p = self.expect_ident()?;
                self.advance(); // consume '->'
                let body = self.parse_stmts_until_rbrace()?;
                (Some(p), body)
            } else {
                let body = self.parse_stmts_until_rbrace()?;
                (param, body)
            };

        Ok(OnStmt { event, param, body })
    }

    fn parse_on_event(&mut self) -> ParseResult<(OnEvent, Option<String>)> {
        match self.current().clone() {
            Token::Change => {
                self.advance();
                let var = self.expect_ident()?;
                Ok((OnEvent::Change(var), None))
            }
            Token::Mount => {
                self.advance();
                Ok((OnEvent::Mount, None))
            }
            Token::Unmount => {
                self.advance();
                Ok((OnEvent::Unmount, None))
            }
            Token::Key => {
                self.advance();
                self.expect_lparen()?;
                let key = self.parse_string_literal()?;
                self.expect_rparen()?;
                Ok((OnEvent::Key(key), None))
            }
            Token::Ident(name) if breakpoint_from_str(&name).is_some() => {
                let bp = breakpoint_from_str(&name).unwrap();
                self.advance();
                Ok((OnEvent::Responsive(bp), None))
            }
            other => Err(ParseError::new(
                format!("unknown 'on' event: {:?}", other),
                self.current_span(),
            )),
        }
    }

    // ── Function definition ───────────────────────────────────────────────────

    pub(super) fn parse_fn(&mut self) -> ParseResult<FnDef> {
        self.advance(); // consume 'fn'
        let name = self.expect_ident()?;
        let params = self.parse_params()?;
        let return_ty = if self.check(&Token::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect_lbrace()?;
        let body = self.parse_stmts_until_rbrace()?;
        Ok(FnDef {
            name,
            params,
            return_ty,
            body,
        })
    }

    // ── If / else ─────────────────────────────────────────────────────────────

    fn parse_if(&mut self) -> ParseResult<IfStmt> {
        self.advance(); // consume 'if'
        let condition = self.parse_expr(0)?;
        self.expect_lbrace()?;
        let then_body = self.parse_stmts_until_rbrace()?;
        let else_body = if self.check(&Token::Else) {
            self.advance();
            self.expect_lbrace()?;
            Some(self.parse_stmts_until_rbrace()?)
        } else {
            None
        };
        Ok(IfStmt {
            condition,
            then_body,
            else_body,
        })
    }

    // ── Match ─────────────────────────────────────────────────────────────────

    fn parse_match(&mut self) -> ParseResult<MatchStmt> {
        self.advance(); // consume 'match'
        let value = self.parse_expr(0)?;
        self.expect_lbrace()?;
        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            arms.push(self.parse_match_arm()?);
        }
        self.expect_rbrace()?;
        Ok(MatchStmt { value, arms })
    }

    fn parse_match_arm(&mut self) -> ParseResult<MatchArm> {
        let pattern = self.parse_pattern()?;
        self.expect(&Token::Arrow)?;
        let body = if self.check(&Token::LBrace) {
            self.advance();
            let stmts = self.parse_stmts_until_rbrace()?;
            MatchBody::Block(stmts)
        } else {
            MatchBody::Expr(self.parse_expr(0)?)
        };
        Ok(MatchArm { pattern, body })
    }

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        match self.current().clone() {
            Token::Ident(name) if name == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::True => {
                self.advance();
                Ok(Pattern::Ident("true".to_owned()))
            }
            Token::False => {
                self.advance();
                Ok(Pattern::Ident("false".to_owned()))
            }
            Token::Ident(name) => {
                self.advance();
                if self.check(&Token::LParen) {
                    self.advance();
                    let mut binds = Vec::new();
                    while !self.check(&Token::RParen) && !self.is_at_end() {
                        binds.push(self.expect_ident()?);
                        self.eat(&Token::Comma);
                    }
                    self.expect_rparen()?;
                    Ok(Pattern::Variant(name, binds))
                } else {
                    Ok(Pattern::Ident(name))
                }
            }
            other => Err(ParseError::new(
                format!("expected pattern, found {:?}", other),
                self.current_span(),
            )),
        }
    }

    // ── Let / assignment ──────────────────────────────────────────────────────

    fn parse_let(&mut self) -> ParseResult<LetStmt> {
        let name = self.expect_ident()?;
        self.expect(&Token::Assign)?;
        let value = self.parse_expr(0)?;
        Ok(LetStmt { name, value })
    }
}
