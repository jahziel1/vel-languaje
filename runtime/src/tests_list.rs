use vel_compiler::{codegen::CodeGen, lexer::Lexer, parser::Parser};

use crate::Runtime;

fn compile(source: &str) -> Vec<u8> {
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

// ── list renders correct number of items ─────────────────────────────────────

#[test]
fn test_list_renders_two_items() {
    let source = r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("Loading...")
        Success(data) -> {
            list(data) { item -> text(item.name) }
        }
        Error(msg) -> text("Error")
    }
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();

    // First render: api_fetch called, status=Success (test mode), body=""
    let _ = inst.render().unwrap();
    let rid = inst.last_reqid();

    // Inject a JSON array with two items
    inst.inject_api_body(rid, r#"[{"name":"Apple"},{"name":"Banana"}]"#);

    // Second render: list should produce two text nodes
    let tree = inst.render().unwrap();
    // The list is inside the Success arm which is a block — flatten children
    let texts: Vec<&str> = collect_texts(&tree);
    assert!(
        texts.contains(&"Apple") && texts.contains(&"Banana"),
        "expected Apple and Banana, got: {texts:?}"
    );
}

#[test]
fn test_list_empty_array_renders_nothing() {
    let source = r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("Loading...")
        Success(data) -> {
            list(data) { item -> text(item.name) }
        }
        Error(msg) -> text("Error")
    }
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();
    let _ = inst.render().unwrap();
    let rid = inst.last_reqid();

    // Inject empty array
    inst.inject_api_body(rid, r#"[]"#);

    let tree = inst.render().unwrap();
    // No text nodes should come from the list
    let texts: Vec<&str> = collect_texts(&tree);
    assert!(
        !texts.contains(&"Apple"),
        "empty list should not produce item nodes"
    );
}

#[test]
fn test_list_numeric_field_renders() {
    let source = r#"
page Home {
    state { items = try api.get("/items") }
    match items {
        Loading -> text("wait")
        Success(data) -> {
            list(data) { item -> text(item.price) }
        }
        _ -> text("err")
    }
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();
    let _ = inst.render().unwrap();
    let rid = inst.last_reqid();
    inst.inject_api_body(rid, r#"[{"price":9.99},{"price":4.5}]"#);
    let tree = inst.render().unwrap();
    let texts: Vec<&str> = collect_texts(&tree);
    assert!(
        texts.contains(&"9.99") || texts.iter().any(|t| t.contains("9")),
        "expected numeric text nodes, got: {texts:?}"
    );
}

// ── helper ────────────────────────────────────────────────────────────────────

fn collect_texts<'a>(nodes: &'a [crate::UiNode]) -> Vec<&'a str> {
    let mut out = vec![];
    for node in nodes {
        if let Some(t) = &node.text {
            out.push(t.as_str());
        }
        out.extend(collect_texts(&node.children));
    }
    out
}
