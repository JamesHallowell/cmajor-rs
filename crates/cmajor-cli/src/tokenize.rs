use {
    crate::io,
    cmajor_lang::{SourceFile, lexer::tokenize},
    std::path::Path,
};

pub fn run(path: &Path) -> bool {
    let source = SourceFile::new(io::read_file(path));

    let token_stream = tokenize(source.contents());
    for (id, token) in token_stream.into_iter().ignore_trivia() {
        let span = token_stream.span(id);
        let text = &source[span];

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
