mod lexer;

use lexer::Lexer;

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
            Error(msg) -> text("Error: {msg}", color: #EF4444)
            Success(data) -> list(data) { p -> ProductCard(product: p) }
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
}
