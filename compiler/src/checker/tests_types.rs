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

#[test]
fn test_custom_type_field_text() {
    assert_no_errors(
        r#"
type User {
    name: Text
    age: Number
}
component UserCard(user: User) {
    text(user.name)
}
"#,
    );
}

#[test]
fn test_custom_type_field_number() {
    assert_no_errors(
        r#"
type Product {
    title: Text
    price: Number
    active: Bool
}
page Shop {
    state { p: Product? = none }
    text("{p?.price}")
}
"#,
    );
}

#[test]
fn test_custom_type_undefined_field_error() {
    let errors = check(
        r#"
type User {
    name: Text
}
component UserCard(user: User) {
    text(user.phone)
}
"#,
    );
    assert_error_contains(&errors, "has no field `phone`");
}

#[test]
fn test_undefined_type_in_state_error() {
    let errors = check(
        r#"
page Bad {
    state { user: Ghost = none }
}
"#,
    );
    assert_error_contains(&errors, "undefined type `Ghost`");
}

#[test]
fn test_undefined_type_in_param_error() {
    let errors = check(
        r#"
component Foo(x: Ghost) {
    text("hi")
}
"#,
    );
    assert_error_contains(&errors, "undefined type `Ghost`");
}

#[test]
fn test_custom_type_optional_field_access() {
    assert_no_errors(
        r#"
type User {
    name: Text
    age: Number
}
page Profile {
    state { user: User? = none }
    text(user?.name)
}
"#,
    );
}

#[test]
fn test_field_on_optional_without_chaining_is_error() {
    let errors = check(
        r#"
type User {
    name: Text
}
page Profile {
    state { user: User? = none }
    text(user.name)
}
"#,
    );
    assert_error_contains(&errors, "cannot access field `name` on type `User?`");
}

#[test]
fn test_field_on_number_is_error() {
    let errors = check(
        r#"
page Counter {
    state { count: Number = 0 }
    text(count.length)
}
"#,
    );
    assert_error_contains(&errors, "cannot access field `length` on type `Number`");
}

#[test]
fn test_field_on_text_is_error() {
    let errors = check(
        r#"
page Greeting {
    state { name: Text = "hi" }
    text(name.size)
}
"#,
    );
    assert_error_contains(&errors, "cannot access field `size` on type `Text`");
}
