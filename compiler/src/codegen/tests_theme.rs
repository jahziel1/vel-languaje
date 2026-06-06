use super::CodeGen;
use crate::checker::Checker;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn compile(src: &str) -> Vec<u8> {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

fn compile_gen(src: &str) -> CodeGen {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut cg = CodeGen::new();
    cg.generate(&program);
    cg
}

fn check_errors(src: &str) -> Vec<String> {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut checker = Checker::new();
    let errors = checker.check(&program);
    errors.iter().map(|e| e.message.clone()).collect()
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

const THEME_SRC: &str = r#"
export theme {
    colors {
        primary:     #3B82F6
        primaryDark: #2563EB
        danger:      #EF4444
        success:     #10B981
        gray100:     #F3F4F6
        gray200:     #E5E7EB
        gray700:     #374151
    }
    text {
        xs: 12
        sm: 14
        md: 16
        lg: 18
        xl: 24
        h3: 28
        h2: 36
        h1: 48
    }
    radius {
        sm: 4
        md: 8
        lg: 12
        xl: 20
        full: 9999
    }
}
"#;

// ── 1. Theme file parses without errors ───────────────────────────────────────

#[test]
fn theme_parses_no_checker_errors() {
    let src = format!("{}\npage Home {{ text(\"hi\") }}", THEME_SRC);
    let errors = check_errors(&src);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
}

// ── 2. theme.colors.primary → correct packed RGB constant ────────────────────

#[test]
fn theme_color_primary_collected() {
    let src = format!("{}\npage Home {{ text(\"hi\") }}", THEME_SRC);
    let cg = compile_gen(&src);
    let rgb = cg.theme_colors.get("colors/primary").copied().unwrap_or(0);
    // #3B82F6 = 0x3B82F6
    assert_eq!(rgb, 0x3B_82_F6, "expected 0x3B82F6, got 0x{:06X}", rgb);
}

// ── 3. theme.text.h3 → F64Const(28.0) ────────────────────────────────────────

#[test]
fn theme_text_h3_collected() {
    let src = format!("{}\npage Home {{ text(\"hi\") }}", THEME_SRC);
    let cg = compile_gen(&src);
    let n = cg.theme_numbers.get("text/h3").copied().unwrap_or(0.0);
    assert!((n - 28.0).abs() < f64::EPSILON, "expected 28.0, got {}", n);
}

// ── 4. theme.radius.lg → F64Const(12.0) ──────────────────────────────────────

#[test]
fn theme_radius_lg_collected() {
    let src = format!("{}\npage Home {{ text(\"hi\") }}", THEME_SRC);
    let cg = compile_gen(&src);
    let n = cg.theme_numbers.get("radius/lg").copied().unwrap_or(0.0);
    assert!((n - 12.0).abs() < f64::EPSILON, "expected 12.0, got {}", n);
}

// ── 5. Unknown theme token produces a checker error ───────────────────────────

#[test]
fn theme_unknown_token_produces_error() {
    let src = format!(
        r#"{}
page Home {{
    rect(background: theme.colors.nonexistent) {{
        text("hi")
    }}
}}"#,
        THEME_SRC
    );
    let errors = check_errors(&src);
    assert!(
        errors.iter().any(|e| e.contains("nonexistent")),
        "expected error about nonexistent, got: {:?}",
        errors
    );
}

// ── 6. Theme used in rect compiles to valid WASM ──────────────────────────────

#[test]
fn theme_in_rect_compiles_valid_wasm() {
    let src = format!(
        r#"{}
page Home {{
    rect(
        background: theme.colors.primary,
        radius: theme.radius.lg,
        border: 1,
        borderColor: theme.colors.gray200
    ) {{
        text("Title", size: theme.text.h3, color: theme.colors.gray700)
    }}
}}"#,
        THEME_SRC
    );
    validate_wasm(&compile(&src));
}

// ── 7. Multiple theme sections all collected ──────────────────────────────────

#[test]
fn all_theme_sections_collected() {
    let src = format!("{}\npage Home {{ text(\"hi\") }}", THEME_SRC);
    let cg = compile_gen(&src);
    assert!(
        cg.theme_colors.contains_key("colors/danger"),
        "danger color missing"
    );
    assert!(cg.theme_numbers.contains_key("text/xl"), "text/xl missing");
    assert!(
        cg.theme_numbers.contains_key("radius/full"),
        "radius/full missing"
    );
}
