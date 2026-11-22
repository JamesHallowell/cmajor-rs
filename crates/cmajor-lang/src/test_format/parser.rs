use std::fmt;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TestFile {
    pub prelude: String,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub directive: Directive,
    pub disabled: bool,
    pub body: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    Global,
    TestCompile,
    TestFunction,
    TestProcessor,
    TestConsole,
    TestInputEndpoints,
    ExpectError(ExpectedError),
    Other { name: String },
    Unknown { raw: String },
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExpectedError {
    pub message: String,
    pub namespace: TestNamespace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TestNamespace {
    #[default]
    Wrapped,
    Unwrapped,
}

const TEST_PROCESSOR: &str =
    "processor TestProcessor { output stream float32 out; void main() { advance(); } }";

impl TestFile {
    pub fn global_source(&self) -> String {
        self.sections
            .iter()
            .filter(|section| section.directive == Directive::Global)
            .map(|section| section.body.as_str())
            .collect()
    }
}

impl Section {
    pub fn source(&self, global: &str) -> Option<String> {
        let body = &self.body;
        match &self.directive {
            Directive::ExpectError(ExpectedError {
                namespace: TestNamespace::Wrapped,
                ..
            }) => Some(format!(
                "namespace tests {{ {body} }}\n{TEST_PROCESSOR}\n{global}"
            )),
            Directive::TestFunction => Some(format!("namespace tests {{ {body} }}{global}")),
            Directive::ExpectError(ExpectedError {
                namespace: TestNamespace::Unwrapped,
                ..
            })
            | Directive::TestCompile
            | Directive::TestProcessor
            | Directive::TestConsole
            | Directive::TestInputEndpoints => Some(format!("{body}{global}")),
            Directive::Global | Directive::Other { .. } | Directive::Unknown { .. } => None,
        }
    }
}

impl fmt::Display for Directive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Directive::Global => "global",
            Directive::TestCompile => "testCompile",
            Directive::TestFunction => "testFunction",
            Directive::TestProcessor => "testProcessor",
            Directive::TestConsole => "testConsole",
            Directive::TestInputEndpoints => "testInputEndpoints",
            Directive::ExpectError(_) => "expectError",
            Directive::Other { name } => name,
            Directive::Unknown { raw } => raw,
        })
    }
}

impl ExpectedError {
    fn parse(args: &str) -> Self {
        let mut args = split_top_level_commas(args).map(str::trim);
        let message = args
            .next()
            .and_then(parse_string_literal)
            .unwrap_or_default();
        let namespace = args
            .next()
            .map(TestNamespace::from_options)
            .unwrap_or_default();
        Self { message, namespace }
    }
}

impl TestNamespace {
    fn from_options(options: &str) -> Self {
        let Some(fields) = options
            .strip_prefix('{')
            .and_then(|fields| fields.strip_suffix('}'))
        else {
            return TestNamespace::Wrapped;
        };
        let do_not_wrap = split_top_level_commas(fields)
            .filter_map(property)
            .filter(|(key, _)| key == "doNotWrapInTestNamespace")
            .last();
        match do_not_wrap {
            Some((_, value)) if is_truthy(value) => TestNamespace::Unwrapped,
            _ => TestNamespace::Wrapped,
        }
    }
}

fn property(field: &str) -> Option<(String, &str)> {
    let (colon, ..) = unquoted_chars(field).find(|&(_, c, depth)| c == ':' && depth == 0)?;
    let key = field[..colon].trim();
    let key = if key.starts_with(['"', '\'']) {
        parse_string_literal(key)?
    } else {
        key.to_string()
    };
    Some((key, field[colon + 1..].trim()))
}

fn is_truthy(literal: &str) -> bool {
    match literal {
        "false" | "null" | "undefined" | "NaN" | "\"\"" | "''" | "``" => false,
        _ => number_value(literal).is_none_or(|n| n != 0.0 && !n.is_nan()),
    }
}

