use {
    super::{Directive, TestFile},
    crate::parser,
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
}

pub fn run(file: &TestFile) -> Vec<TestResult> {
    let global_code: String = file
        .sections
        .iter()
        .filter(|section| section.directive == Directive::Global)
        .map(|section| section.body.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    file.sections
        .iter()
        .filter(|section| section.directive != Directive::Global)
        .map(|section| {
            let name = directive_name(&section.directive);
            let expected_error = match &section.directive {
                Directive::ExpectError { .. } => section.directive.expected_error_message(),
                _ => None,
            };

            let outcome = if section.disabled {
                Outcome::Skipped("disabled".to_string())
            } else {
                match &section.directive {
                    Directive::TestCompile => {
                        let source = format!("{global_code}\n{}", section.body);
                        let parse = parser::parse(&source);
                        if parse.ast.has_errors() {
                            let detail = describe_parse_errors(&parse, &source);
                            Outcome::Fail(format!("parse error produced\n{detail}"))
                        } else {
                            Outcome::Pass
                        }
                    }
                    Directive::ExpectError { .. } => {
                        let source = format!("{global_code}\n{}", section.body);
                        let parse = parser::parse(&source);
                        if parse.ast.has_errors() {
                            Outcome::Pass
                        } else {
                            Outcome::Fail("expected a parse error but none occurred".to_string())
                        }
                    }
                    _ => Outcome::Skipped(format!("execution of '{name}' not yet supported")),
                }
            };

            TestResult {
                name,
                line: section.line,
                outcome,
                expected_error,
            }
        })
        .collect()
}

fn describe_parse_errors(parse: &parser::Parse, source: &str) -> String {
    parse
        .ast
        .error_tokens()
        .into_iter()
        .map(|token| {
            let Some(span) = parse.tokens.span(token) else {
                return "  <error token out of range>".to_string();
            };
            let start = span.start as usize;
            let (line, col) = line_col(source, start);
            let line_text = source.lines().nth(line - 1).unwrap_or("");
            let token_text = &source[start..span.end as usize];
            format!("  {line}:{col}: at {token_text:?} in {line_text:?}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_col(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for ch in source[..byte_offset.min(source.len())].chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn directive_name(directive: &Directive) -> String {
    match directive {
        Directive::Global => "global".to_string(),
        Directive::TestCompile => "testCompile".to_string(),
        Directive::ExpectError { .. } => "expectError".to_string(),
        Directive::Call { name, .. } => name.clone(),
        Directive::Unknown { raw } => raw.clone(),
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::test_format::parse_test_file};

    #[test]
    fn test_compile_passes_on_clean_code() {
        let file = parse_test_file("## testCompile()\n\nvoid f() { int i = 1; }\n");
        let results = run(&file);
        assert_eq!(
            results,
            vec![TestResult {
                name: "testCompile".into(),
                line: 1,
                outcome: Outcome::Pass,
                expected_error: None,
            }]
        );
    }

    #[test]
    fn test_compile_fails_on_syntax_error() {
        let file = parse_test_file("## testCompile()\n\nvoid f( { }\n");
        let results = run(&file);
        assert_eq!(
            results,
            vec![TestResult {
                name: "testCompile".into(),
                line: 1,
                outcome: Outcome::Fail(
                    "parse error produced\n  3:9: at \"{\" in \"void f( { }\"".into()
                ),
                expected_error: None,
            }]
        );
    }

    #[test]
    fn expect_error_passes_when_a_parse_error_occurs() {
        let file = parse_test_file("## expectError (\"2:9: error: nope\")\n\nvoid f( { }\n");
        let results = run(&file);
        assert_eq!(
            results,
            vec![TestResult {
                name: "expectError".into(),
                line: 1,
                outcome: Outcome::Pass,
                expected_error: Some("2:9: error: nope".into()),
            }]
        );
    }

    #[test]
    fn expect_error_fails_when_code_parses_cleanly() {
        let file = parse_test_file("## expectError (\"2:9: error: nope\")\n\nvoid f() {}\n");
        let results = run(&file);
        assert_eq!(
            results,
            vec![TestResult {
                name: "expectError".into(),
                line: 1,
                outcome: Outcome::Fail("expected a parse error but none occurred".into()),
                expected_error: Some("2:9: error: nope".into()),
            }]
        );
    }

    #[test]
    fn disabled_section_is_skipped() {
        let file = parse_test_file("## disabled testCompile()\n\nvoid f( { }\n");
        let results = run(&file);
        assert_eq!(
            results,
            vec![TestResult {
                name: "testCompile".into(),
                line: 1,
                outcome: Outcome::Skipped("disabled".into()),
                expected_error: None,
            }]
        );
    }

    #[test]
    fn unsupported_directives_are_skipped() {
        let file = parse_test_file(
            "## testConsole (\"hello\")\n\nprocessor P { output stream int out; void main() { out <- -1; advance(); } }\n",
        );
        let results = run(&file);
        assert_eq!(
            results,
            vec![TestResult {
                name: "testConsole".into(),
                line: 1,
                outcome: Outcome::Skipped("execution of 'testConsole' not yet supported".into()),
                expected_error: None,
            }]
        );
    }

    #[test]
    fn global_section_is_prefixed_and_not_itself_a_result() {
        let file = parse_test_file(
            "## global\n\nstruct S { int i; }\n\n## testCompile()\n\nvoid f() { S s; }\n",
        );
        let results = run(&file);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].outcome, Outcome::Pass);
    }
}
