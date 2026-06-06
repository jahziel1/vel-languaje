use super::CodeGen;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn compile(source: &str) -> Vec<u8> {
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

fn compile_gen(source: &str) -> CodeGen {
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut cg = CodeGen::new();
    cg.generate(&program);
    cg
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

// ── API state ────────────────────────────────────────────────────────────────

#[test]
fn test_api_state_creates_two_globals() {
    let cg = compile_gen(
        r#"
page Profile {
    state { user = try api.get("/user") }
    text("hello")
}
"#,
    );
    assert_eq!(cg.globals.len(), 2, "expected status + reqid globals");
    use wasm_encoder::ValType;
    assert_eq!(cg.globals[0].val_type, ValType::I32, "status must be i32");
    assert_eq!(cg.globals[1].val_type, ValType::I32, "reqid must be i32");
}

#[test]
fn test_api_state_valid_wasm() {
    let wasm = compile(
        r#"
page Profile {
    state { user = try api.get("/user") }
    match user {
        Loading       -> text("Loading...")
        Error(msg)    -> text("Error")
        Success(data) -> text("Loaded")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_api_state_mixed_with_regular_state() {
    let cg = compile_gen(
        r#"
page Store {
    state {
        products = try api.get("/products")
        query    = ""
    }
    text("hello")
}
"#,
    );
    // products → 2 globals (status i32 + reqid i32); query = "" → Text state (host-side, no WASM global)
    assert_eq!(cg.globals.len(), 2);
    use wasm_encoder::ValType;
    assert_eq!(cg.globals[0].val_type, ValType::I32);
    assert_eq!(cg.globals[1].val_type, ValType::I32);
    assert!(cg.text_state.contains("Store/query"));
}

#[test]
fn test_api_post_valid_wasm() {
    let wasm = compile(
        r#"
page Checkout {
    state { result = try api.post("/order") }
    match result {
        Loading       -> text("Submitting...")
        Error(msg)    -> text("Failed")
        Success(data) -> text("Done")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

// ── API field access ──────────────────────────────────────────────────────────

#[test]
fn test_api_field_access_valid_wasm() {
    let wasm = compile(
        r#"
page Profile {
    state { user = try api.get("/user") }
    match user {
        Loading       -> text("Loading...")
        Error(msg)    -> text("Error")
        Success(data) -> text(data.name)
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_api_field_interpolation_valid_wasm() {
    let wasm = compile(
        r#"
page Product {
    state { item = try api.get("/item") }
    match item {
        Loading       -> text("Loading")
        Error(msg)    -> text("Error")
        Success(data) -> text("{data.name} costs {data.price}")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_api_field_binding_scoped_to_arm() {
    // data binding must not leak into Error arm
    let wasm = compile(
        r#"
page Store {
    state { products = try api.get("/products") }
    match products {
        Loading       -> text("wait")
        Success(data) -> text(data.title)
        Error(msg)    -> text("err")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

// ── Full program ──────────────────────────────────────────────────────────────

#[test]
fn test_store_page_valid_wasm() {
    let wasm = compile(
        r#"
page Store {
    state {
        products = try api.get("/products")
        search = ""
    }
    column(padding: 24, gap: 16) {
        text("Welcome to the store", size: 32, weight: 700)
        input(search, placeholder: "Search products...")
        match products {
            Loading       -> spinner()
            Error(msg)    -> text("Error")
            Success(data) -> text("ok")
        }
    }
}
"#,
    );
    validate_wasm(&wasm);
}