fn number_value(literal: &str) -> Option<f64> {
    let unsigned = literal.trim_start_matches(['-', '+']);
    let digits = unsigned
        .strip_suffix('n')
        .unwrap_or(unsigned)
        .replace('_', "");
    let prefixed = [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ]
    .into_iter()
    .find_map(|(prefix, radix)| Some((digits.strip_prefix(prefix)?, radix)));
    match prefixed {
        Some((digits, radix)) => u128::from_str_radix(digits, radix).ok().map(|n| n as f64),
        None => digits.parse().ok(),
    }
}

pub fn parse_test_file(source: &str) -> TestFile {
    let lines: Vec<&str> = source.lines().collect();

    let first_header = lines
        .iter()
        .position(|line| line.trim_start().starts_with("##"));

    let Some(first_header) = first_header else {
        return TestFile {
            prelude: lines.join("\n"),
            sections: Vec::new(),
        };
    };

    let prelude = lines[..first_header].join("\n");

    let mut headers = Vec::new();
    for (i, line) in lines.iter().enumerate().skip(first_header) {
        if line.trim_start().starts_with("##") {
            headers.push(i);
        }
    }

    let mut sections = Vec::with_capacity(headers.len());
    for (idx, &header_line) in headers.iter().enumerate() {
        let body_start = header_line + 1;
        let body_end = headers.get(idx + 1).copied().unwrap_or(lines.len());
        let body = lines[body_start..body_end]
            .iter()
            .flat_map(|line| [*line, "\n"])
            .collect();

        let header_rest = lines[header_line].trim_start()[2..].trim();
        let (directive, disabled) = classify_header(header_rest);

        sections.push(Section {
            directive,
            disabled,
            body,
            line: header_line + 1,
        });
    }

    TestFile { prelude, sections }
}

fn classify_header(header_rest: &str) -> (Directive, bool) {
    if let Some(rest) = strip_word(header_rest, "disabled") {
        let rest = rest.trim();
        return (classify_directive(rest), true);
    }

    (classify_directive(header_rest), false)
}

fn classify_directive(rest: &str) -> Directive {
    let rest = rest.trim();
    let unknown = || Directive::Unknown {
        raw: rest.to_string(),
    };

    let Some(Call { name, args }) = Call::parse(rest) else {
        return unknown();
    };

    match name {
        "global" if args.is_none() => Directive::Global,
        "testCompile" => Directive::TestCompile,
        "testFunction" => Directive::TestFunction,
        "testProcessor" => Directive::TestProcessor,
        "testConsole" => Directive::TestConsole,
        "testInputEndpoints" => Directive::TestInputEndpoints,
        "expectError" => Directive::ExpectError(ExpectedError::parse(args.unwrap_or_default())),
        "if" => unknown(),
        name => Directive::Other {
            name: name.to_string(),
        },
    }
}

fn strip_word<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(word)?;
    match rest.chars().next() {
        None => Some(rest),
        Some(c) if !c.is_alphanumeric() && c != '_' => Some(rest),
        _ => None,
    }
}

struct Call<'a> {
    name: &'a str,
    args: Option<&'a str>,
}

impl<'a> Call<'a> {
    fn parse(text: &'a str) -> Option<Self> {
        let name_len = text
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(text.len());
        if name_len == 0 {
            return None;
        }
        let (name, rest) = text.split_at(name_len);

        let rest = rest.trim();
        if rest.is_empty() {
            return Some(Self { name, args: None });
        }

        if !rest.starts_with('(') {
            return None;
        }
        let (close, ..) = unquoted_chars(rest).find(|&(_, c, depth)| c == ')' && depth == 0)?;
        ends_statement(&rest[close + 1..]).then(|| Self {
            name,
            args: Some(rest[1..close].trim()),
        })
    }
}

fn ends_statement(text: &str) -> bool {
    let text = text.trim_start();
    let text = text.strip_prefix(';').unwrap_or(text).trim_start();
    text.is_empty() || text.starts_with("//")
}

