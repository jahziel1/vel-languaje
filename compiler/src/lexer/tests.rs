use super::Lexer;
use super::token::Token;

fn lex(src: &str) -> Vec<Token> {
    Lexer::new(src)
        .tokenize()
        .into_iter()
        .map(|t| t.token)
        .collect()
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
