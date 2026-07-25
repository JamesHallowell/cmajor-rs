use {
    cmajor_lang::test_format::{self, Outcome},
    std::path::{Path, PathBuf},
};

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Usage: cmajtest <input.cmajtest | directory>");
    let path = PathBuf::from(path);

    let files = collect_test_files(&path).unwrap_or_else(|err| {
        eprintln!("failed to read {}: {err}", path.display());
        std::process::exit(1);
    });

    if files.is_empty() {
        eprintln!("no .cmajtest files found under {}", path.display());
        std::process::exit(1);
    }

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let multiple = files.len() > 1;

    for file_path in &files {
        let source = std::fs::read_to_string(file_path).unwrap_or_else(|err| {
            eprintln!("failed to read {}: {err}", file_path.display());
            std::process::exit(1);
        });

        let file = test_format::parse_test_file(&source);
        let results = test_format::run(&file);
        let location = file_path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or(file_path.to_string_lossy());

        for result in &results {
            match &result.outcome {
                Outcome::Pass => {
                    passed += 1;
                    println!(
                        "{GREEN}{BOLD}PASS{RESET}  {DIM}{location}:{RESET}{}:{}",
                        result.line, result.name
                    );
                    if let Some(expected) = &result.expected_error {
                        println!("      {DIM}expected: {expected}{RESET}");
                    }
                }
                Outcome::Fail(reason) => {
                    failed += 1;
                    println!(
                        "{RED}{BOLD}FAIL{RESET}  {DIM}{location}:{RESET}{}:{} {DIM}- {reason}{RESET}",
                        result.line, result.name
                    );
                    if let Some(expected) = &result.expected_error {
                        println!("      {DIM}expected: {expected}{RESET}");
                    }
                }
                Outcome::Skipped(reason) => {
                    skipped += 1;
                    println!(
                        "{YELLOW}SKIP{RESET}  {DIM}{location}:{RESET}{}:{} {DIM}- {reason}{RESET}",
                        result.line, result.name
                    );
                }
            }
        }
    }

    let total = passed + failed + skipped;
    if multiple {
        println!("\n{BOLD}{} files{RESET}", files.len());
    }
    println!(
        "{GREEN}{passed} passed{RESET}, {RED}{failed} failed{RESET}, {YELLOW}{skipped} skipped{RESET} ({total} total)",
    );

    if failed > 0 {
        std::process::exit(1);
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
            } else if entry_path.extension().is_some_and(|ext| ext == "cmajtest") {
                files.push(entry_path);
            }
        }
    }

    files.sort();
    Ok(files)
}
