use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::ast::{Item, Program};

// ── Public API ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LoadError {
    pub message: String,
    pub path: String,
}

/// Load a .vel file and recursively resolve all its imports.
/// Returns a merged Program with imported items appearing before the entry
/// file's items (so they are available during type-checking and codegen).
///
/// Diamond imports (multiple files importing the same dependency) are
/// deduplicated — the shared file's items appear only once.
/// Circular imports are rejected with a descriptive error.
pub fn load_program(entry: &Path) -> Result<Program, LoadError> {
    let mut in_stack = HashSet::new();
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    collect_items(entry, &mut in_stack, &mut seen, &mut items)?;
    Ok(Program { items })
}

// ── Internal ──────────────────────────────────────────────────────────────────

fn collect_items(
    path: &Path,
    in_stack: &mut HashSet<PathBuf>,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Item>,
) -> Result<(), LoadError> {
    let canonical = path.canonicalize().map_err(|e| LoadError {
        message: format!("cannot open '{}': {}", path.display(), e),
        path: path.to_string_lossy().into_owned(),
    })?;

    if in_stack.contains(&canonical) {
        return Err(LoadError {
            message: format!(
                "circular import detected: '{}' is already being loaded",
                canonical.display()
            ),
            path: canonical.to_string_lossy().into_owned(),
        });
    }
    if !seen.insert(canonical.clone()) {
        return Ok(()); // diamond import — already loaded, skip
    }

    let source = std::fs::read_to_string(&canonical).map_err(|e| LoadError {
        message: format!("cannot read '{}': {}", canonical.display(), e),
        path: canonical.to_string_lossy().into_owned(),
    })?;

    let tokens = Lexer::new(&source).tokenize();
    let program = Parser::new(tokens).parse().map_err(|e| LoadError {
        message: e.message,
        path: canonical.to_string_lossy().into_owned(),
    })?;

    let base_dir = canonical.parent().unwrap_or(Path::new("."));
    in_stack.insert(canonical.clone());

    let mut this_items = Vec::new();
    for item in program.items {
        match item {
            Item::Import(ref import) => {
                // Include all items from the imported file.
                // Named/default import syntax is checked by the type-checker in the future;
                // here we just ensure everything the dependency defines is available.
                // Diamond imports are deduplicated via `seen`, so items appear at most once.
                let import_path = resolve_path(base_dir, &import.path);
                collect_items(&import_path, in_stack, seen, out)?;
            }
            other => this_items.push(other),
        }
    }

    in_stack.remove(&canonical);
    out.extend(this_items);
    Ok(())
}

fn resolve_path(base: &Path, import_str: &str) -> PathBuf {
    let p = Path::new(import_str);
    let with_ext = if p.extension().is_some() {
        p.to_owned()
    } else {
        p.with_extension("vel")
    };
    base.join(with_ext)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(dir: &TempDir, name: &str, src: &str) -> PathBuf {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, src).unwrap();
        path
    }

    #[test]
    fn test_single_file_no_imports() {
        let dir = TempDir::new().unwrap();
        let entry = write(&dir, "main.vel", "page Home { text(\"hi\") }");
        let program = load_program(&entry).expect("load failed");
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_default_import_component() {
        let dir = TempDir::new().unwrap();
        write(&dir, "button.vel", "component Button() { text(\"click\") }");
        let entry = write(
            &dir,
            "main.vel",
            r#"import Button from "button"
page Home { Button() }"#,
        );
        let program = load_program(&entry).expect("load failed");
        // Button component (from import) + Home page
        assert_eq!(program.items.len(), 2);
        assert!(
            program
                .items
                .iter()
                .any(|i| matches!(i, Item::Component(c) if c.name == "Button"))
        );
        assert!(
            program
                .items
                .iter()
                .any(|i| matches!(i, Item::Page(p) if p.name == "Home"))
        );
    }

    #[test]
    fn test_named_import_type() {
        let dir = TempDir::new().unwrap();
        write(&dir, "types.vel", "type User { name: Text }");
        let entry = write(
            &dir,
            "main.vel",
            r#"import { User } from "types"
page Home { text("ok") }"#,
        );
        let program = load_program(&entry).expect("load failed");
        assert_eq!(program.items.len(), 2);
        assert!(matches!(&program.items[0], Item::TypeDef(t) if t.name == "User"));
    }

    #[test]
    fn test_diamond_import_deduplicated() {
        let dir = TempDir::new().unwrap();
        write(&dir, "types.vel", "type User { name: Text }");
        write(
            &dir,
            "a.vel",
            r#"import { User } from "types"
component A() { text("a") }"#,
        );
        write(
            &dir,
            "b.vel",
            r#"import { User } from "types"
component B() { text("b") }"#,
        );
        let entry = write(
            &dir,
            "main.vel",
            r#"import A from "a"
import B from "b"
page Home { A() }"#,
        );
        let program = load_program(&entry).expect("load failed");
        // User appears only once (diamond dedup), A, B, Home
        let type_count = program
            .items
            .iter()
            .filter(|i| matches!(i, Item::TypeDef(t) if t.name == "User"))
            .count();
        assert_eq!(type_count, 1, "User should appear exactly once");
    }

    #[test]
    fn test_circular_import_error() {
        let dir = TempDir::new().unwrap();
        write(
            &dir,
            "b.vel",
            r#"import A from "a.vel"
component B() { text("b") }"#,
        );
        let entry = write(
            &dir,
            "a.vel",
            r#"import B from "b.vel"
component A() { text("a") }"#,
        );
        let result = load_program(&entry);
        assert!(result.is_err(), "circular import should fail");
        assert!(result.unwrap_err().message.contains("circular"));
    }

    #[test]
    fn test_missing_file_error() {
        let dir = TempDir::new().unwrap();
        let entry = write(
            &dir,
            "main.vel",
            r#"import X from "nonexistent"
page Home { text("ok") }"#,
        );
        let result = load_program(&entry);
        assert!(result.is_err());
    }
}
