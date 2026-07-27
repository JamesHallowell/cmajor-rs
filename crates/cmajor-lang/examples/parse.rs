use cmajor_lang::{ast, parser};

fn main() {
    let path = std::env::args().nth(1).expect("Usage: parse <input>");
    let source = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        eprintln!("failed to read {path}: {err}");
        std::process::exit(1);
    });

    let parser::Parse {
        ast,
        roots,
        tokens,
        diagnostics,
    } = parser::parse(&source);

    println!("{} tokens, {} AST nodes\n", tokens.len(), ast.len());
    for root in roots {
        print!("{}", ast::dump(&ast, &tokens, &source, root));
    }

    for diagnostic in &diagnostics {
        println!("error: {diagnostic}");
    }
}
