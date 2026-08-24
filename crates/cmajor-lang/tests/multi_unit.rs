use cmajor_lang::SourceFile;

fn stdlib_units() -> Vec<SourceFile> {
    vec![
        SourceFile {
            name: "oscillator.cmajor".to_string(),
            source: indoc::indoc! {"
                namespace teststd::osc
                {
                    float sine (float phase)
                    {
                        return phase;
                    }
                }
            "}
            .to_string(),
        },
        SourceFile {
            name: "envelope.cmajor".to_string(),
            source: indoc::indoc! {"
                namespace teststd::env
                {
                    float attack (float level)
                    {
                        return level;
                    }
                }
            "}
            .to_string(),
        },
    ]
}

#[test]
fn compiles_a_program_against_a_synthetic_standard_library() {
    let mut units = stdlib_units();
    units.push(SourceFile {
        name: "main.cmajor".to_string(),
        source: indoc::indoc! {"
            processor P
            {
                output stream float out;
                void main()
                {
                    let x = teststd::osc::sine (1.0f);
                    let y = teststd::env::attack (1.0f);
                }
            }
        "}
        .to_string(),
    });

    let program = cmajor_lang::compile::compile(units);

    assert!(
        program.resolution.diagnostics.is_empty(),
        "expected no diagnostics, got: {:?}",
        program
            .resolution
            .diagnostics
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );
}

#[test]
fn undeclared_stdlib_reference_is_reported() {
    let mut units = stdlib_units();
    units.push(SourceFile {
        name: "main.cmajor".to_string(),
        source: indoc::indoc! {"
            processor P
            {
                output stream float out;
                void main()
                {
                    let x = teststd::osc::bogus (1.0f);
                }
            }
        "}
        .to_string(),
    });

    let program = cmajor_lang::compile::compile(units);

    assert_eq!(program.resolution.diagnostics.len(), 1);
    assert!(
        program.resolution.diagnostics[0]
            .message
            .contains("undeclared identifier 'bogus'")
    );
}

#[test]
fn compiling_with_no_standard_library_units_still_works() {
    let units = [SourceFile {
        name: "main.cmajor".to_string(),
        source: "processor P { output stream int out; void main() {} }\n".to_string(),
    }];

    let program = cmajor_lang::compile::compile(units);

    assert!(program.resolution.diagnostics.is_empty());
}
