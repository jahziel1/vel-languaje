use crate::codegen::CodeGen;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn compile(src: &str) -> Vec<u8> {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

fn export_names(src: &str) -> Vec<String> {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut cg = CodeGen::new();
    cg.generate(&program);
    cg.func_export_names()
        .into_iter()
        .flatten()
        .map(|s| s.to_owned())
        .collect()
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

// ── store api.headers — no headers without store ─────────────────────────────

#[test]
fn api_call_without_store_headers_valid_wasm() {
    let src = r#"
page Home {
    state { me = try api.get("/me") }
    match me {
        Loading       -> text("...")
        Error(msg)    -> text("err")
        Success(data) -> text("ok")
    }
}
"#;
    validate_wasm(&compile(src));
}

// ── multiple stores ───────────────────────────────────────────────────────────

#[test]
fn multiple_stores_compile() {
    let src = r#"
store auth {
    state { loggedIn: Bool = false }
    fn login() { loggedIn = true }
    fn logout() { loggedIn = false }
}
store cart {
    state { count: Number = 0 }
    fn add() { count = count + 1 }
}
page Home {
    if auth.loggedIn {
        text("{cart.count} items")
    }
    fn doAdd() { cart.add() }
}
"#;
    validate_wasm(&compile(src));
    let names = export_names(src);
    assert!(names.contains(&"store_auth_login".to_owned()));
    assert!(names.contains(&"store_auth_logout".to_owned()));
    assert!(names.contains(&"store_cart_add".to_owned()));
}
