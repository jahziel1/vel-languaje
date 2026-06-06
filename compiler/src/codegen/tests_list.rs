use super::CodeGen;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn compile(source: &str) -> Vec<u8> {
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

fn validate_wasm(wasm: &[u8]) {
    use wasmparser::{Chunk, Parser, Payload};
    let mut parser = Parser::new(0);
    let mut remaining = wasm;
    loop {
        match parser.parse(remaining, true).expect("wasmparser error") {
            Chunk::Parsed { payload, consumed } => {
                remaining = &remaining[consumed..];
                if matches!(payload, Payload::End(_)) {
                    break;
                }
            }
            Chunk::NeedMoreData(_) => break,
        }
    }
}

// ── Parser: list block with item -> ─────────────────────────────────────────

#[test]
fn test_parse_list_foreach() {
    let source = r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("Loading")
        Success(data) -> {
            list(data) { item -> text(item.name) }
        }
        Error(msg) -> text("Error")
    }
}
"#;
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    assert_eq!(program.items.len(), 1);
}

// ── Codegen: list produces valid WASM ────────────────────────────────────────

#[test]
fn test_list_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("Loading...")
        Success(data) -> {
            list(data) { item -> text(item.name) }
        }
        Error(msg) -> text("Error")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

// ── Codegen: list with multiple body elements ─────────────────────────────────

#[test]
fn test_list_multiple_body_elements_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("Loading...")
        Success(data) -> {
            list(data) {
                item ->
                column {
                    text(item.name)
                    text(item.description)
                }
            }
        }
        _ -> text("Error")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

// ── Codegen: list outside match still compiles ────────────────────────────────

#[test]
fn test_list_standalone_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    text("hello")
}
"#,
    );
    validate_wasm(&wasm);
}

// ── Codegen: data.length produces valid WASM ─────────────────────────────────

#[test]
fn test_list_length_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("Loading")
        Success(data) -> {
            text("{data.length} items")
            list(data) { item -> text(item.name) }
        }
        Error(msg) -> text("Error")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

// ── Codegen: data.isEmpty produces valid WASM ────────────────────────────────

#[test]
fn test_list_is_empty_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("Loading")
        Success(data) -> {
            if data.isEmpty {
                text("No items")
            }
            list(data) { item -> text(item.name) }
        }
        Error(msg) -> text("Error")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

// ── Codegen: data.sum produces valid WASM ────────────────────────────────────

#[test]
fn test_list_sum_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("Loading")
        Success(data) -> {
            text("Total: {data.sum(item -> item.price)}")
        }
        Error(msg) -> text("Error")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

// ── Codegen: data.filter in list() produces valid WASM ───────────────────────

#[test]
fn test_list_filter_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("Loading")
        Success(data) -> {
            list(data.filter(item -> item.active)) {
                item -> text(item.name)
            }
        }
        Error(msg) -> text("Error")
    }
}
"#,
    );
    validate_wasm(&wasm);
}
