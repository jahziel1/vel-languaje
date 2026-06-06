use std::{env, fs, path::Path, process, sync::mpsc};

use notify::{EventKind, RecursiveMode, Watcher, recommended_watcher};
use vel_compiler::{
    checker::Checker, checker::TypeError, codegen::CodeGen, loader, parser::ast::Item,
};
use vel_runtime::{PageInstance, Runtime, run_window, run_window_error, run_window_watch};

mod web;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("run") => cmd_run(args.get(2)),
        Some("watch") => cmd_watch(args.get(2)),
        Some("build") => match args.get(2).map(String::as_str) {
            Some("--web") => web::cmd_build_web(args.get(3)),
            _ => cmd_build(args.get(2)),
        },
        Some("check") => cmd_check(args.get(2)),
        Some("new") => cmd_new(args.get(2)),
        Some("serve") => web::cmd_serve(args.get(2)),
        _ => {
            eprintln!("Usage:");
            eprintln!("  vel run <file.vel>        — execute in a native window");
            eprintln!("  vel watch <file.vel>       — run with hot reload");
            eprintln!("  vel build <file.vel>       — compile to .wasm");
            eprintln!("  vel build --web <file.vel> — compile to dist/ for browser");
            eprintln!("  vel serve <file.vel>       — dev server with hot reload");
            eprintln!("  vel check <file.vel>       — type-check without running");
            eprintln!("  vel new <name>             — scaffold a new project");
            process::exit(1);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_source(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("error: cannot read '{path}': {e}");
        process::exit(1);
    })
}

fn format_span_error(source: &str, path: &str, line: usize, col: usize, message: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = format!("error: {message}\n  --> {path}:{line}:{col}\n");
    if line > 0 && line <= lines.len() {
        let src_line = lines[line - 1];
        out.push_str(&format!("{line:>4} | {src_line}\n"));
        let ptr = " ".repeat(col.saturating_sub(1));
        out.push_str(&format!("     | {ptr}^\n"));
    }
    out
}

fn format_type_error(source: &str, path: &str, e: &TypeError) -> String {
    let mut s = format_span_error(source, path, e.span.line, e.span.col, &e.message);
    if let Some(help) = &e.help {
        s.push_str(&format!("help: {}\n", help));
    }
    s
}

pub(crate) struct Compiled {
    pub(crate) wasm: Vec<u8>,
    pub(crate) first_page: String,
}

pub(crate) fn compile_source(path: &str) -> Compiled {
    let source = read_source(path); // kept for error span display
    let entry = Path::new(path);
    let program = loader::load_program(entry).unwrap_or_else(|e| {
        eprintln!("error: {}", e.message);
        process::exit(1);
    });

    let mut checker = Checker::new();
    let errors = checker.check(&program);
    if !errors.is_empty() {
        for e in errors {
            eprint!("{}", format_type_error(&source, path, e));
        }
        process::exit(1);
    }

    let first_page = program
        .items
        .iter()
        .find_map(|item| {
            if let Item::Page(p) = item {
                Some(p.name.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            eprintln!("error: no page defined in '{path}'");
            process::exit(1);
        });

    let wasm = CodeGen::new().generate(&program);
    Compiled { wasm, first_page }
}

/// Compile only — returns Err(formatted_error_text) instead of exiting.
pub(crate) fn try_compile(path: &str) -> Result<Compiled, String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("error: cannot read '{path}': {e}"))?;
    let entry = Path::new(path);
    let program = loader::load_program(entry).map_err(|e| format!("error: {}", e.message))?;

    let mut checker = Checker::new();
    let errors = checker.check(&program);
    if !errors.is_empty() {
        let text = errors
            .iter()
            .map(|e| format_type_error(&source, path, e))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(text);
    }

    let first_page = program
        .items
        .iter()
        .find_map(|item| {
            if let Item::Page(p) = item {
                Some(p.name.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| format!("error: no page defined in '{path}'"))?;

    let wasm = CodeGen::new().generate(&program);
    Ok(Compiled { wasm, first_page })
}

/// Compile and instantiate without exiting — returns None on any error.
fn try_instantiate(path: &str) -> Option<PageInstance> {
    let source = fs::read_to_string(path).ok()?;
    let entry = Path::new(path);
    let program = match loader::load_program(entry) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e.message);
            return None;
        }
    };

    let mut checker = Checker::new();
    let errors = checker.check(&program);
    if !errors.is_empty() {
        for e in errors {
            eprint!("{}", format_type_error(&source, path, e));
        }
        return None;
    }

    let page_name = program.items.iter().find_map(|item| {
        if let Item::Page(p) = item {
            Some(p.name.clone())
        } else {
            None
        }
    })?;

    let wasm = CodeGen::new().generate(&program);
    match Runtime::new().instantiate(&wasm, &format!("page_{page_name}")) {
        Ok(inst) => Some(inst),
        Err(e) => {
            eprintln!("runtime error: {e}");
            None
        }
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

fn cmd_check(path_arg: Option<&String>) {
    let path = path_arg.unwrap_or_else(|| {
        eprintln!("error: missing file\nUsage: vel check <file.vel>");
        process::exit(1);
    });
    let source = read_source(path);
    let entry = Path::new(path);
    let program = loader::load_program(entry).unwrap_or_else(|e| {
        eprintln!("error: {}", e.message);
        process::exit(1);
    });

    let mut checker = Checker::new();
    let errors = checker.check(&program);
    if errors.is_empty() {
        println!("{path}: no errors");
    } else {
        for e in errors {
            eprint!("{}", format_type_error(&source, path, e));
        }
        process::exit(1);
    }
}

fn cmd_run(path_arg: Option<&String>) {
    let path = path_arg.unwrap_or_else(|| {
        eprintln!("error: missing file\nUsage: vel run <file.vel>");
        process::exit(1);
    });

    let compiled = match try_compile(path) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}");
            run_window_error(msg, format!("Vel — {path}"));
            return;
        }
    };

    let instance = match Runtime::new()
        .instantiate(&compiled.wasm, &format!("page_{}", compiled.first_page))
    {
        Ok(i) => i,
        Err(e) => {
            let msg = format!("error: runtime error\n{e}");
            eprintln!("{msg}");
            run_window_error(msg, format!("Vel — {path}"));
            return;
        }
    };

    run_window(instance, &format!("Vel — {}", compiled.first_page));
}

