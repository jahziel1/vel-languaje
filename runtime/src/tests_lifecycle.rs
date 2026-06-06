use vel_compiler::{codegen::CodeGen, lexer::Lexer, parser::Parser};

use crate::Runtime;

fn first_text(nodes: &[crate::UiNode]) -> Option<&str> {
    nodes.iter().find_map(|n| {
        if n.tag_name == "text" {
            n.text.as_deref()
        } else {
            first_text(&n.children)
        }
    })
}

fn compile(source: &str) -> Vec<u8> {
    let tokens = Lexer::new(source).tokenize();
    let program = Parser::new(tokens).parse().expect("parse failed");
    let mut cg = CodeGen::new();
    cg.generate(&program)
}

// ── on mount ─────────────────────────────────────────────────────────────────

#[test]
fn test_on_mount_fires_after_first_render() {
    let src = r#"
page Home {
    state { count: Number = 0 }
    on mount { count = 1 }
    text("{count}")
}
"#;
    let wasm = compile(src);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();

    // First render: page fn sees count=0, then mount sets count=1.
    let tree1 = inst.render().unwrap();
    assert_eq!(
        tree1[0].text.as_deref(),
        Some("0"),
        "tree before mount fires"
    );

    // Second render: count=1 from mount mutation.
    let tree2 = inst.render().unwrap();
    assert_eq!(tree2[0].text.as_deref(), Some("1"), "tree after mount");
}

#[test]
fn test_on_mount_fires_only_once() {
    let src = r#"
page Home {
    state { count: Number = 0 }
    on mount { count = 1 }
    text("{count}")
}
"#;
    let wasm = compile(src);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();

    inst.render().unwrap(); // mount fires → count = 1
    let tree2 = inst.render().unwrap();
    let tree3 = inst.render().unwrap();

    assert_eq!(tree2[0].text.as_deref(), Some("1"));
    assert_eq!(
        tree3[0].text.as_deref(),
        Some("1"),
        "mount must not fire again"
    );
}

// ── on unmount ────────────────────────────────────────────────────────────────

#[test]
fn test_on_unmount_fires_on_navigate() {
    let src = r#"
page Home {
    state { count: Number = 0 }
    on unmount { count = 99 }
    text("{count}")
}
page Other { text("other") }
"#;
    let wasm = compile(src);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();

    inst.render().unwrap(); // initial render

    // Navigate away → unmount fires and sets count=99.
    inst.go("/other").unwrap();
    // Navigate back.
    inst.go("back").unwrap();

    // count=99 (set by unmount) is visible now.
    let tree = inst.render().unwrap();
    assert_eq!(tree[0].text.as_deref(), Some("99"));
}

#[test]
fn test_on_mount_fires_again_after_navigate_back() {
    let src = r#"
page Home {
    state { count: Number = 0 }
    on mount { count = 1 }
    text("{count}")
}
page Other { text("other") }
"#;
    let wasm = compile(src);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();

    inst.render().unwrap(); // first render: count=0, mount → count=1
    inst.render().unwrap(); // second render: count=1

    inst.go("/other").unwrap();
    inst.go("back").unwrap();

    // First render back on Home: page fn sees count=1, then mount fires → count=1 again.
    let tree1 = inst.render().unwrap();
    assert_eq!(
        tree1[0].text.as_deref(),
        Some("1"),
        "value before mount on re-mount"
    );

    // Second render: still 1.
    let tree2 = inst.render().unwrap();
    assert_eq!(tree2[0].text.as_deref(), Some("1"));
}

// ── responsive ────────────────────────────────────────────────────────────────

const RESPONSIVE_SRC: &str = r#"
page Home {
    on phone   { text("phone") }
    on tablet  { text("tablet") }
    on desktop { text("desktop") }
}
"#;

#[test]
fn test_responsive_phone_renders_at_small_width() {
    let wasm = compile(RESPONSIVE_SRC);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();
    inst.set_window_width(320.0);
    let tree = inst.render().unwrap();
    assert_eq!(first_text(&tree), Some("phone"));
}

#[test]
fn test_responsive_tablet_renders_at_medium_width() {
    let wasm = compile(RESPONSIVE_SRC);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();
    inst.set_window_width(800.0);
    let tree = inst.render().unwrap();
    assert_eq!(first_text(&tree), Some("tablet"));
}

#[test]
fn test_responsive_desktop_renders_at_large_width() {
    let wasm = compile(RESPONSIVE_SRC);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();
    inst.set_window_width(1280.0);
    let tree = inst.render().unwrap();
    assert_eq!(first_text(&tree), Some("desktop"));
}

#[test]
fn test_responsive_changes_on_resize() {
    let wasm = compile(RESPONSIVE_SRC);
    let rt = Runtime::new();
    let mut inst = rt.instantiate(&wasm, "page_Home").unwrap();

    inst.set_window_width(320.0);
    let tree1 = inst.render().unwrap();
    assert_eq!(first_text(&tree1), Some("phone"));

    inst.set_window_width(1280.0);
    let tree2 = inst.render().unwrap();
    assert_eq!(first_text(&tree2), Some("desktop"));
}
