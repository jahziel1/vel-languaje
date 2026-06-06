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

// ── Structure ─────────────────────────────────────────────────────────────────

#[test]
fn test_wasm_magic_bytes() {
    let wasm = compile(r#"page Home { text("Hello") }"#);
    assert!(wasm.len() > 8);
    assert_eq!(&wasm[..4], b"\0asm");
}

#[test]
fn test_wasm_version_1() {
    let wasm = compile(r#"page Home { text("Hello") }"#);
    assert_eq!(&wasm[4..8], &[1, 0, 0, 0]);
}

#[test]
fn test_valid_wasm_simple() {
    let wasm = compile(r#"page Home { text("Hello") }"#);
    validate_wasm(&wasm);
}

#[test]
fn test_valid_wasm_with_state() {
    let wasm = compile(
        r#"
page Counter {
    state { count = 0 }
    fn increment() {
        count = count + 1
    }
    column(padding: 16) {
        text("count")
        Button("Increment")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_valid_wasm_with_if() {
    let wasm = compile(
        r#"
page Login {
    state { loggedIn = false }
    if loggedIn {
        text("Welcome")
    } else {
        text("Please log in")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

// ── State globals ─────────────────────────────────────────────────────────────

#[test]
fn test_state_globals_count() {
    let cg = compile_gen(
        r#"
page Shop {
    state {
        count = 0
        name = "Alice"
        active = false
    }
    text("x")
}
"#,
    );
    // name = "Alice" is Text state (host-side) — only count (f64) + active (i32) as WASM globals
    assert_eq!(cg.globals.len(), 2);
}

#[test]
fn test_state_global_number_is_f64() {
    use wasm_encoder::ValType;
    let cg = compile_gen(r#"page P { state { n = 42 } text("x") }"#);
    assert_eq!(cg.globals[0].val_type, ValType::F64);
    assert_eq!(cg.globals[0].init_f64, 42.0);
}

#[test]
fn test_state_global_bool_is_i32() {
    use wasm_encoder::ValType;
    let cg = compile_gen(r#"page P { state { flag = false } text("x") }"#);
    assert_eq!(cg.globals[0].val_type, ValType::I32);
    assert_eq!(cg.globals[0].init_f64, 0.0);
}

// ── Functions ─────────────────────────────────────────────────────────────────

#[test]
fn test_page_exports_render_function() {
    let cg = compile_gen(r#"page Home { text("hello") }"#);
    let names = cg.func_export_names();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], Some("page_Home"));
}

#[test]
fn test_page_with_fn_exports_both() {
    let cg = compile_gen(
        r#"
page Counter {
    state { count = 0 }
    fn increment() {
        count = count + 1
    }
    text("x")
}
"#,
    );
    let names = cg.func_export_names();
    assert_eq!(names.len(), 2);
    assert_eq!(names[0], Some("page_Counter"));
    assert_eq!(names[1], Some("Counter_increment"));
}

#[test]
fn test_multiple_pages() {
    let cg = compile_gen(
        r#"
page Home  { text("home") }
page About { text("about") }
"#,
    );
    assert_eq!(cg.func_count(), 2);
}

#[test]
fn test_component_export() {
    let cg = compile_gen(r#"component Card(title: Text) { text("x") }"#);
    let names = cg.func_export_names();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0], Some("component_Card"));
}

// ── Derived ───────────────────────────────────────────────────────────────────

#[test]
fn test_derived_valid_wasm() {
    let wasm = compile(
        r#"
page Cart {
    state {
        count = 3
        price = 10
    }
    derived {
        subtotal = count + price
        doubled  = count + count
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_derived_registers_locals() {
    let cg = compile_gen(
        r#"
page P {
    state { n = 0 }
    derived { big = n + n }
}
"#,
    );
    // `big` is a pre-scanned local — no extra globals beyond state
    assert_eq!(cg.globals.len(), 1); // only `n`
}

// ── On change ─────────────────────────────────────────────────────────────────

#[test]
fn test_on_change_valid_wasm() {
    let wasm = compile(
        r#"
page Search {
    state {
        query  = 0
        result = 0
    }
    on change query {
        result = query + 1
    }
    fn setQuery() { query = 5 }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_on_change_creates_shadow_global() {
    let cg = compile_gen(
        r#"
page P {
    state { x = 0 }
    on change x { x = x + 1 }
}
"#,
    );
    // `x` (state) + `x__prev` (shadow) = 2 globals
    assert_eq!(cg.globals.len(), 2);
}
