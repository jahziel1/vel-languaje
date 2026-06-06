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

// ── lifecycle exports ─────────────────────────────────────────────────────────

#[test]
fn test_on_mount_emits_lifecycle_export() {
    let cg = compile_gen(
        r#"
page Home {
    state { count = 0 }
    on mount { count = 1 }
    text("{count}")
}
"#,
    );
    let exports: Vec<_> = cg.func_export_names().into_iter().flatten().collect();
    assert!(
        exports.contains(&"page_Home__mount"),
        "expected page_Home__mount export, got: {:?}",
        exports
    );
}

#[test]
fn test_on_unmount_emits_lifecycle_export() {
    let cg = compile_gen(
        r#"
page Home { text("hi") }
page Other {
    state { x = 0 }
    on unmount { x = 1 }
    text("{x}")
}
"#,
    );
    let exports: Vec<_> = cg.func_export_names().into_iter().flatten().collect();
    assert!(
        exports.contains(&"page_Other__unmount"),
        "expected page_Other__unmount export, got: {:?}",
        exports
    );
}

#[test]
fn test_on_mount_wasm_valid() {
    let wasm = compile(
        r#"
page Home {
    state { count = 0 }
    on mount { count = 1 }
    text("{count}")
}
"#,
    );
    validate_wasm(&wasm);
}

// ── responsive ────────────────────────────────────────────────────────────────

#[test]
fn test_responsive_phone_wasm_valid() {
    let wasm = compile(
        r#"
page Home {
    on phone {
        text("mobile layout")
    }
    on desktop {
        text("desktop layout")
    }
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_all_breakpoints_wasm_valid() {
    let wasm = compile(
        r#"
page Home {
    on phone   { text("phone") }
    on mobile  { text("mobile") }
    on tablet  { text("tablet") }
    on desktop { text("desktop") }
    on wide    { text("wide") }
}
"#,
    );
    validate_wasm(&wasm);
}
