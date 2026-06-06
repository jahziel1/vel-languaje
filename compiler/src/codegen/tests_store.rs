use crate::codegen::CodeGen;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn compile(src: &str) -> Vec<u8> {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

fn export_names(src: &str) -> Vec<String> {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut cg = CodeGen::new();
    cg.generate(&program);
    cg.func_export_names()
        .into_iter()
        .flatten()
        .map(|s| s.to_owned())
        .collect()
}

fn compile_gen(src: &str) -> CodeGen {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
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

// ── store state globals ───────────────────────────────────────────────────────

#[test]
fn store_state_creates_globals() {
    let src = r#"
store counter {
    state { count: Number = 0 }
}
page Home {
    text("hi")
}
"#;
    let cg = compile_gen(src);
    assert!(
        cg.global_map.contains_key("counter/count"),
        "counter/count global missing"
    );
}

// ── store derived collected ───────────────────────────────────────────────────

#[test]
fn store_derived_collected() {
    let src = r#"
store counter {
    state { count: Number = 0 }
    derived { doubled = count * 2 }
}
page Home {
    text("hi")
}
"#;
    let cg = compile_gen(src);
    let derived = cg.store_derived.get("counter");
    assert!(derived.is_some(), "store_derived missing 'counter'");
    let names: Vec<&str> = derived.unwrap().iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        names.contains(&"doubled"),
        "derived 'doubled' missing: {:?}",
        names
    );
}

// ── store derived accessed from page compiles ─────────────────────────────────

#[test]
fn store_derived_compiles() {
    let src = r#"
store counter {
    state { count: Number = 0 }
    derived { doubled = count * 2 }
}
page Home {
    text("{counter.doubled}")
}
"#;
    validate_wasm(&compile(src));
}

// ── store fn exported ─────────────────────────────────────────────────────────

#[test]
fn store_fn_is_exported() {
    let src = r#"
store counter {
    state { count: Number = 0 }
    fn increment() { count = count + 1 }
}
page Home {
    text("hi")
}
"#;
    let names = export_names(src);
    assert!(
        names.contains(&"store_counter_increment".to_owned()),
        "store_counter_increment not exported: {:?}",
        names
    );
}

// ── page reads store state ────────────────────────────────────────────────────

#[test]
fn page_reads_store_state_compiles() {
    let src = r#"
store counter {
    state { count: Number = 0 }
    fn increment() { count = count + 1 }
}
page Home {
    text("{counter.count}")
    fn doInc() { counter.increment() }
}
"#;
    let names = export_names(src);
    assert!(names.contains(&"page_Home".to_owned()));
    assert!(names.contains(&"Home_doInc".to_owned()));
    assert!(names.contains(&"store_counter_increment".to_owned()));
    validate_wasm(&compile(src));
}

// ── store with bool derived used in if ───────────────────────────────────────

#[test]
fn store_bool_derived_in_if() {
    let src = r#"
store auth {
    state { loggedIn: Bool = false }
    derived { isLoggedIn = loggedIn }
    fn logout() { loggedIn = false }
}
page Home {
    if auth.isLoggedIn {
        text("hello")
    }
    fn doLogout() { auth.logout() }
}
"#;
    validate_wasm(&compile(src));
}

// ── type checker: store names in scope ───────────────────────────────────────

#[test]
fn checker_allows_store_access() {
    use crate::checker::Checker;

    let src = r#"
store auth {
    state { loggedIn: Bool = false }
}
page Home {
    if auth.loggedIn {
        text("ok")
    }
}
"#;
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut checker = Checker::new();
    let errors = checker.check(&program);
    assert!(errors.is_empty(), "unexpected type errors: {:?}", errors);
}

// ── store api.headers — static value ─────────────────────────────────────────

#[test]
fn store_api_headers_static_applies_to_api_call() {
    let src = r#"
export store auth {
    state { loggedIn: Bool = false }
    api.headers { Authorization: "Bearer statictoken" }
}
page Home {
    state { me = try api.get("/me") }
    match me {
        Loading       -> text("...")
        Error(msg)    -> text("err")
        Success(data) -> text("ok")
    }
}
"#;
    validate_wasm(&compile(src));
    let cg = compile_gen(src);
    let data = String::from_utf8(cg.string_bytes.clone()).unwrap();
    assert!(
        data.contains("Authorization"),
        "Authorization key not interned"
    );
    assert!(
        data.contains("Bearer statictoken"),
        "header value not interned"
    );
}

// ── store api.headers — dynamic text state value ──────────────────────────────

#[test]
fn store_api_headers_dynamic_text_state_valid_wasm() {
    let src = r#"
export store auth {
    state { token: Text = "" }
    api.headers { Authorization: "Bearer {token}" }
    fn setToken(t: Number) { token = "admin" }
}
page Home {
    state { me = try api.get("/me") }
    match me {
        Loading       -> text("...")
        Error(msg)    -> text("err")
        Success(data) -> text("ok")
    }
}
"#;
    validate_wasm(&compile(src));
}

// ── store api.headers — collected in codegen ─────────────────────────────────

#[test]
fn store_api_headers_collected() {
    let src = r#"
export store auth {
    state { loggedIn: Bool = false }
    api.headers { Authorization: "Bearer abc" }
}
page Home {
    text("hi")
}
"#;
    let cg = compile_gen(src);
    let headers = cg.store_api_headers.get("auth");
    assert!(headers.is_some(), "store_api_headers missing 'auth'");
    let keys: Vec<&str> = headers.unwrap().iter().map(|(k, _)| k.as_str()).collect();
    assert!(
        keys.contains(&"Authorization"),
        "Authorization header missing: {:?}",
        keys
    );
}
