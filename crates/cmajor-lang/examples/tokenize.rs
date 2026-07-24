use cmajor_lang::lexer::tokenize;

fn main() {
    let path = std::env::args().nth(1).expect("Usage: tokenize <input>");
    let source = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        eprintln!("failed to read {path}: {err}");
        std::process::exit(1);
    });

    let mut pos = 0usize;

    for token in tokenize(&source) {
        let start = pos;
        pos += token.len as usize;
        let text = &source[start..pos];

        if token.kind.is_trivia() {
            continue;
        }

        println!(
            "{:>4}..{:<4} {:<15} {:?}",
            start,
            pos,
            format!("{:?}", token.kind),
            text
        );
    }
}
