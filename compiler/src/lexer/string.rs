use super::Lexer;
use super::token::{Token, TokenWithSpan};

impl Lexer {
    /// Lexes content inside a string literal, handling escape sequences
    /// and `{expr}` interpolation markers.
    pub(super) fn lex_string_content(&mut self) -> TokenWithSpan {
        let span = self.span();
        match self.current() {
            Some('"') => {
                self.advance();
                self.in_string = false;
                self.tok(Token::StringEnd, span)
            }
            Some('{') => {
                self.advance();
                self.in_string = false;
                self.interp_depth = 1;
                self.tok(Token::InterpolationStart, span)
            }
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
}
