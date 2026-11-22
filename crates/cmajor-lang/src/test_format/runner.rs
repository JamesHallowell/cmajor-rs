use {
    super::{Directive, TestFile},
    crate::{
        Program, SourceFile,
        resolver::{self, ResolvedProgram, StandardLibrary},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail(String),
    Skipped(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResult {
    pub name: String,
    pub line: usize,
    pub outcome: Outcome,
    pub expected_error: Option<String>,
    pub actual_error: Option<String>,
}

pub fn run(file: &TestFile, baseline: Option<&StandardLibrary>) -> Vec<TestResult> {
    let global = file.global_source();

    file.sections
        .iter()
        .filter(|section| section.directive != Directive::Global)
        .map(|section| {
            let name = section.directive.to_string();
            let expected_error = match &section.directive {
                Directive::ExpectError(expected) => Some(expected.message.clone()),
                _ => None,
            };

            let mut actual_error = None;

            let outcome = match (
                section.disabled,
                &section.directive,
                section.source(&global),
            ) {
                (true, ..) => Outcome::Skipped("disabled".to_string()),
                (false, Directive::TestCompile, Some(source)) => {
                    let errors = compile(source, baseline);
                    if errors.is_empty() {
                        Outcome::Pass
                    } else {
                        let detail = errors
                            .iter()
                            .map(|error| format!("  {error}"))
                            .collect::<Vec<_>>()
                            .join("\n");
                        Outcome::Fail(format!("compile error produced\n{detail}"))
                    }
                }
                (false, Directive::ExpectError(_), Some(source)) => {
                    let errors = compile(source, baseline);
                    if errors.is_empty() {
                        Outcome::Fail("expected an error but none occurred".to_string())
                    } else {
                        actual_error = Some(errors.join("\n"));
                        Outcome::Pass
                    }
                }
                _ => Outcome::Skipped(format!("execution of '{name}' not yet supported")),
            };

            TestResult {
                name,
                line: section.line,
                outcome,
                expected_error,
                actual_error,
            }
        })
        .collect()
}

const REJECTED_WITHOUT_A_DIAGNOSTIC: &str = "error: rejected without a diagnostic";

fn compile(source: String, baseline: Option<&StandardLibrary>) -> Vec<String> {
    let program = Program::new().with_source_file(SourceFile::new(source));

    let parse_errors: Vec<_> = program
        .diagnostics()
        .map(|(unit, diagnostic)| diagnostic.in_unit(unit, &program).to_string())
        .collect();

    if !parse_errors.is_empty() {
        parse_errors
    } else if program.error_tokens().next().is_some() {
        vec![REJECTED_WITHOUT_A_DIAGNOSTIC.to_string()]
    } else {
        resolution_errors(program, baseline)
    }
}

fn resolution_errors(program: Program, baseline: Option<&StandardLibrary>) -> Vec<String> {
    let ResolvedProgram {
        program,
        resolution,
    } = match baseline {
        Some(baseline) => resolver::resolve_with_standard_library(program, baseline),
        None => {
            let resolution = resolver::resolve(&program);
            ResolvedProgram {
                program,
                resolution,
            }
        }
    };

    resolution
        .diagnostics
        .iter()
        .map(|(unit, diagnostic)| diagnostic.in_unit(*unit, &program).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use {super::*, crate::test_format::parse_test_file};

    #[test]
    fn test_compile_passes_on_clean_code() {
        let file = parse_test_file("## testCompile()\n\nvoid f() { int i = 1; }\n");
        let results = run(&file, None);
        assert_eq!(
            results,
            vec![TestResult {
                name: "testCompile".into(),
                line: 1,
                outcome: Outcome::Pass,
                expected_error: None,
                actual_error: None,
            }]
        );
    }

    #[test]
    fn test_compile_fails_on_syntax_error() {
        let file = parse_test_file("## testCompile()\n\nvoid f( { }\n");
        let results = run(&file, None);
        assert_eq!(
            results,
            vec![TestResult {
                name: "testCompile".into(),
                line: 1,
                outcome: Outcome::Fail(
                    "compile error produced\n  2:9: error: Expected a type\n  2:11: error: Found \"}\" when expecting identifier\n  2:11: error: Found \"}\" when expecting \")\"\n  2:11: error: Found \"}\" when expecting \"{\"".into()
                ),
                expected_error: None,
                actual_error: None,
            }]
        );
    }

    #[test]
    fn parse_error_columns_count_characters_not_bytes() {
        let file = parse_test_file(
            "## expectError (\"2:17: error: Expected a type\")\n\n/* é */ void f( { }\n",
        );
        let results = run(&file, None);
        let first_error = results[0]
            .actual_error
            .as_deref()
            .and_then(|actual| actual.lines().next());
        assert_eq!(first_error, Some("2:17: error: Expected a type"));
    }

    #[test]
    fn test_compile_fails_on_a_recovered_parse_diagnostic() {
        let file = parse_test_file("## testCompile()\n\nenum Mode {}\n");
        let results = run(&file, None);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].outcome, Outcome::Fail(_)));
    }

    #[test]
    fn expect_error_passes_when_a_parse_error_occurs() {
        let file = parse_test_file("## expectError (\"2:9: error: nope\")\n\nvoid f( { }\n");
        let results = run(&file, None);
        assert_eq!(
            results,
            vec![TestResult {
                name: "expectError".into(),
                line: 1,
                outcome: Outcome::Pass,
                expected_error: Some("2:9: error: nope".into()),
                actual_error: Some("2:9: error: Expected a type\n2:11: error: Found \"}\" when expecting identifier\n2:11: error: Found \"}\" when expecting \")\"\n2:11: error: Found \"}\" when expecting \"{\"".into()),
            }]
        );
    }

    #[test]
    fn expect_error_passes_when_the_parser_rejects_code_without_a_diagnostic() {
        let file =
            parse_test_file("## expectError (\"2:12: error: nope\")\n\nvoid f() { node {} }\n");

        let results = run(&file, None);

        assert_eq!(results[0].outcome, Outcome::Pass);
        assert_eq!(
            results[0].actual_error.as_deref(),
            Some(REJECTED_WITHOUT_A_DIAGNOSTIC)
        );
    }

    #[test]
    fn test_compile_fails_when_the_parser_rejects_code_without_a_diagnostic() {
        let file = parse_test_file("## testCompile()\n\nvoid f() { node {} }\n");

        let results = run(&file, None);

        assert_eq!(
            results[0].outcome,
            Outcome::Fail(format!(
                "compile error produced\n  {REJECTED_WITHOUT_A_DIAGNOSTIC}"
            ))
        );
    }

    #[test]
    fn expect_error_fails_when_code_parses_cleanly() {
        let file = parse_test_file("## expectError (\"2:9: error: nope\")\n\nvoid f() {}\n");
        let results = run(&file, None);
        assert_eq!(
            results,
            vec![TestResult {
                name: "expectError".into(),
                line: 1,
                outcome: Outcome::Fail("expected an error but none occurred".into()),
                expected_error: Some("2:9: error: nope".into()),
                actual_error: None,
            }]
        );
    }

    #[test]
    fn disabled_section_is_skipped() {
        let file = parse_test_file("## disabled testCompile()\n\nvoid f( { }\n");
        let results = run(&file, None);
        assert_eq!(
            results,
            vec![TestResult {
                name: "testCompile".into(),
                line: 1,
                outcome: Outcome::Skipped("disabled".into()),
                expected_error: None,
                actual_error: None,
            }]
        );
    }

    #[test]
    fn unsupported_directives_are_skipped() {
        let file = parse_test_file(
            "## testConsole (\"hello\")\n\nprocessor P { output stream int out; void main() { out <- -1; advance(); } }\n",
        );
        let results = run(&file, None);
        assert_eq!(
            results,
            vec![TestResult {
                name: "testConsole".into(),
                line: 1,
                outcome: Outcome::Skipped("execution of 'testConsole' not yet supported".into()),
                expected_error: None,
                actual_error: None,
            }]
        );
    }

    fn stdlib_declaring_processor_p() -> StandardLibrary {
        resolver::declare_stdlib(&Program::new().with_source_file(
            SourceFile::new("processor P { output stream int out; }").with_name("lib.cmajor"),
        ))
    }

    #[test]
    fn note_in_the_standard_library_names_its_file() {
        let stdlib = stdlib_declaring_processor_p();
        let file = parse_test_file(
            "## expectError (\"2:11: error: nope\", {\"doNotWrapInTestNamespace\": true})\n\nprocessor P { output stream int out; }\n",
        );

        let results = run(&file, Some(&stdlib));

        assert_eq!(
            results[0].actual_error.as_deref(),
            Some(
                "2:11: error: The name 'P' is already in use //// lib.cmajor:1:11: note: See previous declaration"
            )
        );
    }

    #[test]
    fn compile_failure_note_in_the_standard_library_names_its_file() {
        let stdlib = stdlib_declaring_processor_p();
        let file = parse_test_file("## testCompile()\n\nprocessor P { output stream int out; }\n");

        let results = run(&file, Some(&stdlib));

        assert_eq!(
            results[0].outcome,
            Outcome::Fail(
                "compile error produced\n  2:11: error: The name 'P' is already in use //// lib.cmajor:1:11: note: See previous declaration"
                    .into()
            )
        );
    }

    #[test]
    fn global_section_is_prefixed_and_not_itself_a_result() {
        let file = parse_test_file(
            "## global\n\nstruct S { int i; }\n\n## testCompile()\n\nvoid f() { S s; }\n",
        );
        let results = run(&file, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, Outcome::Pass);
    }

    #[test]
    fn global_code_does_not_shift_body_line_numbers() {
        let with_global = parse_test_file(
            "## global\n\nstruct S { int i; }\nstruct T { int j; }\n\n## testCompile()\n\nvoid f( { }\n",
        );
        let without_global = parse_test_file("## testCompile()\n\nvoid f( { }\n");

        assert_eq!(
            run(&with_global, None)[0].outcome,
            run(&without_global, None)[0].outcome
        );
    }

    #[test]
    fn expect_error_body_is_wrapped_in_the_tests_namespace() {
        let file = parse_test_file(
            "## global\n\nstruct T { tests::S s; }\n\n## expectError (\"3:12: error: Cannot find symbol 'Missing'\")\n\nstruct S { int i; }\nstruct U { Missing m; }\n",
        );

        let results = run(&file, None);

        assert_eq!(
            results[0].actual_error.as_deref(),
            Some("3:12: error: Cannot find symbol 'Missing'")
        );
    }

    #[test]
    fn unwrapped_expect_error_is_compiled_bare() {
        let file = parse_test_file(
            "## expectError (\"1:12: error: Cannot find symbol 'Missing'\", {\"doNotWrapInTestNamespace\": true})\nstruct U { Missing m; }\n",
        );

        let results = run(&file, None);

        assert_eq!(
            results[0].actual_error.as_deref(),
            Some("1:12: error: Cannot find symbol 'Missing'")
        );
    }
}
