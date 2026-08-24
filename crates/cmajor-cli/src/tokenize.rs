use {
    crate::io,
    cmajor_lang::{lexer::tokenize, utils::source::Source},
    std::path::Path,
};

pub fn run(path: &Path) -> bool {
    let source = io::read_file(path);

    let token_stream = tokenize(&source);
    for (id, token) in token_stream.into_iter().ignore_trivia() {
        let span = token_stream.span(id);
        let source_ref: Source<'_> = source.as_str().into();
        let text = &source_ref[span];

        println!(
            "{:>4}..{:<4} {:<15} {:?}",
            span.start,
            span.end,
            format!("{:?}", token.kind),
            text
        );
    }

    true
}
