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

// ── Typed API responses ───────────────────────────────────────────────────────

#[test]
fn test_result_user_field_access() {
    assert_no_errors(
        r#"
type User {
    name: Text
    age: Number
}
page Profile {
    state { user: Result<User> = try api.get("/me") }
    match user {
        Loading       -> spinner()
        Success(data) -> text(data.name)
        Error(msg)    -> text(msg)
    }
}
"#,
    );
}

#[test]
fn test_result_list_foreach_field_access() {
    assert_no_errors(
        r#"
type User {
    name: Text
    age: Number
}
page Users {
    state { users: Result<List<User>> = try api.get("/users") }
    match users {
        Loading       -> spinner()
        Success(data) -> list(data) { item -> text(item.name) }
        Error(msg)    -> text(msg)
    }
}
"#,
    );
}

#[test]
fn test_error_binding_is_text() {
    assert_no_errors(
        r#"
type User {
    name: Text
}
page Profile {
    state { user: Result<User> = try api.get("/me") }
    match user {
        Loading       -> spinner()
        Success(data) -> text(data.name)
        Error(msg)    -> text(msg)
    }
}
"#,
    );
}

#[test]
fn test_result_user_wrong_field_error() {
    let errors = check(
        r#"
type User {
    name: Text
}
page Profile {
    state { user: Result<User> = try api.get("/me") }
    match user {
        Loading       -> spinner()
        Success(data) -> text(data.phone)
        Error(msg)    -> text(msg)
    }
}
"#,
    );
    assert_error_contains(&errors, "has no field `phone`");
}

#[test]
fn test_result_list_item_wrong_field_error() {
    let errors = check(
        r#"
type Product {
    title: Text
    price: Number
}
page Shop {
    state { products: Result<List<Product>> = try api.get("/products") }
    match products {
        Loading       -> spinner()
        Success(data) -> list(data) { item -> text(item.sku) }
        Error(msg)    -> text(msg)
    }
}
"#,
    );
    assert_error_contains(&errors, "has no field `sku`");
}
