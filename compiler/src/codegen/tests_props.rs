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

// ── Bug fixes ────────────────────────────────────────────────────────────────

#[test]
fn test_comparison_in_if_valid_wasm() {
    let wasm = compile(
        r#"
page P {
    state { count = 0 }
    fn inc() { count = count + 1 }
    if count > 0 { text("positive") }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_logical_and_in_if_valid_wasm() {
    let wasm = compile(
        r#"
page P {
    state { a = false   b = false }
    if a and b { text("both") }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_bool_state_assignment_valid_wasm() {
    let wasm = compile(
        r#"
page P {
    state { active = false }
    fn toggle() { active = true }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_string_interpolation_valid_wasm() {
    let wasm = compile(
        r#"
page P {
    state { count = 0 }
    text("Count: {count}")
}
"#,
    );
    validate_wasm(&wasm);
}

// ── Visual props ─────────────────────────────────────────────────────────────

#[test]
fn test_visual_props_valid_wasm() {
    let wasm = compile(
        r#"
page Profile {
    column(background: white, radius: 12, padding: 24) {
        text("Hello", color: gray700, size: 24)
        text("Sub", color: gray400, size: 14)
        Button("Save", background: blue)
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_hex_color_valid_wasm() {
    let wasm = compile(r#"page P { rect(background: #3B82F6, radius: 8) { text("hi") } }"#);
    validate_wasm(&wasm);
}

#[test]
fn test_enum_props_valid_wasm() {
    let wasm = compile(
        r#"
page P {
    column(align: center, gap: 16) {
        text("Title", align: center, size: 32)
        text("sub", overflow: ellipsis, lines: 1)
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_font_size_alias() {
    let wasm = compile(r#"page P { text("hi", fontSize: 24) }"#);
    validate_wasm(&wasm);
}

#[test]
fn test_font_weight_alias() {
    let wasm = compile(r#"page P { text("hi", fontWeight: 700) }"#);
    validate_wasm(&wasm);
}

#[test]
fn test_hover_state_block() {
    let wasm = compile(r#"page P { rect(hover: { background: gray50 }) }"#);
    validate_wasm(&wasm);
}

#[test]
fn test_focus_state_block() {
    let wasm = compile(r#"page P { input(email, focus: { borderColor: blue, border: 2 }) }"#);
    validate_wasm(&wasm);
}

#[test]
fn test_active_state_block() {
    let wasm = compile(r#"page P { Button("OK", active: { background: #1a3a8a }) }"#);
    validate_wasm(&wasm);
}

#[test]
fn test_hover_and_base_props() {
    let wasm = compile(
        r#"page P { rect(background: gray200, radius: 8, hover: { background: gray50, animate: true }) }"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_grid_fixed_columns() {
    let wasm = compile(r#"page P { grid(columns: 3, gap: 16) { text("a") text("b") text("c") } }"#);
    validate_wasm(&wasm);
}

#[test]
fn test_grid_auto_columns() {
    let wasm = compile(
        r#"page P { grid(columns: auto, minWidth: 200, gap: 24) { text("a") text("b") } }"#,
    );
    validate_wasm(&wasm);
}
