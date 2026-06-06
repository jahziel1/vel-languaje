use vel_compiler::checker::Checker;
use vel_compiler::codegen::CodeGen;
use vel_compiler::lexer::Lexer;
use vel_compiler::parser::Parser;

fn main() {
    let source = r#"
// A simple Vel page
page Store {
    state {
        products = try api.get("/products")
        search = ""
    }

    column(padding: 24, gap: 16) {
        text("Welcome to the store", size: 32, weight: 700)
        input(search, placeholder: "Search products...")

        match products {
            Loading -> spinner()
            Error(msg) -> text("Error")
            Success(data) -> text("ok")
        }
    }
}
"#;

    println!("Vel Compiler — Lexer\n");
    println!("Source:\n{}\n", source);
    println!("Tokens:");

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    for tok in &tokens {
        println!("  [{:3}:{:2}] {:?}", tok.span.line, tok.span.col, tok.token);
    }

    println!("\nTotal: {} tokens", tokens.len());

    // ── Parser ────────────────────────────────────────────────────────────────
    println!("\nParsing...\n");
    let tokens2 = Lexer::new(source).tokenize();
    let program = match Parser::new(tokens2).parse() {
        Ok(p) => {
            println!("AST: {:#?}", p);
            p
        }
        Err(e) => {
            println!(
                "Parse error at {}:{} — {}",
                e.span.line, e.span.col, e.message
            );
            return;
        }
    };

    // ── Type checker ──────────────────────────────────────────────────────────
    println!("\nType checking...\n");
    let mut checker = Checker::new();
    let errors = checker.check(&program);
    if errors.is_empty() {
        println!("Type check: OK");
    } else {
        for e in errors {
            println!(
                "  Type error at {}:{} — {}",
                e.span.line, e.span.col, e.message
            );
        }
        return;
    }

    // ── Code generator ────────────────────────────────────────────────────────
    println!("\nGenerating WASM...\n");
    let mut cg = CodeGen::new();
    let wasm = cg.generate(&program);
    println!(
        "WASM: {} bytes, {} globals, {} functions",
        wasm.len(),
        cg.globals.len(),
        cg.func_count(),
    );
}
