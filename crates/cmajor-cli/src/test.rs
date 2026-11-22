use {
    crate::{diagnostics, io, stdlib},
    cmajor_lang::{
        Program,
        test_format::{self, Directive, FileScore, Outcome, Section, TestFile, TestResult},
        utils::source::Line,
    },
    libtest_mimic::{Arguments, Completion, Failed, Trial},
    std::path::{Path, PathBuf},
};

const NOT_YET_IMPLEMENTED: &str = "cmaj_test_not_yet_implemented.cmajtest";

pub fn run(path: &Path, stdlib_dir: Option<&Path>) -> bool {
    let files = collect_test_files(path).unwrap_or_else(|err| {
        eprintln!("failed to read {}: {err}", path.display());
        std::process::exit(1);
    });

    if files.is_empty() {
        eprintln!("no .cmajtest files found under {}", path.display());
        std::process::exit(1);
    }

    let standard_library = stdlib_dir.map(|dir| {
        let program: Program = stdlib::collect(dir).collect();
        let library = cmajor_lang::resolver::declare_stdlib(&program);
        diagnostics::eprint(&program, library.diagnostics());
        library
    });

    let mut trials = Vec::new();
    let mut scores = Vec::new();

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
                    line: Line::from(1),
                }],
            }
        } else {
            test_format::parse_test_file(&source)
        };

        let location = location_of(file_path, path);

        let directory = file_path.parent().unwrap_or(Path::new(""));
        let global = file.global_source(|name| io::read_file_or_directory(&directory.join(name)));
        let results = test_format::run(&file, &global, standard_library.as_ref());
        let score: FileScore = results.iter().collect();

        for result in results {
            let name = format!("{location}:{}:{}", result.line, result.directive);
            trials.push(Trial::ignorable_test(name, move || outcome_of(result)));
        }

        scores.push((location, score));
    }

    let args = Arguments {
        test_threads: Some(1),
        ..Arguments::default()
    };

    let conclusion = libtest_mimic::run(&args, trials);
    print_scoreboard(&scores);
    !conclusion.has_failed()
}

fn print_scoreboard(scores: &[(String, FileScore)]) {
    const VALID_PROGRAMS: &str = "valid programs accepted";
    const EXPECTED_ERRORS: &str = "expected errors matched";
    const TOTAL: &str = "total";

    let total: FileScore = scores.iter().map(|(_, score)| *score).sum();
    let name_width = scores
        .iter()
        .map(|(location, _)| location.len())
        .chain([TOTAL.len()])
        .max()
        .unwrap_or_default();

    println!();
    println!(
        "{:name_width$}  {VALID_PROGRAMS:>25}  {EXPECTED_ERRORS:>25}",
        "file"
    );
    for (location, score) in scores
        .iter()
        .map(|(location, score)| (location.as_str(), score))
        .chain([(TOTAL, &total)])
    {
        println!(
            "{location:name_width$}  {:>25}  {:>25}",
            score.valid_programs, score.expected_errors
        );
    }
}

fn location_of(file_path: &Path, root: &Path) -> String {
    file_path
        .strip_prefix(root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .or_else(|| file_path.file_name().map(Path::new))
        .unwrap_or(file_path)
        .display()
        .to_string()
}

fn outcome_of(result: TestResult) -> Result<Completion, Failed> {
    match result.outcome {
        Outcome::Pass => Ok(Completion::Completed),
        Outcome::Fail(failure) => Err(Failed::from(failure)),
        Outcome::Skipped(skip) => Ok(Completion::ignored_with(skip.to_string())),
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
                .file_name()
                .is_some_and(|name| name == NOT_YET_IMPLEMENTED)
            {
                continue;
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
