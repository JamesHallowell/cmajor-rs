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
    ExpectError { error: String },
    Call { name: String, raw_args: String },
    Unknown { raw: String },
}

impl Directive {
    pub fn expected_error_message(&self) -> Option<String> {
        let Directive::ExpectError { error, .. } = self else {
            return None;
        };
        let start = error.find('"')? + 1;
        let rest = &error[start..];
        let mut out = String::new();
        let mut chars = rest.chars();
        loop {
            match chars.next()? {
                '\\' => match chars.next()? {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    other => out.push(other),
                },
                '"' => return Some(out),
                c => out.push(c),
            }
        }
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
        let body = lines[body_start..body_end].join("\n");

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
    if rest == "global" {
        return Directive::Global;
    }

    match parse_call(rest) {
        Some(("expectError", error)) => Directive::ExpectError { error },
        Some(("testCompile", _)) => Directive::TestCompile,
        Some((name, raw_args)) if name != "if" => Directive::Call {
            name: name.to_string(),
            raw_args,
        },
        _ => Directive::Unknown {
            raw: rest.to_string(),
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

fn parse_call(text: &str) -> Option<(&str, String)> {
    let name_len = text
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .count();
    if name_len == 0 {
        return None;
    }
    let name = &text[..name_len];

    let after_name = &text[name.len()..];
    let open_offset = after_name.find(|c: char| !c.is_whitespace())?;
    if after_name[open_offset..].chars().next()? != '(' {
        return None;
    }

    let paren_start = name.len() + open_offset;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut close_pos = None;

    for (i, c) in text[paren_start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close_pos = Some(paren_start + i);
                    break;
                }
            }
            _ => {}
        }
    }

    let close_pos = close_pos?;
    let raw_args = text[paren_start + 1..close_pos].trim().to_string();
    Some((name, raw_args))
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
        assert_eq!(file.sections[0].body, "\nprocessor P {}\n");
    }

    #[test]
    fn disabled_wraps_inner_directive() {
        let file =
            parse_test_file("## disabled expectError (\"2:9: error: nope\")\n\nvoid f() {}\n");
        let section = &file.sections[0];
        assert!(section.disabled);
        assert_eq!(
            section.directive,
            Directive::ExpectError {
                error: "\"2:9: error: nope\"".to_string(),
            }
        );
    }

    #[test]
    fn expect_error_message_is_extracted_and_unescaped() {
        let file =
            parse_test_file("## expectError (\"2:9: error: Found \\\"XX\\\"\")\n\nvoid f() {}\n");
        let msg = file.sections[0].directive.expected_error_message().unwrap();
        assert_eq!(msg, "2:9: error: Found \"XX\"");
    }

    #[test]
    fn raw_args_balances_nested_parens() {
        let file = parse_test_file(
            "## runScript ({ frequency:1000, blockSize:32, samplesToRender:1000, subDir:\"gain\"})\n\nprocessor P {}\n",
        );
        let Directive::Call { name, raw_args } = &file.sections[0].directive else {
            panic!("expected a call directive");
        };
        assert_eq!(name, "runScript");
        assert_eq!(
            raw_args,
            "{ frequency:1000, blockSize:32, samplesToRender:1000, subDir:\"gain\"}"
        );
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
        assert_eq!(file.sections[0].body, "\nvoid a() {}\n");
        assert_eq!(file.sections[1].body, "\nvoid b() {}");
        assert_eq!(file.sections[0].line, 1);
        assert_eq!(file.sections[1].line, 5);
    }
}
