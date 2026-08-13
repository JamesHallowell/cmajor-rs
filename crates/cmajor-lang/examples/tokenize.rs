use cmajor_lang::lexer::tokenize;
use cmajor_lang::utils::source::Source;

fn main() {
    let path = std::env::args().nth(1).expect("Usage: tokenize <input>");
    let source = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        eprintln!("failed to read {path}: {err}");
        std::process::exit(1);
    });

    let token_stream = tokenize(&source);
    for (id, token) in token_stream.into_iter().ignore_trivia() {
        let span = token_stream.span(id);
        let source: Source<'_> = source.as_str().into();
        let text = &source[span];

        println!(
            "{:>4}..{:<4} {:<15} {:?}",
            span.start,
            span.end,
            format!("{:?}", token.kind),
            text
        );
    }
}
