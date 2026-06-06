use crate::codegen::CodeGen;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn compile_gen(src: &str) -> CodeGen {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut cg = CodeGen::new();
    cg.generate(&program);
    cg
}

fn compile(src: &str) -> Vec<u8> {
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

fn export_names(src: &str) -> Vec<String> {
    compile_gen(src)
        .func_export_names()
        .into_iter()
        .flatten()
        .map(|s| s.to_owned())
        .collect()
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

// ── Parser: persist flag set ──────────────────────────────────────────────────

#[test]
fn persist_flag_parsed() {
    use crate::parser::ast::{Item, Stmt};

    let src = r#"
store auth {
    state { token: Text = "", persist }
}
page Home { text("hi") }
"#;
    let tokens = Lexer::new(src).tokenize();
    let program = Parser::new(tokens).parse().expect("parse error");
    let store = program.items.iter().find_map(|i| {
        if let Item::Store(s) = i {
            Some(s)
        } else {
            None
        }
    });
    assert!(store.is_some(), "store not found");
    let state = store.unwrap().state.as_ref().expect("state block missing");
    let entry = state.entries.first().expect("no state entries");
    assert!(entry.persist, "persist flag not set on 'token'");
}

// ── Persist Number state tracked in persist_state ────────────────────────────

#[test]
fn persist_num_state_tracked() {
    let src = r#"
store settings {
    state { volume: Number = 80, persist }
}
page Home { text("hi") }
"#;
    let cg = compile_gen(src);
    assert!(
        cg.persist_state.contains("settings/volume"),
        "persist_state missing 'settings/volume'"
    );
}

// ── Persist Bool state tracked ────────────────────────────────────────────────

#[test]
fn persist_bool_state_tracked() {
    let src = r#"
store prefs {
    state { darkMode: Bool = false, persist }
}
page Home { text("hi") }
"#;
    let cg = compile_gen(src);
    assert!(
        cg.persist_state.contains("prefs/darkMode"),
        "persist_state missing 'prefs/darkMode'"
    );
}

// ── Non-persist state NOT in persist_state ────────────────────────────────────

#[test]
fn non_persist_state_not_tracked() {
    let src = r#"
store counter {
    state { count: Number = 0 }
}
page Home { text("hi") }
"#;
    let cg = compile_gen(src);
    assert!(
        !cg.persist_state.contains("counter/count"),
        "counter/count should NOT be in persist_state"
    );
}

// ── vel_persist_init exported when persist state exists ───────────────────────

#[test]
fn vel_persist_init_exported() {
    let src = r#"
store settings {
    state { volume: Number = 80, persist }
}
page Home { text("hi") }
"#;
    let names = export_names(src);
    assert!(
        names.contains(&"vel_persist_init".to_owned()),
        "vel_persist_init not exported: {:?}",
        names
    );
}

// ── vel_persist_init NOT exported when no persist state ──────────────────────

#[test]
fn vel_persist_init_not_exported_when_no_persist() {
    let src = r#"
store counter {
    state { count: Number = 0 }
}
page Home { text("hi") }
"#;
    let names = export_names(src);
    assert!(
        !names.contains(&"vel_persist_init".to_owned()),
        "vel_persist_init should not be exported when no persist: {:?}",
        names
    );
}

// ── WASM with persist is valid ────────────────────────────────────────────────

#[test]
fn persist_store_compiles_valid_wasm() {
    let src = r#"
store settings {
    state {
        volume: Number = 80, persist
        darkMode: Bool = false, persist
    }
    fn setVolume(v: Number) { volume = v }
    fn toggleDark() { darkMode = not darkMode }
}
page Home {
    text("{settings.volume}")
}
"#;
    validate_wasm(&compile(src));
}

// ── Persist + non-persist mix compiles ───────────────────────────────────────

#[test]
fn persist_and_normal_state_compile() {
    let src = r#"
store auth {
    state {
        count: Number = 0
        loggedIn: Bool = false, persist
    }
    fn login() { loggedIn = true }
}
page Home {
    if auth.loggedIn { text("ok") }
    fn doLogin() { auth.login() }
}
"#;
    validate_wasm(&compile(src));
    let names = export_names(src);
    assert!(names.contains(&"vel_persist_init".to_owned()));
}