fn split_top_level_commas(text: &str) -> impl Iterator<Item = &str> {
    let mut start = 0;
    unquoted_chars(text)
        .filter(|&(_, c, depth)| c == ',' && depth == 0)
        .map(|(i, ..)| i)
        .chain([text.len()])
        .map(move |end| {
            let part = &text[start..end];
            start = end + 1;
            part
        })
}

fn unquoted_chars(text: &str) -> impl Iterator<Item = (usize, char, usize)> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    text.char_indices().filter_map(move |(i, c)| {
        if let Some(open) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == open {
                quote = None;
            }
            return None;
        }
        match c {
            '"' | '\'' => {
                quote = Some(c);
                None
            }
            '(' | '[' | '{' => {
                depth += 1;
                Some((i, c, depth - 1))
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                Some((i, c, depth))
            }
            _ => Some((i, c, depth)),
        }
    })
}

fn parse_string_literal(literal: &str) -> Option<String> {
    let mut chars = literal.char_indices().peekable();
    let quote = chars
        .next()
        .map(|(_, c)| c)
        .filter(|c| matches!(c, '"' | '\''))?;
    let mut value = String::new();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => value.push(match chars.next()?.1 {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'b' => '\u{8}',
                'f' => '\u{c}',
                'v' => '\u{b}',
                '0' => '\0',
                'x' => hex_char(&take_chars(&mut chars, 2))?,
                'u' if chars.next_if(|&(_, c)| c == '{').is_some() => {
                    let digits: String = chars
                        .by_ref()
                        .map(|(_, c)| c)
                        .take_while(|&c| c != '}')
                        .collect();
                    hex_char(&digits)?
                }
                'u' => hex_char(&take_chars(&mut chars, 4))?,
                other => other,
            }),
            c if c == quote => return (i + 1 == literal.len()).then_some(value),
            c => value.push(c),
        }
    }
    None
}

fn take_chars(chars: &mut impl Iterator<Item = (usize, char)>, count: usize) -> String {
    chars.take(count).map(|(_, c)| c).collect()
}

