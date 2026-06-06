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

// ── Valid programs ────────────────────────────────────────────────────────────

#[test]
fn test_valid_page() {
    assert_no_errors(
        r#"
page Home {
    state {
        count = 0
        name = "Alice"
    }
    text("Hello")
    column(padding: 16) {
        text("World")
    }
}
"#,
    );
}

#[test]
fn test_valid_component() {
    assert_no_errors(
        r#"
component Greeting(name: Text) {
    text("Hello {name}")
}
"#,
    );
}

#[test]
fn test_valid_fn() {
    assert_no_errors(
        r#"
page Checkout {
    state { total = 0.0 }
    fn calculate(): Number {
        total
    }
    text("{calculate()}")
}
"#,
    );
}

#[test]
fn test_match_all_variants_covered() {
    assert_no_errors(
        r#"
enum Status {
    Loading
    Success
    Error
}
page Good {
    state { status: Status = Loading }
    match status {
        Loading -> spinner()
        Success -> text("ok")
        Error   -> text("fail")
    }
}
"#,
    );
}

#[test]
fn test_match_wildcard_covers_rest() {
    assert_no_errors(
        r#"
enum Direction {
    Left
    Right
    Up
    Down
}
page Good {
    state { dir: Direction = Left }
    match dir {
        Left -> text("left")
        _    -> text("other")
    }
}
"#,
    );
}

#[test]
fn test_result_match_exhaustive() {
    assert_no_errors(
        r#"
page Profile {
    state { user = try api.get("/user") }
    match user {
        Loading      -> spinner()
        Success(data) -> text("ok")
        Error(msg)   -> text(msg)
    }
}
"#,
    );
}

#[test]
fn test_let_binding() {
    assert_no_errors(
        r#"
page Calc {
    x = 10
    y = 20
    text("{x + y}")
}
"#,
    );
}

// ── Type errors ───────────────────────────────────────────────────────────────

#[test]
fn test_no_implicit_coercion_add() {
    let errors = check(
        r#"
page Bad {
    state { age = 25 }
    text("hello" + age)
}
"#,
    );
    assert_error_contains(&errors, "cannot add");
}

#[test]
fn test_arithmetic_on_text() {
    let errors = check(
        r#"
page Bad {
    state { name = "alice" }
    let result = name - 5
}
"#,
    );
    assert_error_contains(&errors, "arithmetic requires Number");
}

#[test]
fn test_undefined_variable() {
    let errors = check(
        r#"
page Bad {
    text(notDefined)
}
"#,
    );
    assert_error_contains(&errors, "undefined variable `notDefined`");
}

#[test]
fn test_non_exhaustive_match() {
    let errors = check(
        r#"
enum Status {
    Loading
    Success
    Error
}
page Bad {
    state { status: Status = Loading }
    match status {
        Loading -> spinner()
        Success -> text("ok")
    }
}
"#,
    );
    assert_error_contains(&errors, "non-exhaustive match");
    assert_error_contains(&errors, "Error");
}

#[test]
fn test_negate_text_error() {
    let errors = check(
        r#"
page Bad {
    state { name = "alice" }
    let x = -name
}
"#,
    );
    assert_error_contains(&errors, "cannot negate");
}

#[test]
fn test_comparison_on_text() {
    let errors = check(
        r#"
page Bad {
    state {
        a = "x"
        b = "y"
    }
    ok = a < b
}
"#,
    );
    assert_error_contains(&errors, "comparison requires Number");
}

// ── Error suggestions ─────────────────────────────────────────────────────────

fn assert_help_contains(errors: &[TypeError], substr: &str) {
    assert!(
        errors
            .iter()
            .any(|e| e.help.as_deref().unwrap_or("").contains(substr)),
        "expected a help message containing {:?}, got: {:?}",
        substr,
        errors
            .iter()
            .map(|e| e.help.as_deref().unwrap_or("(none)"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_bool_prop_used_as_ident_suggests_colon_true() {
    let errors = check(
        r#"
page Bad {
    text("Sign in") { fontWeight: bold }
}
"#,
    );
    assert_error_contains(&errors, "undefined variable `bold`");
    assert_help_contains(&errors, "bold: true");
}

#[test]
fn test_italic_as_ident_suggests_colon_true() {
    let errors = check(
        r#"
page Bad {
    text("Hi") { fontStyle: italic }
}
"#,
    );
    assert_error_contains(&errors, "undefined variable `italic`");
    assert_help_contains(&errors, "italic: true");
}

#[test]
fn test_unknown_ident_has_no_help() {
    let errors = check(
        r#"
page Bad {
    text(unknownVar)
}
"#,
    );
    assert_error_contains(&errors, "undefined variable `unknownVar`");
    assert!(
        errors.iter().all(|e| e.help.is_none()),
        "expected no help for truly unknown identifiers"
    );
}
