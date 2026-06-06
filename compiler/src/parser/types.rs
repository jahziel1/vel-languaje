use super::ast::*;
use super::{ParseError, ParseResult, Parser};
use crate::lexer::token::Token;

impl Parser {
    /// Parse a Vel type annotation.
    pub(super) fn parse_type(&mut self) -> ParseResult<Type> {
        let ty = match self.current().clone() {
            Token::Ident(name) => {
                self.advance();
                match name.as_str() {
                    "Text" => Type::Text,
                    "Number" => Type::Number,
                    "Bool" => Type::Bool,
                    "List" => {
                        self.expect(&Token::Lt)?;
                        let inner = self.parse_type()?;
                        self.expect(&Token::Gt)?;
                        Type::List(Box::new(inner))
                    }
                    "Map" => {
                        self.expect(&Token::Lt)?;
                        let k = self.parse_type()?;
                        self.expect(&Token::Comma)?;
                        let v = self.parse_type()?;
                        self.expect(&Token::Gt)?;
                        Type::Map(Box::new(k), Box::new(v))
                    }
                    "Result" => {
                        self.expect(&Token::Lt)?;
                        let inner = self.parse_type()?;
                        self.expect(&Token::Gt)?;
                        Type::Result(Box::new(inner))
                    }
                    _ => Type::Named(name),
                }
            }
            Token::None => {
                self.advance();
                Type::None
            }
            Token::LParen => {
                // function type: () -> ReturnType
                self.advance();
                let mut param_types = Vec::new();
                while !self.check(&Token::RParen) && !self.is_at_end() {
                    param_types.push(self.parse_type()?);
                    self.eat(&Token::Comma);
                }
                self.expect_rparen()?;
                self.expect(&Token::Arrow)?;
                let ret = self.parse_type()?;
                Type::Fn(param_types, Box::new(ret))
            }
            other => {
                return Err(ParseError::new(
                    format!("expected type, found {:?}", other),
                    self.current_span(),
                ));
            }
        };

        // Optional suffix: `?`
        if self.check(&Token::Question) {
            self.advance();
            Ok(Type::Optional(Box::new(ty)))
        } else {
            Ok(ty)
        }
    }

    /// Parse a raw string literal (just the content, no interpolation).
    pub(super) fn parse_string_literal(&mut self) -> ParseResult<String> {
        self.expect(&Token::StringStart)?;
        let content = match self.current().clone() {
            Token::StringLiteral(s) => {
                self.advance();
                s
            }
            _ => String::new(),
        };
        self.expect(&Token::StringEnd)?;
        Ok(content)
    }
}
