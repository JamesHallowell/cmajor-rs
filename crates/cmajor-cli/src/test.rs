use {
    crate::{io, stdlib},
    cmajor_lang::{
        Diagnostic,
        ast::Ast,
        lexer::{self, TokenStream},
        parser,
        resolver::Unit,
        test_format::{self, Directive, Outcome, Section, TestFile, TestResult},
    },
    libtest_mimic::{Arguments, Completion, Failed, Trial},
    std::path::{Path, PathBuf},
};

pub fn run(path: &Path, stdlib_dir: Option<&Path>) -> bool {
    let files = collect_test_files(path).unwrap_or_else(|err| {
        eprintln!("failed to read {}: {err}", path.display());
        std::process::exit(1);
    });

    if files.is_empty() {
        eprintln!("no .cmajtest files found under {}", path.display());
        std::process::exit(1);
    }

    let stdlib_files = stdlib_dir
        .map(stdlib::collect)
        .map(|files| files.collect::<Vec<_>>())
        .unwrap_or_default();
    let stdlib_parses: Vec<(TokenStream, Ast, Vec<Diagnostic>)> = stdlib_files
        .iter()
        .map(|file| {
            let tokens = lexer::tokenize(&file.source);
            let (ast, diagnostics) = parser::parse(&file.source, &tokens);
            (tokens, ast, diagnostics)
        })
        .collect();
    let stdlib_units: Vec<Unit<'_>> = stdlib_files
        .iter()
        .zip(&stdlib_parses)
        .map(|(file, (tokens, ast, _))| Unit {
            name: &file.name,
            source: &file.source,
            tokens,
            ast,
        })
        .collect();
    let standard_library = cmajor_lang::resolver::declare_stdlib(&stdlib_units);

    let mut trials = Vec::new();

    for file_path in &files {
        let source = io::read_file(file_path);

        let is_plain_cmajor = file_path.extension().is_some_and(|ext| ext == "cmajor");
        let file = if is_plain_cmajor {
            TestFile {
                prelude: String::new(),
                sections: vec![Section {
                    directive: Directive::TestCompile,
                    disabled: false,
                    body: source,
                    line: 1,
                }],
            }
        } else {
            test_format::parse_test_file(&source)
        };

        let location = file_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| file_path.to_string_lossy().into_owned());

        for result in test_format::run(&file, &standard_library) {
            let name = format!("{location}:{}:{}", result.line, result.name);
            trials.push(Trial::ignorable_test(name, move || outcome_of(result)));
        }
    }

    let args = Arguments {
        test_threads: Some(1),
        ..Arguments::default()
    };

    !libtest_mimic::run(&args, trials).has_failed()
}

fn outcome_of(result: TestResult) -> Result<Completion, Failed> {
    match result.outcome {
        Outcome::Pass => {
            if let Some(expected) = &result.expected_error {
                println!("  expected: {expected}");
                println!(
                    "  actual:   {}",
                    result.actual_error.as_deref().unwrap_or("<none>")
                );
            }
            Ok(Completion::Completed)
        }
        Outcome::Fail(mut reason) => {
            if let Some(expected) = &result.expected_error {
                reason.push_str(&format!("\n  expected: {expected}"));
                reason.push_str(&format!(
                    "\n  actual:   {}",
                    result.actual_error.as_deref().unwrap_or("<none>")
                ));
            }
            Err(Failed::from(reason))
        }
        Outcome::Skipped(reason) => Ok(Completion::ignored_with(reason)),
    }
}

fn collect_test_files(path: &Path) -> std::io::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    let mut files = Vec::new();
    let mut dirs = vec![path.to_path_buf()];

    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                dirs.push(entry_path);
            } else if entry_path
                .extension()
                .is_some_and(|ext| ext == "cmajtest" || ext == "cmajor")
            {
                files.push(entry_path);
            }
        }
    }

    files.sort();
    Ok(files)
}
