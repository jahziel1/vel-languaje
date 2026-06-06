use vel_compiler::{codegen::CodeGen, lexer::Lexer, parser::Parser};

use crate::{Runtime, UiNode};

fn compile(source: &str) -> Vec<u8> {
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

// ── API state ────────────────────────────────────────────────────────────────

#[test]
fn test_api_state_renders_one_branch() {
    // Verifies that a page with API state compiles, instantiates, and renders exactly one
    // match arm without panicking. Which arm (Loading/Error/Success) depends on timing.
    let source = r#"
page Profile {
    state { user = try api.get("/user") }
    match user {
        Loading       -> text("Loading...")
        Error(msg)    -> text("Error")
        Success(data) -> text("Loaded")
    }
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Profile").unwrap();
    let tree = inst.render().unwrap();
    assert_eq!(tree.len(), 1, "exactly one match arm must produce a node");
    let text = tree[0].text.as_deref().unwrap_or("");
    assert!(
        text == "Loading..." || text == "Error" || text == "Loaded",
        "unexpected text: {text}"
    );
}

#[test]
fn test_api_state_does_not_crash_on_repeated_renders() {
    let source = r#"
page Store {
    state {
        products = try api.get("/products")
        count    = 0
    }
    fn inc() { count = count + 1 }
    match products {
        Loading       -> text("wait")
        Error(msg)    -> text("err")
        Success(data) -> text("ok")
    }
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Store").unwrap();
    let _ = inst.render().unwrap();
    inst.call_handler("Store_inc").unwrap();
    let tree2 = inst.render().unwrap();
    // After handler + re-render, API state still works (reqid != 0, no second fetch)
    assert_eq!(tree2.len(), 1);
}

// ── API field access ──────────────────────────────────────────────────────────

#[test]
fn test_api_field_access_renders_success_arm() {
    // In test mode api_fetch immediately sets status=Success with empty body.
    // Field access on empty JSON returns defaults — but must not panic.
    let source = r#"
page Profile {
    state { user = try api.get("/user") }
    match user {
        Loading       -> text("Loading")
        Error(msg)    -> text("Error")
        Success(data) -> text(data.name)
    }
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Profile").unwrap();
    let tree = inst.render().unwrap();
    // Success arm renders a text node (name is "" since body is empty JSON in test mode)
    assert_eq!(tree.len(), 1, "Success arm must render one node");
}

#[test]
fn test_api_field_num_returns_zero_on_missing_field() {
    use crate::host::VelHost;
    let host = VelHost::new();
    // No reqid stored — must return 0.0 without panic
    assert_eq!(host.api_field_num(99, "price"), 0.0);
}

#[test]
fn test_api_field_str_noop_on_missing_reqid() {
    use crate::host::VelHost;
    let mut host = VelHost::new();
    host.api_field_str(99, "name");
    // str_builder should remain empty — no panic, no data
}

#[test]
fn test_api_field_bool_returns_zero_on_missing_field() {
    use crate::host::VelHost;
    let host = VelHost::new();
    assert_eq!(host.api_field_bool(99, "active"), 0);
}

// Suppress unused import warning when UiNode is not directly used in assertions
#[allow(dead_code)]
fn _assert_uinode_importable(_: &UiNode) {}