fn cmd_watch(path_arg: Option<&String>) {
    let path = path_arg.unwrap_or_else(|| {
        eprintln!("error: missing file\nUsage: vel watch <file.vel>");
        process::exit(1);
    });

    // Initial compile — show error window if it fails
    let compiled = match try_compile(path) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}");
            run_window_error(msg, format!("Vel — {path}"));
            return;
        }
    };
    let instance = Runtime::new()
        .instantiate(&compiled.wasm, &format!("page_{}", compiled.first_page))
        .unwrap_or_else(|e| {
            eprintln!("runtime error: {e}");
            process::exit(1);
        });

    let (tx, rx) = mpsc::channel::<Result<PageInstance, String>>();
    let watch_path = path.to_owned();
    let abs_path = fs::canonicalize(path).unwrap_or_else(|_| Path::new(path).to_path_buf());

    let mut watcher = recommended_watcher(move |res: notify::Result<notify::Event>| {
        let Ok(event) = res else { return };
        if !matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
            return;
        }
        match try_instantiate(&watch_path) {
            Some(inst) => {
                eprintln!("[vel] reloaded");
                let _ = tx.send(Ok(inst));
            }
            None => {
                // try_instantiate already printed the error; send a generic overlay message
                let _ = tx.send(Err(
                    "Compilation failed — check the terminal for details.".to_owned()
                ));
            }
        }
    })
    .unwrap_or_else(|e| {
        eprintln!("error: cannot start file watcher: {e}");
        process::exit(1);
    });

    watcher
        .watch(&abs_path, RecursiveMode::NonRecursive)
        .unwrap_or_else(|e| {
            eprintln!("error: cannot watch '{path}': {e}");
            process::exit(1);
        });

    run_window_watch(instance, &format!("Vel — {}", compiled.first_page), rx);
}

fn cmd_build(path_arg: Option<&String>) {
    let path = path_arg.unwrap_or_else(|| {
        eprintln!("error: missing file\nUsage: vel build <file.vel>");
        process::exit(1);
    });
    let compiled = compile_source(path);

    let out = Path::new(path).with_extension("wasm");
    let out_str = out.display().to_string();
    fs::write(&out, &compiled.wasm).unwrap_or_else(|e| {
        eprintln!("error: cannot write '{out_str}': {e}");
        process::exit(1);
    });
    println!(
        "compiled {path} → {out_str} ({} bytes)",
        compiled.wasm.len()
    );
}

fn cmd_new(name_arg: Option<&String>) {
    let name = name_arg.unwrap_or_else(|| {
        eprintln!("error: missing project name\nUsage: vel new <name>");
        process::exit(1);
    });

    let dir = Path::new(name);
    if dir.exists() {
        eprintln!("error: '{name}' already exists");
        process::exit(1);
    }

    fs::create_dir_all(dir).unwrap_or_else(|e| {
        eprintln!("error: cannot create directory '{name}': {e}");
        process::exit(1);
    });

    let main_vel = format!(
        "page Home {{\n\
         \tstate {{ count = 0 }}\n\
         \tfn increment() {{ count = count + 1 }}\n\
         \tcolumn(gap: 16) {{\n\
         \t\ttext(\"{name}\") {{\n\
         \t\t\tfontSize: 24\n\
         \t\t\tfontWeight: bold\n\
         \t\t}}\n\
         \t\tButton(\"Click me\", onClick: increment) {{}}\n\
         \t}}\n\
         }}\n"
    );

    fs::write(dir.join("main.vel"), &main_vel).unwrap_or_else(|e| {
        eprintln!("error: cannot write main.vel: {e}");
        process::exit(1);
    });

    println!("created {name}/");
    println!("  main.vel");
    println!();
    println!("next: vel watch {name}/main.vel");
}
