use {
    super::{Directive, TestFile, parser::Check},
    crate::{
        Program, SourceFile,
        resolver::{self, ResolvedProgram, StandardLibrary},
        utils::source::Line,
    },
    std::fmt,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TestResult {
    pub directive: Directive,
    pub line: Line,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail(Failure),
    Skipped(Skip),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    Rejected { diagnostics: Vec<String> },
    NoError,
    WrongError { expected: String, actual: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    Disabled,
    NotCompiled,
}

pub fn run(file: &TestFile, global: &str, baseline: Option<&StandardLibrary>) -> Vec<TestResult> {
    file.sections
        .iter()
        .filter(|section| !matches!(section.directive, Directive::Global { .. }))
        .map(|section| TestResult {
            directive: section.directive.clone(),
            line: section.line,
            outcome: if section.disabled {
                Outcome::Skipped(Skip::Disabled)
            } else {
                section.check(global).run(baseline)
            },
        })
        .collect()
}

impl Check<'_> {
    fn run(self, baseline: Option<&StandardLibrary>) -> Outcome {
        match self {
            Check::Accepts { source } => {
                let diagnostics = compile(source, baseline);
                if diagnostics.is_empty() {
                    Outcome::Pass
                } else {
                    Outcome::Fail(Failure::Rejected { diagnostics })
                }
            }
            Check::Rejects { source, expected } => {
                match compile(source, baseline).into_iter().next() {
                    None => Outcome::Fail(Failure::NoError),
                    Some(actual) if actual == expected => Outcome::Pass,
                    Some(actual) => Outcome::Fail(Failure::WrongError {
                        expected: expected.to_string(),
                        actual,
                    }),
                }
            }
            Check::NotCompiled => Outcome::Skipped(Skip::NotCompiled),
        }
    }
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

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Rejected { diagnostics } => {
                f.write_str("a valid program was rejected")?;
                for diagnostic in diagnostics {
                    write!(f, "\n  {diagnostic}")?;
                }
                Ok(())
            }
            Failure::NoError => f.write_str("an error was expected and none came"),
            Failure::WrongError { expected, actual } => write!(
                f,
                "the wrong error came\n  expected: {expected}\n  actual:   {actual}"
            ),
        }
    }
}

