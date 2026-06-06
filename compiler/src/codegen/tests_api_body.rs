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
fn test_api_post_with_static_number_body() {
    let wasm = compile(
        r#"
page Order {
    state { result = try api.post("/order", { quantity: 2 }) }
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

#[test]
fn test_api_post_with_static_bool_body() {
    let wasm = compile(
        r#"
page Toggle {
    state { result = try api.post("/toggle", { active: true }) }
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

#[test]
fn test_api_post_with_static_string_body() {
    let wasm = compile(
        r#"
page Login {
    state { result = try api.post("/login", { role: "admin" }) }
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

#[test]
fn test_api_post_with_variable_body() {
    let wasm = compile(
        r#"
page Cart {
    state {
        count    = 1
        result   = try api.post("/order", { quantity: count })
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

#[test]
fn test_api_put_with_body() {
    let wasm = compile(
        r#"
page Edit {
    state { result = try api.put("/item/1", { price: 9.99 }) }
    match result {
        Loading       -> text("Saving...")
        Error(msg)    -> text("err")
        Success(data) -> text("Saved")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_api_post_multi_field_body() {
    let wasm = compile(
        r#"
page Signup {
    state { result = try api.post("/signup", { age: 25, verified: false }) }
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
