use super::ast::*;
use super::{ParseError, ParseResult, Parser};
use crate::lexer::token::Token;

// Pratt parser binding powers.
// Each infix operator returns (left_bp, right_bp).
// Higher = binds tighter. right_bp > left_bp = right-associative.
fn infix_bp(tok: &Token) -> Option<(u8, u8)> {
    match tok {
        Token::Or => Some((2, 3)),
        Token::And => Some((4, 5)),
        Token::Eq | Token::NotEq => Some((6, 7)),
        Token::Lt | Token::Gt | Token::LtEq | Token::GtEq => Some((8, 9)),
        Token::Plus | Token::Minus => Some((10, 11)),
        Token::Star | Token::Slash => Some((12, 13)),
        Token::NullCoal => Some((14, 15)),
        Token::Dot | Token::OptChain => Some((20, 21)),
        _ => None,
    }
}

impl Parser {
    /// Entry point for expression parsing.
    /// `min_bp` is the minimum binding power for infix operators (0 = any).
    pub(super) fn parse_expr(&mut self, min_bp: u8) -> ParseResult<Expr> {
        let mut lhs = self.parse_prefix()?;

        loop {
            // Postfix: function call `expr(args)` with optional block
            if self.check(&Token::LParen) {
                let (args, block) = self.parse_call_args()?;
                lhs = Expr::Call(Box::new(lhs), args, block);
                continue;
            }

            // Ternary: `cond ? a : b`
            if self.check(&Token::Question) {
                if min_bp > 1 {
                    break;
                }
                self.advance();
                let then = self.parse_expr(0)?;
                self.expect(&Token::Colon)?;
                let else_ = self.parse_expr(0)?;
                lhs = Expr::Ternary(Box::new(lhs), Box::new(then), Box::new(else_));
                continue;
            }

            // Infix operators
            let Some((l_bp, r_bp)) = infix_bp(self.current()) else {
                break;
            };
            if l_bp < min_bp {
                break;
            }

            let op_tok = self.current().clone();
            self.advance();

            lhs = match op_tok {
                Token::Dot => {
                    let field = self.expect_ident()?;
                    Expr::Field(Box::new(lhs), field)
                }
                Token::OptChain => {
                    let field = self.expect_ident()?;
                    Expr::OptField(Box::new(lhs), field)
                }
                Token::NullCoal => {
                    let rhs = self.parse_expr(r_bp)?;
                    Expr::NullCoal(Box::new(lhs), Box::new(rhs))
                }
                _ => {
                    let rhs = self.parse_expr(r_bp)?;
                    Expr::BinOp(Box::new(lhs), tok_to_binop(&op_tok), Box::new(rhs))
                }
            };
        }

        Ok(lhs)
    }

    // ── Prefix expressions ────────────────────────────────────────────────────

    fn parse_prefix(&mut self) -> ParseResult<Expr> {
        match self.current().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Token::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            Token::None => {
                self.advance();
                Ok(Expr::None)
            }
            Token::Color(hex) => {
                self.advance();
                Ok(Expr::Color(hex))
            }
            Token::StringStart => self.parse_interpolated_string(),
            Token::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name))
            }
            // `theme` keyword used as an expression identifier (e.g. theme.colors.X)
            Token::Theme => {
                self.advance();
                Ok(Expr::Ident("theme".to_owned()))
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_expr(18)?;
                Ok(Expr::UnOp(UnOp::Not, Box::new(operand)))
            }
            Token::Minus => {
                self.advance();
                let operand = self.parse_expr(18)?;
                Ok(Expr::UnOp(UnOp::Neg, Box::new(operand)))
            }
            Token::Try => {
                self.advance();
                let operand = self.parse_expr(0)?;
                Ok(Expr::Try(Box::new(operand)))
            }
            Token::LParen => {
                self.advance();
                let inner = self.parse_expr(0)?;
                self.expect_rparen()?;
                Ok(inner)
            }
            Token::LBracket => self.parse_list(),
            Token::LBrace => self.parse_object(),
            other => Err(ParseError::new(
                format!("unexpected token in expression: {:?}", other),
                self.current_span(),
            )),
        }
    }

    // ── String interpolation ──────────────────────────────────────────────────

    fn parse_interpolated_string(&mut self) -> ParseResult<Expr> {
        self.advance(); // consume StringStart
        let mut parts = Vec::new();

        loop {
            match self.current().clone() {
                Token::StringEnd => {
                    self.advance();
                    break;
                }
                Token::StringLiteral(s) => {
                    self.advance();
                    parts.push(StringPart::Lit(s));
                }
                Token::InterpolationStart => {
                    self.advance();
                    let expr = self.parse_expr(0)?;
                    self.expect(&Token::InterpolationEnd)?;
                    parts.push(StringPart::Interp(Box::new(expr)));
                }
                Token::Eof => {
                    return Err(ParseError::new(
                        "unterminated string literal",
                        self.current_span(),
                    ));
                }
                other => {
                    return Err(ParseError::new(
                        format!("unexpected token in string: {:?}", other),
                        self.current_span(),
                    ));
                }
            }
        }

        Ok(Expr::Str(parts))
    }

    // ── Call arguments ────────────────────────────────────────────────────────

    // ── List literal ──────────────────────────────────────────────────────────

    fn parse_list(&mut self) -> ParseResult<Expr> {
        self.advance(); // consume '['
        let items = self.parse_comma_list(&Token::RBracket, |p| p.parse_expr(0))?;
        Ok(Expr::List(items))
    }

    // ── Object literal ────────────────────────────────────────────────────────

    fn parse_object(&mut self) -> ParseResult<Expr> {
        self.advance(); // consume '{'
        let mut entries = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if self.check(&Token::Spread) {
                self.advance();
                let expr = self.parse_expr(0)?;
                entries.push(ObjectEntry::Spread(expr));
            } else {
                let key = self.expect_ident()?;
                self.expect(&Token::Colon)?;
                let val = self.parse_expr(0)?;
                entries.push(ObjectEntry::Field(key, val));
            }
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect_rbrace()?;
        Ok(Expr::Object(entries))
    }
}

fn tok_to_binop(tok: &Token) -> BinOp {
    match tok {
        Token::Plus => BinOp::Add,
        Token::Minus => BinOp::Sub,
        Token::Star => BinOp::Mul,
        Token::Slash => BinOp::Div,
        Token::Eq => BinOp::Eq,
        Token::NotEq => BinOp::NotEq,
        Token::Lt => BinOp::Lt,
        Token::Gt => BinOp::Gt,
        Token::LtEq => BinOp::LtEq,
        Token::GtEq => BinOp::GtEq,
        Token::And => BinOp::And,
        Token::Or => BinOp::Or,
        _ => unreachable!("not a binary operator: {:?}", tok),
    }
}
