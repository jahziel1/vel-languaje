use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::ast::*;

fn parse(src: &str) -> Program {
    let tokens = Lexer::new(src).tokenize();
    Parser::new(tokens).parse().expect("parse failed")
}

// ── Page ──────────────────────────────────────────────────────────────────────

#[test]
fn test_parse_empty_page() {
    let prog = parse("page Home { }");
    assert_eq!(prog.items.len(), 1);
    let Item::Page(page) = &prog.items[0] else {
        panic!("expected Page");
    };
    assert_eq!(page.name, "Home");
    assert!(page.params.is_empty());
    assert!(page.body.is_empty());
}

#[test]
fn test_parse_page_with_params() {
    let prog = parse("page Product(id: Number) { }");
    let Item::Page(page) = &prog.items[0] else {
        panic!()
    };
    assert_eq!(page.params.len(), 1);
    assert_eq!(page.params[0].name, "id");
    assert!(matches!(page.params[0].ty, Type::Number));
}

// ── Component ─────────────────────────────────────────────────────────────────

#[test]
fn test_parse_component() {
    let prog = parse(r#"component Card(title: Text) { text(title) }"#);
    let Item::Component(comp) = &prog.items[0] else {
        panic!()
    };
    assert_eq!(comp.name, "Card");
    assert_eq!(comp.params.len(), 1);
    assert_eq!(comp.body.len(), 1);
}

// ── Types and enums ───────────────────────────────────────────────────────────

#[test]
fn test_parse_type_def() {
    let prog = parse("type User { name: Text age: Number }");
    let Item::TypeDef(td) = &prog.items[0] else {
        panic!()
    };
    assert_eq!(td.name, "User");
    assert_eq!(td.fields.len(), 2);
    assert_eq!(td.fields[0].name, "name");
}

#[test]
fn test_parse_enum() {
    let prog = parse("enum Status { Loading | Success | Error }");
    let Item::EnumDef(ed) = &prog.items[0] else {
        panic!()
    };
    assert_eq!(ed.name, "Status");
    assert_eq!(ed.variants.len(), 3);
    assert_eq!(ed.variants[0].name, "Loading");
}

// ── Import ────────────────────────────────────────────────────────────────────

#[test]
fn test_parse_import_default() {
    let prog = parse(r#"import Button from "components/button""#);
    let Item::Import(imp) = &prog.items[0] else {
        panic!()
    };
    let ImportNames::Default(name) = &imp.names else {
        panic!()
    };
    assert_eq!(name, "Button");
    assert_eq!(imp.path, "components/button");
}

#[test]
fn test_parse_import_named() {
    let prog = parse(r#"import { Card, Modal } from "components""#);
    let Item::Import(imp) = &prog.items[0] else {
        panic!()
    };
    let ImportNames::Named(names) = &imp.names else {
        panic!()
    };
    assert_eq!(names, &["Card", "Modal"]);
}

// ── State block ───────────────────────────────────────────────────────────────

#[test]
fn test_parse_state_block() {
    let prog = parse(r#"page Home { state { count = 0 name = "Ana" } }"#);
    let Item::Page(page) = &prog.items[0] else {
        panic!()
    };
    let Stmt::State(state) = &page.body[0] else {
        panic!()
    };
    assert_eq!(state.entries.len(), 2);
    assert_eq!(state.entries[0].name, "count");
}

// ── If statement ──────────────────────────────────────────────────────────────

#[test]
fn test_parse_if() {
    let prog = parse("page P { if loading { spinner() } }");
    let Item::Page(page) = &prog.items[0] else {
        panic!()
    };
    assert!(matches!(page.body[0], Stmt::If(_)));
}

// ── Match statement ───────────────────────────────────────────────────────────

#[test]
fn test_parse_match() {
    let prog = parse("page P { match status { Loading -> spinner() Error(msg) -> text(msg) } }");
    let Item::Page(page) = &prog.items[0] else {
        panic!()
    };
    let Stmt::Match(m) = &page.body[0] else {
        panic!()
    };
    assert_eq!(m.arms.len(), 2);
    assert!(matches!(m.arms[0].pattern, Pattern::Ident(_)));
    assert!(matches!(m.arms[1].pattern, Pattern::Variant(_, _)));
}

// ── Expressions ───────────────────────────────────────────────────────────────

#[test]
fn test_parse_binary_op() {
    let prog = parse("page P { x = 1 + 2 * 3 }");
    let Item::Page(page) = &prog.items[0] else {
        panic!()
    };
    let Stmt::Let(l) = &page.body[0] else {
        panic!()
    };
    // 1 + (2 * 3) — multiplication binds tighter
    let Expr::BinOp(_, BinOp::Add, rhs) = &l.value else {
        panic!("expected Add at top level")
    };
    assert!(matches!(rhs.as_ref(), Expr::BinOp(_, BinOp::Mul, _)));
}

#[test]
fn test_parse_string_interpolation() {
    let prog = parse(r#"page P { text("Hello {name}!") }"#);
    let Item::Page(page) = &prog.items[0] else {
        panic!()
    };
    let Stmt::Expr(Expr::Call(_, args, _)) = &page.body[0] else {
        panic!()
    };
    let Expr::Str(parts) = &args[0].value else {
        panic!()
    };
    assert_eq!(parts.len(), 3); // "Hello ", {name}, "!"
}

#[test]
fn test_parse_field_access() {
    let prog = parse("page P { x = user.name }");
    let Item::Page(page) = &prog.items[0] else {
        panic!()
    };
    let Stmt::Let(l) = &page.body[0] else {
        panic!()
    };
    assert!(matches!(&l.value, Expr::Field(_, _)));
}
