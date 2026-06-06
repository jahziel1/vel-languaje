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
    wasmparser::Validator::new_with_features(wasmparser::WasmFeatures::default())
        .validate_all(wasm)
        .expect("invalid wasm");
}

#[test]
fn test_page_with_params_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    button("Go to product") { onClick: goToProduct }
    fn goToProduct() { go("/product", id: 42) }
}

page Product(id: Number) {
    text("Product {id}")
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_print_static_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    fn submit() {
        print("hello from Vel")
    }
    Button("Go", onClick: submit) {}
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_print_interpolated_valid_wasm() {
    let wasm = compile(
        r#"
page Home {
    state { count = 0 }
    fn inc() {
        count = count + 1
        print("count is now: {count}")
    }
    Button("Inc", onClick: inc) {}
}
"#,
    );
    validate_wasm(&wasm);
}

#[test]
fn test_page_with_params_exports() {
    let cg = compile_gen(
        r#"
page Product(id: Number) {
    text("Product {id}")
}
"#,
    );
    let names: Vec<_> = cg.func_export_names().into_iter().flatten().collect();
    assert!(
        names.contains(&"page_Product"),
        "missing page_Product export"
    );
}
