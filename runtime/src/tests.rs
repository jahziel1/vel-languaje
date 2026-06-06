use vel_compiler::{codegen::CodeGen, lexer::Lexer, parser::Parser};

use crate::{Runtime, UiNode, render_debug};

// ── Helpers ───────────────────────────────────────────────────────────────────

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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_simple_text() {
    let tree = run(r#"page Home { text("hello") }"#, "Home");
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].tag_name, "text");
    assert_eq!(tree[0].text.as_deref(), Some("hello"));
}

#[test]
fn test_nested_column_with_text() {
    let tree = run(
        r#"
page Home {
    column(padding: 16, gap: 8) {
        text("Title")
        text("Body")
    }
}
"#,
        "Home",
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].tag_name, "column");
    assert_eq!(tree[0].children.len(), 2);
    assert_eq!(tree[0].children[0].tag_name, "text");
    assert_eq!(tree[0].children[1].tag_name, "text");
}

#[test]
fn test_props_on_element() {
    use crate::tree::PropValue;
    let tree = run(
        r#"page P { column(padding: 24, gap: 16) { text("x") } }"#,
        "P",
    );
    let col = &tree[0];
    assert_eq!(col.tag_name, "column");
    let padding = col.props.iter().find(|p| p.key_name == "padding");
    assert!(padding.is_some());
    if let Some(p) = padding {
        assert!(matches!(p.value, PropValue::Number(24.0)));
    }
    let gap = col.props.iter().find(|p| p.key_name == "gap");
    assert!(gap.is_some());
}

#[test]
fn test_multiple_top_level_elements() {
    let tree = run(
        r#"
page Home {
    text("one")
    text("two")
    text("three")
}
"#,
        "Home",
    );
    assert_eq!(tree.len(), 3);
    assert!(tree.iter().all(|n| n.tag_name == "text"));
}

#[test]
fn test_if_else_correct_branch() {
    let tree = run(
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
        "Login",
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].text.as_deref(), Some("Please log in"));
}

#[test]
fn test_if_true_branch() {
    let tree = run(
        r#"
page P {
    state { active = true }
    if active { text("yes") } else { text("no") }
}
"#,
        "P",
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].text.as_deref(), Some("yes"));
}

#[test]
fn test_match_bool_false() {
    let tree = run(
        r#"
page P {
    state { flag = false }
    match flag {
        true  -> text("on")
        false -> text("off")
    }
}
"#,
        "P",
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].text.as_deref(), Some("off"));
}

#[test]
fn test_match_loading_variant() {
    let tree = run(
        r#"
page Store {
    state { products = 0 }
    match products {
        Loading       -> spinner()
        Success(data) -> text("ok")
        Error(msg)    -> text("err")
    }
}
"#,
        "Store",
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].tag_name, "spinner");
}

#[test]
fn test_match_wildcard() {
    let tree = run(
        r#"
page P {
    state { flag = true }
    match flag {
        true -> text("yes")
        _    -> text("fallback")
    }
}
"#,
        "P",
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].text.as_deref(), Some("yes"));
}

#[test]
fn test_state_does_not_crash() {
    let tree = run(
        r#"
page Counter {
    state { count = 0 }
    fn increment() {
        count = count + 1
    }
    column(padding: 16) {
        text("Count")
        Button("Increment")
    }
}
"#,
        "Counter",
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].tag_name, "column");
}

#[test]
fn test_render_debug_produces_output() {
    let tree = run(
        r#"page Home { column(padding: 16) { text("hello") } }"#,
        "Home",
    );
    let debug = render_debug(&tree);
    assert!(debug.contains("column"));
    assert!(debug.contains("text"));
    assert!(debug.contains("padding=16"));
    assert!(debug.contains("hello"));
}

#[test]
fn test_button_text_content() {
    let tree = run(r#"page P { Button("Click me") }"#, "P");
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].text.as_deref(), Some("Click me"));
}

#[test]
fn test_multiple_strings_deduped() {
    let tree = run(
        r#"
page Home {
    text("Title")
    text("Body")
    text("Title")
}
"#,
        "Home",
    );
    assert_eq!(tree.len(), 3);
    assert_eq!(tree[0].text.as_deref(), Some("Title"));
    assert_eq!(tree[1].text.as_deref(), Some("Body"));
    assert_eq!(tree[2].text.as_deref(), Some("Title"));
}

#[test]
fn test_spinner_element() {
    let tree = run(r#"page Loading { spinner() }"#, "Loading");
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].tag_name, "spinner");
}

#[test]
fn test_button_element() {
    let tree = run(r#"page P { Button("Click me") }"#, "P");
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].tag_name, "Button");
}

#[test]
fn test_grid_tree() {
    let tree = run(
        r#"page P { grid(columns: 3, gap: 8) { text("a") text("b") text("c") text("d") } }"#,
        "P",
    );
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].tag_name, "grid");
    assert_eq!(tree[0].children.len(), 4);
}

#[test]
fn test_print_does_not_crash() {
    // print() should emit valid WASM and execute without errors
    let wasm = compile(
        r#"
page P {
    state { count = 0 }
    fn inc() { count = count + 1   print("count: {count}") }
    Button("inc", onClick: inc) {}
}
"#,
    );
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_P").expect("instantiate");
    inst.render().expect("render");
    inst.call_handler("P_inc").expect("handler");
    inst.render().expect("render after print");
}
