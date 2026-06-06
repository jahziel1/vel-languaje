use vel_compiler::{codegen::CodeGen, lexer::Lexer, parser::Parser};

use crate::{Runtime, UiNode, render_debug};

fn compile(source: &str) -> Vec<u8> {
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

fn run(source: &str, page: &str) -> Vec<UiNode> {
    let wasm = compile(source);
    let rt = Runtime::new();
    rt.run_page(&wasm, &format!("page_{}", page))
        .expect("runtime error")
}

// ── Bug fixes ────────────────────────────────────────────────────────────────

#[test]
fn test_comparison_in_if_renders_correctly() {
    let source = r#"
page P {
    state { count = 0 }
    if count > 0 { text("positive") }
    text("always")
}
"#;
    let tree = run(source, "P");
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].text.as_deref(), Some("always"));
}

#[test]
fn test_bool_toggle_renders_correctly() {
    let source = r#"
page P {
    state { active = false }
    fn toggle() { active = true }
    if active { text("on") }
    text("base")
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_P").unwrap();
    let tree = inst.render().unwrap();
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].text.as_deref(), Some("base"));
    inst.call_handler("P_toggle").unwrap();
    let tree2 = inst.render().unwrap();
    assert_eq!(tree2.len(), 2);
    assert_eq!(tree2[0].text.as_deref(), Some("on"));
    assert_eq!(tree2[1].text.as_deref(), Some("base"));
}

#[test]
fn test_string_interpolation_renders_number() {
    let source = r#"
page P {
    state { count = 42 }
    text("Count: {count}")
}
"#;
    let tree = run(source, "P");
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].text.as_deref(), Some("Count: 42"));
}

// ── Derived ───────────────────────────────────────────────────────────────────

#[test]
fn test_derived_does_not_crash() {
    let source = r#"
page Cart {
    state { count = 3 }
    derived { doubled = count + count }
    text("items")
}
"#;
    let tree = run(source, "Cart");
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].tag_name, "text");
}

// ── On change ─────────────────────────────────────────────────────────────────

#[test]
fn test_on_change_does_not_crash() {
    let source = r#"
page Counter {
    state {
        count  = 0
        ticked = 0
    }
    on change count {
        ticked = ticked + 1
    }
    fn inc() { count = count + 1 }
    text("hello")
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Counter").unwrap();
    let tree = inst.render().unwrap();
    assert_eq!(tree.len(), 1);
    inst.call_handler("Counter_inc").unwrap();
    let tree2 = inst.render().unwrap();
    assert_eq!(tree2.len(), 1);
    let tree3 = inst.render().unwrap();
    assert_eq!(tree3.len(), 1);
}

// Silence unused import warning
#[allow(dead_code)]
fn _uses(_: &str) {
    let _ = render_debug;
}
