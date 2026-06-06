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

#[test]
fn test_api_get_with_single_header_valid_wasm() {
    let wasm = compile(
        r#"
page Profile {
    state { me = try api.get("/me", headers: { Authorization: "Bearer token123" }) }
    match me {
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
fn test_api_get_with_multiple_headers_valid_wasm() {
    let wasm = compile(
        r#"
page Data {
    state { resp = try api.get("/data", headers: { Authorization: "Bearer x", Accept: "application/json" }) }
    match resp {
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
fn test_api_post_with_body_and_headers_valid_wasm() {
    let wasm = compile(
        r#"
page Submit {
    state { result = try api.post("/data", { qty: 1 }, headers: { Authorization: "Bearer abc" }) }
    match result {
        Loading       -> text("...")
        Error(msg)    -> text("err")
        Success(data) -> text("done")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_api_get_without_headers_still_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading       -> text("Loading")
        Error(msg)    -> text("Error")
        Success(data) -> text("ok")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_headers_interned_in_data_section() {
    let mut cg = {
        let tokens = Lexer::new(
            r#"
page Auth {
    state { me = try api.get("/me", headers: { Authorization: "Bearer secretkey" }) }
    match me {
        Loading -> text("...")
        Error(msg) -> text("err")
        Success(data) -> text("ok")
    }
}
"#,
        )
        .tokenize();
        let program = Parser::new(tokens).parse().expect("parse failed");
        let mut cg = CodeGen::new();
        cg.generate(&program);
        cg
    };
    let data = String::from_utf8(cg.string_bytes.clone()).unwrap();
    assert!(
        data.contains("Authorization"),
        "header key must be interned"
    );
    assert!(
        data.contains("Bearer secretkey"),
        "header value must be interned"
    );
}