impl fmt::Display for Skip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Skip::Disabled => "disabled",
            Skip::NotCompiled => "not compiled",
        })
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::test_format::parse_test_file};

    fn no_files(name: &str) -> String {
        unreachable!("no global file should be read, but {name} was")
    }

    fn outcomes(test_file: &str, baseline: Option<&StandardLibrary>) -> Vec<Outcome> {
        let file = parse_test_file(test_file);
        run(&file, &file.global_source(no_files), baseline)
            .into_iter()
            .map(|result| result.outcome)
            .collect()
    }

    fn rejected(diagnostics: &[&str]) -> Outcome {
        Outcome::Fail(Failure::Rejected {
            diagnostics: diagnostics.iter().map(|d| d.to_string()).collect(),
        })
    }

    fn wrong_error(expected: &str, actual: &str) -> Outcome {
        Outcome::Fail(Failure::WrongError {
            expected: expected.into(),
            actual: actual.into(),
        })
    }

    #[test]
    fn result_names_its_directive_and_header_line() {
        let file = parse_test_file("## global\n\nint g;\n\n## testCompile()\n\nvoid f() {}\n");
        let results = run(&file, &file.global_source(no_files), None);

        assert_eq!(
            results,
            vec![TestResult {
                directive: Directive::TestCompile,
                line: Line::from(5),
                outcome: Outcome::Pass,
            }]
        );
    }

    #[test]
    fn test_compile_passes_on_clean_code() {
        assert_eq!(
            outcomes("## testCompile()\n\nvoid f() { int i = 1; }\n", None),
            vec![Outcome::Pass]
        );
    }

    #[test]
    fn test_compile_fails_with_every_diagnostic_on_a_syntax_error() {
        assert_eq!(
            outcomes("## testCompile()\n\nvoid f( { }\n", None),
            vec![rejected(&[
                "2:9: error: Expected a type",
                "2:11: error: Found \"}\" when expecting identifier",
                "2:11: error: Found \"}\" when expecting \")\"",
                "2:11: error: Found \"}\" when expecting \"{\"",
            ])]
        );
    }

    #[test]
    fn test_compile_fails_on_a_recovered_parse_diagnostic() {
        assert!(matches!(
            outcomes("## testCompile()\n\nenum Mode {}\n", None)[..],
            [Outcome::Fail(Failure::Rejected { .. })]
        ));
    }

    #[test]
    fn test_compile_fails_when_the_parser_rejects_code_without_a_diagnostic() {
        assert_eq!(
            outcomes("## testCompile()\n\nvoid f() { node {} }\n", None),
            vec![rejected(&[REJECTED_WITHOUT_A_DIAGNOSTIC])]
        );
    }

    #[test]
    fn test_function_test_processor_test_console_and_test_input_endpoints_are_compile_checks() {
        let results = outcomes(
            "## testFunction()\nstruct S { Missing m; }\n\
             ## testProcessor()\nstruct S { Missing m; }\n\
             ## testConsole (\"hi\")\nstruct S { Missing m; }\n\
             ## testInputEndpoints ([])\nstruct S { Missing m; }\n",
            None,
        );

        assert_eq!(
            results,
            vec![
                rejected(&["1:30: error: Cannot find symbol 'Missing'"]),
                rejected(&["1:12: error: Cannot find symbol 'Missing'"]),
                rejected(&["1:12: error: Cannot find symbol 'Missing'"]),
                rejected(&["1:12: error: Cannot find symbol 'Missing'"]),
            ]
        );
    }

    #[test]
    fn expect_error_passes_when_the_first_diagnostic_matches_exactly() {
        assert_eq!(
            outcomes(
                "## expectError (\"2:9: error: Expected a type\")\n\nvoid f( { }\n",
                None
            ),
            vec![Outcome::Pass]
        );
    }

    #[test]
    fn expect_error_fails_when_the_message_differs() {
        assert_eq!(
            outcomes(
                "## expectError (\"2:9: error: Expected a value\")\n\nvoid f( { }\n",
                None
            ),
            vec![wrong_error(
                "2:9: error: Expected a value",
                "2:9: error: Expected a type"
            )]
        );
    }

    #[test]
    fn expect_error_fails_when_the_position_differs() {
        assert_eq!(
            outcomes(
                "## expectError (\"2:10: error: Expected a type\")\n\nvoid f( { }\n",
                None
            ),
            vec![wrong_error(
                "2:10: error: Expected a type",
                "2:9: error: Expected a type"
            )]
        );
    }

    #[test]
    fn expect_error_counts_columns_in_characters_not_bytes() {
        assert_eq!(
            outcomes(
                "## expectError (\"2:17: error: Expected a type\")\n\n/* é */ void f( { }\n",
                None
            ),
            vec![Outcome::Pass]
        );
    }

    #[test]
    fn expect_error_fails_when_code_compiles_cleanly() {
        assert_eq!(
            outcomes(
                "## expectError (\"2:9: error: nope\")\n\nvoid f() {}\n",
                None
            ),
            vec![Outcome::Fail(Failure::NoError)]
        );
    }

    #[test]
    fn rejection_without_a_diagnostic_is_the_wrong_error_for_expect_error() {
        assert_eq!(
            outcomes(
                "## expectError (\"2:12: error: nope\")\n\nvoid f() { node {} }\n",
                None
            ),
            vec![wrong_error(
                "2:12: error: nope",
                REJECTED_WITHOUT_A_DIAGNOSTIC
            )]
        );
    }

    #[test]
    fn disabled_section_is_skipped_as_disabled() {
        assert_eq!(
            outcomes("## disabled testCompile()\n\nvoid f( { }\n", None),
            vec![Outcome::Skipped(Skip::Disabled)]
        );
    }

    #[test]
    fn other_directives_are_skipped() {
        assert_eq!(
            outcomes(
                "## runScript ({})\n\n## testPatch (\"p.cmajorpatch\")\n\n## performanceTest ({})\n\n## if (x) testCompile()\n\nvoid f() {}\n",
                None
            ),
            vec![Outcome::Skipped(Skip::NotCompiled); 4]
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

        assert_eq!(
            outcomes(
                "## expectError (\"2:11: error: The name 'P' is already in use //// lib.cmajor:1:11: note: See previous declaration\", {\"doNotWrapInTestNamespace\": true})\n\nprocessor P { output stream int out; }\n",
                Some(&stdlib)
            ),
            vec![Outcome::Pass]
        );
    }

    #[test]
    fn compile_failure_note_in_the_standard_library_names_its_file() {
        let stdlib = stdlib_declaring_processor_p();

        assert_eq!(
            outcomes(
                "## testCompile()\n\nprocessor P { output stream int out; }\n",
                Some(&stdlib)
            ),
            vec![rejected(&[
                "2:11: error: The name 'P' is already in use //// lib.cmajor:1:11: note: See previous declaration"
            ])]
        );
    }

    #[test]
    fn global_section_is_not_itself_a_result() {
        assert_eq!(
            outcomes(
                "## global\n\nstruct S { int i; }\n\n## testCompile()\n\nvoid f() { S s; }\n",
                None
            ),
            vec![Outcome::Pass]
        );
    }

    #[test]
    fn global_code_does_not_shift_body_line_numbers() {
        assert_eq!(
            outcomes(
                "## global\n\nstruct S { int i; }\nstruct T { int j; }\n\n## testCompile()\n\nvoid f( { }\n",
                None
            ),
            outcomes("## testCompile()\n\nvoid f( { }\n", None)
        );
    }

    #[test]
    fn expect_error_body_is_wrapped_in_the_tests_namespace() {
        assert_eq!(
            outcomes(
                "## global\n\nstruct T { tests::S s; }\n\n## expectError (\"3:12: error: Cannot find symbol 'Missing'\")\n\nstruct S { int i; }\nstruct U { Missing m; }\n",
                None
            ),
            vec![Outcome::Pass]
        );
    }

    #[test]
    fn unwrapped_expect_error_is_compiled_bare() {
        assert_eq!(
            outcomes(
                "## expectError (\"1:12: error: Cannot find symbol 'Missing'\", {\"doNotWrapInTestNamespace\": true})\nstruct U { Missing m; }\n",
                None
            ),
            vec![Outcome::Pass]
        );
    }
}
