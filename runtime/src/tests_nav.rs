use vel_compiler::{codegen::CodeGen, lexer::Lexer, parser::Parser};

use crate::{Runtime, render_debug};

fn compile(source: &str) -> Vec<u8> {
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    CodeGen::new().generate(&program)
}

#[test]
fn test_nav_params_page_renders_with_param() {
    let source = r#"
page Home {
    button("Go") { onClick: go_product }
    fn go_product() { go("/product", id: 42) }
}

page Product(id: Number) {
    text("id={id}")
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt
        .instantiate(&wasm, "page_Product")
        .expect("instantiate failed");
    inst.nav_params = vec![7.0];
    let tree = inst.render().expect("render failed");
    let debug = render_debug(&tree);
    assert!(debug.contains("id=7"), "expected 'id=7' in: {debug}");
}

#[test]
fn test_nav_params_go_sets_params() {
    let source = r#"
page Home {
    button("Go") { onClick: go_product }
    fn go_product() { go("/product", id: 99) }
}

page Product(id: Number) {
    text("{id}")
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    let mut inst = rt
        .instantiate(&wasm, "page_Home")
        .expect("instantiate failed");
    inst.render().expect("render failed");
    inst.call_handler("Home_go_product")
        .expect("handler failed");
    let path = inst.take_navigation().expect("no navigation");
    inst.go(&path).expect("go failed");
    let tree = inst.render().expect("render failed");
    let debug = render_debug(&tree);
    assert!(debug.contains("99"), "expected '99' in: {debug}");
}

#[test]
fn test_nav_param_fn_in_page() {
    let source = r#"
page Products {
    state { products = try api.get("https://x.com/products") }
    fn goToProduct(id: Number) { go("/product", id: id) }
    match products {
        Loading       -> spinner()
        Success(data) -> text("ok")
        Error(msg)    -> text("err")
    }
}
"#;
    let wasm = compile(source);
    let rt = Runtime::new();
    match rt.instantiate(&wasm, "page_Products") {
        Ok(_) => {}
        Err(e) => panic!("instantiate failed: {e}"),
    }
}

#[test]
fn test_match_inside_column_valid_wasm() {
    // Regression: match bindings inside a UI block were not pre-scanned,
    // causing {msg} interpolation to emit I32Const(0) instead of f64 → WASM type error.
    let source = r#"
page P {
    state { result = try api.get("https://x.com") }
    column(padding: 16) {
        match result {
            Loading       -> spinner()
            Success(data) -> text("loaded")
            Error(msg)    -> text("Error: {msg}")
        }
    }
}
"#;
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    let wasm = CodeGen::new().generate(&program);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_P").expect("instantiate failed");
    let tree = inst.render().expect("render failed");
    assert!(!tree.is_empty());
}
