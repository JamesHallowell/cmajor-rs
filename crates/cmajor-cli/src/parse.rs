use {
    crate::{diagnostics, io, stdlib},
    cmajor_lang::{Program, SourceFile, ast, lexer, parser},
    std::path::{Path, PathBuf},
};

pub fn run(path: &Path, stdlib_dir: Option<&Path>, verbose: bool) -> bool {
    match (path.is_dir(), stdlib_dir) {
        (true, Some(_)) => {
            eprintln!("--stdlib is not supported together with a directory argument");
            false
        }
        (true, None) => parse_dir(path, verbose),
        (false, Some(stdlib_dir)) => compile_with_stdlib(stdlib_dir, path),
        (false, None) => {
            let (success, output) = parse_file(path);
            print!("{output}");
            success
        }
    }
}

fn compile_with_stdlib(stdlib_dir: &Path, path: &Path) -> bool {
    let program: Program = stdlib::collect(stdlib_dir)
        .chain(std::iter::once(
            SourceFile::new(io::read_file(path)).with_name(path.display().to_string()),
        ))
        .collect();

    let resolution = cmajor_lang::resolver::resolve(&program);

    diagnostics::eprint(&program, &resolution.diagnostics);

    resolution.diagnostics.is_empty()
}

fn parse_file(path: &Path) -> (bool, String) {
    let source = io::read_file(path);
    let tokens = lexer::tokenize(&source);

    let (ast, diagnostics) = parser::parse(&source, &tokens);

    let mut output = format!("{} tokens, {} AST nodes\n\n", tokens.len(), ast.len());
    for &root in ast.roots() {
        output.push_str(&ast::dump(&ast, &tokens, &source, root));
    }
    for diagnostic in &diagnostics {
        output.push_str(&format!("{diagnostic}\n"));
    }

    (diagnostics.is_empty(), output)
}

fn parse_dir(dir: &Path, verbose: bool) -> bool {
    let mut files = Vec::new();
    collect_cmajor_files(dir, &mut files);
    files.sort();

    let total = files.len();
    let mut failed = 0;

    for file in &files {
        let (success, output) = parse_file(file);
        if success {
            println!("\x1b[32mPASS\x1b[0m  {}", file.display());
        } else {
            failed += 1;
            println!("\x1b[31mFAIL\x1b[0m  {}", file.display());
        }
        if verbose {
            for line in output.lines() {
                println!("      {line}");
            }
        }
    }

    println!();
    println!("{}/{total} files parsed cleanly", total - failed);

    failed == 0
}

fn collect_cmajor_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_cmajor_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "cmajor") {
            files.push(path);
        }
    }
}