fn hex_char(digits: &str) -> Option<char> {
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    char::from_u32(u32::from_str_radix(digits, 16).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelude_is_captured_verbatim() {
        let file = parse_test_file(
            "// helper comment\nfunction helper() { return 1; }\n\n## testCompile()\n\nvoid f() {}\n",
        );
        assert_eq!(
            file.prelude,
            "// helper comment\nfunction helper() { return 1; }\n"
        );
        assert_eq!(file.sections.len(), 1);
    }

    #[test]
    fn no_header_lines_is_all_prelude() {
        let file = parse_test_file("just some javascript\nwith no directives\n");
        assert_eq!(file.sections.len(), 0);
        assert_eq!(file.prelude, "just some javascript\nwith no directives");
    }

    #[test]
    fn global_section() {
        let file =
            parse_test_file("## global\n\nprocessor P {}\n\n## testCompile()\n\nvoid f() {}\n");
        assert_eq!(file.sections[0].directive, Directive::Global);
        assert!(!file.sections[0].disabled);
        assert_eq!(file.sections[0].body, "\nprocessor P {}\n\n");
    }

    fn directive_of(header: &str) -> Directive {
        let file = parse_test_file(&format!("{header}\n\nvoid f() {{}}\n"));
        file.sections[0].directive.clone()
    }

    fn expected_error(message: &str, namespace: TestNamespace) -> Directive {
        Directive::ExpectError(ExpectedError {
            message: message.to_string(),
            namespace,
        })
    }

    #[test]
    fn disabled_wraps_inner_directive() {
        let file =
            parse_test_file("## disabled expectError (\"2:9: error: nope\")\n\nvoid f() {}\n");
        let section = &file.sections[0];
        assert!(section.disabled);
        assert_eq!(
            section.directive,
            expected_error("2:9: error: nope", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn test_compile_header_parses_to_test_compile() {
        assert_eq!(directive_of("## testCompile()"), Directive::TestCompile);
    }

    #[test]
    fn test_function_header_parses_to_test_function() {
        assert_eq!(directive_of("## testFunction()"), Directive::TestFunction);
    }

    #[test]
    fn test_processor_header_parses_to_test_processor() {
        assert_eq!(directive_of("## testProcessor()"), Directive::TestProcessor);
    }

    #[test]
    fn test_console_header_parses_to_test_console() {
        assert_eq!(
            directive_of("## testConsole (\"hello\")"),
            Directive::TestConsole
        );
    }

    #[test]
    fn test_input_endpoints_header_parses_to_test_input_endpoints() {
        assert_eq!(
            directive_of("## testInputEndpoints ([\"in\"])"),
            Directive::TestInputEndpoints
        );
    }

    #[test]
    fn call_arguments_may_contain_nested_parentheses() {
        assert_eq!(
            directive_of("## testFunction (makeArgs (1))"),
            Directive::TestFunction
        );
    }

    #[test]
    fn trailing_semicolon_and_comment_are_ignored() {
        assert_eq!(
            directive_of("## testCompile(); // flaky on wasm"),
            Directive::TestCompile
        );
    }

    #[test]
    fn trailing_code_makes_the_header_unknown() {
        assert_eq!(
            directive_of("## testCompile() testFunction()"),
            Directive::Unknown {
                raw: "testCompile() testFunction()".to_string(),
            }
        );
    }

    #[test]
    fn expect_error_message_is_unescaped() {
        assert_eq!(
            directive_of(r#"## expectError ("2:9: error: Found \"XX\"")"#),
            expected_error("2:9: error: Found \"XX\"", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn expect_error_message_may_be_single_quoted() {
        assert_eq!(
            directive_of("## expectError ('10:20: error: An array index must be an integer type')"),
            expected_error(
                "10:20: error: An array index must be an integer type",
                TestNamespace::Wrapped
            )
        );
    }

    #[test]
    fn expect_error_message_may_contain_parentheses_and_commas() {
        assert_eq!(
            directive_of(
                "## expectError (\"1:1: error: Cannot connect in (float32<2>) to out (float32<3>), sorry\")"
            ),
            expected_error(
                "1:1: error: Cannot connect in (float32<2>) to out (float32<3>), sorry",
                TestNamespace::Wrapped
            )
        );
    }

    #[test]
    fn bare_expect_error_has_an_empty_message() {
        assert_eq!(
            directive_of("## expectError"),
            expected_error("", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn do_not_wrap_option_parses_to_an_unwrapped_expected_error() {
        assert_eq!(
            directive_of(
                "## expectError (\"2:11: error: The name 'std' is already in use\", {\"doNotWrapInTestNamespace\":\"true\"})"
            ),
            expected_error(
                "2:11: error: The name 'std' is already in use",
                TestNamespace::Unwrapped
            )
        );
    }

    #[test]
    fn falsy_do_not_wrap_option_leaves_the_test_wrapped() {
        assert_eq!(
            directive_of("## expectError (\"1:1: error: a\", { doNotWrapInTestNamespace: false })"),
            expected_error("1:1: error: a", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn unrelated_expect_error_options_leave_the_test_wrapped() {
        assert_eq!(
            directive_of(
                "## expectError (\"error: Stack size limit exceeded\", { maxStackSize: 50000 })"
            ),
            expected_error("error: Stack size limit exceeded", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn unreadable_expect_error_message_is_empty() {
        assert_eq!(
            directive_of("## expectError (\"1:1: \" + \"error: a\")"),
            expected_error("", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn unreadable_expect_error_options_leave_the_test_wrapped() {
        assert_eq!(
            directive_of("## expectError (\"1:1: error: a\", getOptions())"),
            expected_error("1:1: error: a", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn quoted_option_key_may_contain_a_colon() {
        assert_eq!(
            directive_of(
                "## expectError (\"1:1: error: a\", {\"a:b\": 1, \"doNotWrapInTestNamespace\": true})"
            ),
            expected_error("1:1: error: a", TestNamespace::Unwrapped)
        );
    }

    #[test]
    fn shorthand_option_is_ignored() {
        assert_eq!(
            directive_of("## expectError (\"1:1: error: a\", { doNotWrapInTestNamespace })"),
            expected_error("1:1: error: a", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn last_do_not_wrap_option_wins() {
        assert_eq!(
            directive_of(
                "## expectError (\"1:1: error: a\", { doNotWrapInTestNamespace: true, doNotWrapInTestNamespace: false })"
            ),
            expected_error("1:1: error: a", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn zero_in_any_numeric_form_leaves_the_test_wrapped() {
        for zero in [
            "0", "0.0", "-0", ".0", "0e5", "0x0", "0b0", "0o0", "0n", "0_0",
        ] {
            assert_eq!(
                directive_of(&format!(
                    "## expectError (\"1:1: error: a\", {{ doNotWrapInTestNamespace: {zero} }})"
                )),
                expected_error("1:1: error: a", TestNamespace::Wrapped),
                "{zero}"
            );
        }
    }

    #[test]
    fn non_zero_number_unwraps_the_test() {
        assert_eq!(
            directive_of("## expectError (\"1:1: error: a\", { doNotWrapInTestNamespace: 0x1 })"),
            expected_error("1:1: error: a", TestNamespace::Unwrapped)
        );
    }

    #[test]
    fn hex_escape_is_decoded() {
        assert_eq!(
            directive_of(r#"## expectError ("Found \x22x\x22")"#),
            expected_error("Found \"x\"", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn four_digit_unicode_escape_is_decoded() {
        assert_eq!(
            directive_of(&format!(
                "## expectError (\"Found {0}u0022x{0}u0022\")",
                '\\'
            )),
            expected_error("Found \"x\"", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn braced_unicode_escape_is_decoded() {
        assert_eq!(
            directive_of(r#"## expectError ("Found \u{1F600}")"#),
            expected_error("Found \u{1F600}", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn control_character_escapes_are_decoded() {
        assert_eq!(
            directive_of(r#"## expectError ("\b\f\v\0")"#),
            expected_error("\u{8}\u{c}\u{b}\0", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn malformed_escape_makes_the_message_empty() {
        assert_eq!(
            directive_of(r#"## expectError ("bad \x4")"#),
            expected_error("", TestNamespace::Wrapped)
        );
    }

    #[test]
    fn other_directive_keeps_only_its_name() {
        assert_eq!(
            directive_of(
                "## runScript ({ frequency:1000, blockSize:32, samplesToRender:1000, subDir:\"gain\"})"
            ),
            Directive::Other {
                name: "runScript".to_string(),
            }
        );
    }

    #[test]
    fn other_directive_may_omit_its_arguments() {
        assert_eq!(
            directive_of("## performanceTest"),
            Directive::Other {
                name: "performanceTest".to_string(),
            }
        );
    }

    #[test]
    fn test_kind_displays_as_its_header_name() {
        assert_eq!(
            directive_of("## expectError (\"1:1: error: a\")").to_string(),
            "expectError"
        );
    }

    #[test]
    fn other_directive_displays_as_its_name() {
        assert_eq!(directive_of("## testPatch ()").to_string(), "testPatch");
    }

    #[test]
    fn conditional_header_is_unknown() {
        let file = parse_test_file(
            "## if (getEngineName() != \"cpp\") testConsole (\"hello\")\n\nprocessor P {}\n",
        );
        assert_eq!(
            file.sections[0].directive,
            Directive::Unknown {
                raw: "if (getEngineName() != \"cpp\") testConsole (\"hello\")".to_string(),
            }
        );
    }

    #[test]
    fn multiple_sections_split_correctly() {
        let file = parse_test_file(
            "## expectError (\"1:1: error: a\")\n\nvoid a() {}\n\n## expectError (\"1:1: error: b\")\n\nvoid b() {}\n",
        );
        assert_eq!(file.sections.len(), 2);
        assert_eq!(file.sections[0].body, "\nvoid a() {}\n\n");
        assert_eq!(file.sections[1].body, "\nvoid b() {}\n");
        assert_eq!(file.sections[0].line, 1);
        assert_eq!(file.sections[1].line, 5);
    }

    #[test]
    fn body_starts_on_the_line_after_its_header() {
        let file = parse_test_file("## testCompile()\nvoid f() {}\nvoid g() {}\n");
        assert_eq!(file.sections[0].body, "void f() {}\nvoid g() {}\n");
    }

    #[test]
    fn last_body_ends_with_a_newline_when_the_file_does_not() {
        let file = parse_test_file("## testCompile()\nvoid f() {}");
        assert_eq!(file.sections[0].body, "void f() {}\n");
    }

    #[test]
    fn section_with_no_lines_has_an_empty_body() {
        let file = parse_test_file("## testCompile()\n## testCompile()\nvoid f() {}\n");
        assert_eq!(file.sections[0].body, "");
    }

    fn source_of(header: &str, global: &str) -> Option<String> {
        let file = parse_test_file(&format!("{header}\nint body;\n"));
        file.sections[0].source(global)
    }

    #[test]
    fn expect_error_source_wraps_the_body_and_adds_the_test_processor() {
        assert_eq!(
            source_of("## expectError (\"1:1: error: a\")", "int global;\n").as_deref(),
            Some(
                "namespace tests { int body;\n }\nprocessor TestProcessor { output stream float32 out; void main() { advance(); } }\nint global;\n"
            )
        );
    }

    #[test]
    fn unwrapped_expect_error_source_is_the_body_then_global_code() {
        assert_eq!(
            source_of(
                "## expectError (\"1:1: error: a\", {\"doNotWrapInTestNamespace\": true})",
                "int global;\n"
            )
            .as_deref(),
            Some("int body;\nint global;\n")
        );
    }

    #[test]
    fn test_function_source_wraps_the_body_without_a_test_processor() {
        assert_eq!(
            source_of("## testFunction()", "int global;\n").as_deref(),
            Some("namespace tests { int body;\n }int global;\n")
        );
    }

    #[test]
    fn test_compile_source_is_the_body_then_global_code() {
        assert_eq!(
            source_of("## testCompile()", "int global;\n").as_deref(),
            Some("int body;\nint global;\n")
        );
    }

    #[test]
    fn test_processor_source_is_the_body_then_global_code() {
        assert_eq!(
            source_of("## testProcessor()", "int global;\n").as_deref(),
            Some("int body;\nint global;\n")
        );
    }

    #[test]
    fn test_console_source_is_the_body_then_global_code() {
        assert_eq!(
            source_of("## testConsole (\"hi\")", "int global;\n").as_deref(),
            Some("int body;\nint global;\n")
        );
    }

    #[test]
    fn test_input_endpoints_source_is_the_body_then_global_code() {
        assert_eq!(
            source_of("## testInputEndpoints ([])", "int global;\n").as_deref(),
            Some("int body;\nint global;\n")
        );
    }

    #[test]
    fn global_section_has_no_source_of_its_own() {
        assert_eq!(source_of("## global", ""), None);
    }

    #[test]
    fn other_directive_has_no_source() {
        assert_eq!(source_of("## runScript ({})", ""), None);
    }

    #[test]
    fn unknown_header_has_no_source() {
        assert_eq!(source_of("## if (x) testCompile()", ""), None);
    }

    #[test]
    fn global_source_joins_every_global_section_in_order() {
        let file =
            parse_test_file("## global\nint a;\n## testCompile()\nint b;\n## global\nint c;\n");
        assert_eq!(file.global_source(), "int a;\nint c;\n");
    }
}
