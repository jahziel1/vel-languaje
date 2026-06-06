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
fn test_text_state_not_a_wasm_global() {
    let cg = compile_gen(
        r#"
page Login {
    state {
        email = ""
        password = ""
    }
    text("Login")
}
"#,
    );
    // email and password are Text state — stored in host, NOT as WASM globals
    assert_eq!(
        cg.globals.len(),
        0,
        "Text state must not create WASM globals"
    );
    assert!(
        cg.text_state.contains("Login/email"),
        "Login/email must be in text_state"
    );
    assert!(
        cg.text_state.contains("Login/password"),
        "Login/password must be in text_state"
    );
}

#[test]
fn test_text_state_mixed_with_number() {
    let cg = compile_gen(
        r#"
page Form {
    state {
        name = ""
        age = 0
    }
    text("Form")
}
"#,
    );
    assert_eq!(cg.globals.len(), 1, "only 'age' should be a WASM global");
    assert!(cg.text_state.contains("Form/name"));
    assert!(!cg.text_state.contains("Form/age"));
}

#[test]
fn test_text_interpolation_valid_wasm() {
    let wasm = compile(
        r#"
page Greeting {
    state { name = "" }
    text("Hello {name}!")
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_text_direct_arg_valid_wasm() {
    let wasm = compile(
        r#"
page Profile {
    state { username = "" }
    text(username)
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_text_state_if_condition_valid_wasm() {
    let wasm = compile(
        r#"
page Form {
    state { name = "" }
    if name {
        text("Has name")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_text_state_assignment_valid_wasm() {
    let wasm = compile(
        r#"
page Auth {
    state { token = "" }
    fn clearToken() {
        token = ""
    }
    text("ok")
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_text_state_assignment_non_empty_valid_wasm() {
    let wasm = compile(
        r#"
page Auth {
    state { greeting = "" }
    fn setGreeting() {
        greeting = "Hello World"
    }
    text(greeting)
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_text_state_with_input_valid_wasm() {
    let wasm = compile(
        r#"
page Search {
    state { query = "" }
    input(query, placeholder: "Search...")
    if query {
        text("Searching for: {query}")
    }
}
"#,
    );
    validate_wasm(&wasm);
}
