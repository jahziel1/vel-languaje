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

#[test]
fn test_api_dynamic_url_valid_wasm() {
    let wasm = compile(
        r#"
page Product {
    state {
        id      = 1
        product = try api.get("/products/{id}")
    }
    match product {
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
fn test_api_dynamic_url_creates_correct_globals() {
    let cg = compile_gen(
        r#"
page Product {
    state {
        id      = 1
        product = try api.get("/products/{id}")
    }
    text("ok")
}
"#,
    );
    // id (f64) + product_status (i32) + product_reqid (i32) = 3 globals
    assert_eq!(cg.globals.len(), 3);
    use wasm_encoder::ValType;
    assert_eq!(cg.globals[0].val_type, ValType::F64, "id must be f64");
    assert_eq!(cg.globals[1].val_type, ValType::I32, "status must be i32");
    assert_eq!(cg.globals[2].val_type, ValType::I32, "reqid must be i32");
}

#[test]
fn test_api_dynamic_url_multi_segment() {
    let wasm = compile(
        r#"
page Order {
    state {
        userId  = 1
        orderId = 2
        detail  = try api.get("/users/{userId}/orders/{orderId}")
    }
    match detail {
        Loading       -> text("...")
        Error(msg)    -> text("err")
        Success(data) -> text("ok")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_api_dynamic_url_with_body() {
    let wasm = compile(
        r#"
page Edit {
    state {
        id     = 1
        result = try api.put("/items/{id}", { active: true })
    }
    match result {
        Loading       -> text("...")
        Error(msg)    -> text("err")
        Success(data) -> text("ok")
    }
}
"#,
    );
    validate_wasm(&wasm);
}
