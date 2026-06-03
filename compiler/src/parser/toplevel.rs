use super::ast::*;
use super::{ParseResult, Parser};
use crate::lexer::token::Token;

impl Parser {
    pub(super) fn parse_item(&mut self) -> ParseResult<Item> {
        let exported = self.eat(&Token::Export);
        let item = match self.current() {
            Token::Import => self.parse_import(),
            Token::Page => self.parse_page().map(Item::Page),
            Token::Component => self.parse_component().map(Item::Component),
            Token::Layout => self.parse_layout().map(Item::Layout),
            Token::Store => self.parse_store().map(Item::Store),
            Token::Type => self.parse_type_def().map(Item::TypeDef),
            Token::Enum => self.parse_enum_def().map(Item::EnumDef),
            other => Err(super::ParseError::new(
                format!("unexpected token at top level: {:?}", other),
                self.current_span(),
            )),
        }?;
        // export wraps the item — for now we just return the inner item,
        // the type checker will handle export visibility
        let _ = exported;
        Ok(item)
    }

    // ── Import ────────────────────────────────────────────────────────────────

    fn parse_import(&mut self) -> ParseResult<Item> {
        let span = self.current_span();
        self.advance(); // consume 'import'

        let names = if self.check(&Token::LBrace) {
            self.advance(); // consume '{'
            let mut idents = Vec::new();
            while !self.check(&Token::RBrace) && !self.is_at_end() {
                idents.push(self.expect_ident()?);
                self.eat(&Token::Comma);
            }
            self.expect_rbrace()?;
            ImportNames::Named(idents)
        } else {
            ImportNames::Default(self.expect_ident()?)
        };

        self.expect(&Token::From)?;
        let path = self.parse_string_literal()?;
        Ok(Item::Import(Import { names, path, span }))
    }

    // ── Page ──────────────────────────────────────────────────────────────────

    pub(super) fn parse_page(&mut self) -> ParseResult<Page> {
        let span = self.current_span();
        self.advance(); // consume 'page'
        let name = self.expect_ident()?;
        let params = if self.check(&Token::LParen) {
            self.parse_params()?
        } else {
            Vec::new()
        };
        self.expect_lbrace()?;
        let body = self.parse_body()?;
        Ok(Page {
            name,
            params,
            body,
            span,
        })
    }

    // ── Component ─────────────────────────────────────────────────────────────

    pub(super) fn parse_component(&mut self) -> ParseResult<Component> {
        let span = self.current_span();
        self.advance(); // consume 'component'
        let name = self.expect_ident()?;
        let params = self.parse_params()?;
        self.expect_lbrace()?;
        let body = self.parse_body()?;
        Ok(Component {
            name,
            params,
            body,
            span,
        })
    }

    // ── Layout ────────────────────────────────────────────────────────────────

    fn parse_layout(&mut self) -> ParseResult<Layout> {
        let span = self.current_span();
        self.advance(); // consume 'layout'
        let name = self.expect_ident()?;
        self.expect_lbrace()?;
        let body = self.parse_body()?;
        Ok(Layout { name, body, span })
    }

    // ── Store ─────────────────────────────────────────────────────────────────

    fn parse_store(&mut self) -> ParseResult<Store> {
        let span = self.current_span();
        self.advance(); // consume 'store'
        let name = self.expect_ident()?;
        self.expect_lbrace()?;

        let mut state = None;
        let mut derived = None;
        let mut fns = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            match self.current() {
                Token::State => state = Some(self.parse_state_block()?),
                Token::Derived => derived = Some(self.parse_derived_block()?),
                Token::Fn => fns.push(self.parse_fn()?),
                _ => {
                    return Err(super::ParseError::new(
                        "expected 'state', 'derived', or 'fn' inside store",
                        self.current_span(),
                    ));
                }
            }
        }
        self.expect_rbrace()?;
        Ok(Store {
            name,
            state,
            derived,
            fns,
            span,
        })
    }

    // ── Type definition ───────────────────────────────────────────────────────

    fn parse_type_def(&mut self) -> ParseResult<TypeDef> {
        let span = self.current_span();
        self.advance(); // consume 'type'
        let name = self.expect_ident()?;
        self.expect_lbrace()?;
        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            fields.push(self.parse_field()?);
        }
        self.expect_rbrace()?;
        Ok(TypeDef { name, fields, span })
    }

    // ── Enum definition ───────────────────────────────────────────────────────

    fn parse_enum_def(&mut self) -> ParseResult<EnumDef> {
        let span = self.current_span();
        self.advance(); // consume 'enum'
        let name = self.expect_ident()?;
        self.expect_lbrace()?;
        let mut variants = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            variants.push(self.parse_variant()?);
            self.eat(&Token::Pipe);
        }
        self.expect_rbrace()?;
        Ok(EnumDef {
            name,
            variants,
            span,
        })
    }

    fn parse_variant(&mut self) -> ParseResult<Variant> {
        let name = self.expect_ident()?;
        let fields = if self.check(&Token::LParen) {
            self.advance();
            let mut names = Vec::new();
            while !self.check(&Token::RParen) && !self.is_at_end() {
                names.push(self.expect_ident()?);
                self.eat(&Token::Colon);
                // consume the type annotation (e.g. Text) — store as string for now
                if let Token::Ident(_) = self.current() {
                    self.advance();
                }
                self.eat(&Token::Comma);
            }
            self.expect_rparen()?;
            names
        } else {
            Vec::new()
        };
        Ok(Variant { name, fields })
    }

    // ── Shared helpers ────────────────────────────────────────────────────────

    /// Parse `(param: Type, ...)` parameter list.
    pub(super) fn parse_params(&mut self) -> ParseResult<Vec<Param>> {
        self.expect_lparen()?;
        self.parse_comma_list(&Token::RParen, |p| {
            let name = p.expect_ident()?;
            p.expect(&Token::Colon)?;
            let ty = p.parse_type()?;
            Ok(Param { name, ty })
        })
    }

    /// Parse a `field: Type` entry.
    pub(super) fn parse_field(&mut self) -> ParseResult<Field> {
        let name = self.expect_ident()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type()?;
        Ok(Field { name, ty })
    }

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
                return Err(super::ParseError::new(
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

    /// Parse the body of a page/component/layout/fn until `}`.
    pub(super) fn parse_body(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            stmts.push(self.parse_stmt()?);
        }
        self.expect_rbrace()?;
        Ok(stmts)
    }
}
