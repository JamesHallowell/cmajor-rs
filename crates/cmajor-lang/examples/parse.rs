use {
    cmajor_lang::{ast, parser},
    std::{
        path::{Path, PathBuf},
        process::ExitCode,
    },
};

fn main() -> ExitCode {
    let mut path = None;
    let mut verbose = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--verbose" | "-v" => verbose = true,
            _ if path.is_none() => path = Some(PathBuf::from(arg)),
            _ => usage_error(),
        }
    }

    let Some(path) = path else { usage_error() };

    if path.is_dir() {
        if parse_dir(&path, verbose) {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    } else {
        let (success, output) = parse_file(&path);
        print!("{output}");
        if success {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

fn usage_error() -> ! {
    eprintln!("Usage: parse <file-or-directory> [--verbose]");
    std::process::exit(1);
}

fn parse_file(path: &Path) -> (bool, String) {
    let source = std::fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("failed to read {}: {err}", path.display());
        std::process::exit(1);
    });

    let parser::Parse {
        ast,
        tokens,
        diagnostics,
    } = parser::parse(&source);

    let mut output = format!("{} tokens, {} AST nodes\n\n", tokens.len(), ast.len());
    for &root in ast.roots() {
        output.push_str(&ast::dump(&ast, &tokens, &source, root));
    }
    for diagnostic in &diagnostics {
        output.push_str(&format!("error: {diagnostic}\n"));
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
