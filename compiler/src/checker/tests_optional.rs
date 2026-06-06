use super::{Checker, TypeError};
use crate::lexer::Lexer;
use crate::parser::Parser;

fn check(source: &str) -> Vec<TypeError> {
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut checker = Checker::new();
    checker.check(&program);
    checker.errors.clone()
}

fn assert_no_errors(source: &str) {
    let errors = check(source);
    assert!(
        errors.is_empty(),
        "expected no type errors, got: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

fn assert_error_contains(errors: &[TypeError], substr: &str) {
    assert!(
        errors.iter().any(|e| e.message.contains(substr)),
        "expected an error containing {:?}, got: {:?}",
        substr,
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

// ── Optional chaining ─────────────────────────────────────────────────────────

#[test]
fn test_opt_chain_simple() {
    assert_no_errors(
        r#"
type User {
    name: Text
}
page Profile {
    state { user: User? = none }
    text(user?.name)
}
"#,
    );
}

#[test]
fn test_opt_chain_chained() {
    assert_no_errors(
        r#"
type Address {
    city: Text
}
type User {
    address: Address?
}
page Profile {
    state { user: User? = none }
    text(user?.address?.city)
}
"#,
    );
}

// ── Null coalescing ───────────────────────────────────────────────────────────

#[test]
fn test_null_coal_text_fallback() {
    assert_no_errors(
        r#"
type User {
    name: Text
}
page Profile {
    state { user: User? = none }
    text(user?.name ?? "Guest")
}
"#,
    );
}

#[test]
fn test_null_coal_number_fallback() {
    assert_no_errors(
        r#"
type Product {
    price: Number
}
page Shop {
    state { p: Product? = none }
    text("{p?.price ?? 0}")
}
"#,
    );
}

#[test]
fn test_null_coal_none_literal() {
    assert_no_errors(
        r#"
page X {
    val = none ?? "default"
    text(val)
}
"#,
    );
}

#[test]
fn test_null_coal_type_mismatch_error() {
    let errors = check(
        r#"
type User {
    age: Number
}
page Bad {
    state { user: User? = none }
    let x = user?.age ?? "not a number"
}
"#,
    );
    assert_error_contains(&errors, "type mismatch");
}
