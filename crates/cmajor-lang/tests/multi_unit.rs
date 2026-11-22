use cmajor_lang::{CompileError, Program, SourceFile};

fn stdlib_units() -> impl IntoIterator<Item = SourceFile> {
    [
        SourceFile::new(indoc::indoc! {"
                namespace teststd::osc
                {
                    float sine (float phase)
                    {
                        return phase;
                    }
                }
            "})
        .with_name("oscillator.cmajor"),
        SourceFile::new(indoc::indoc! {"
                namespace teststd::env
                {
                    float attack (float level)
                    {
                        return level;
                    }
                }
            "})
        .with_name("envelope.cmajor"),
    ]
}

#[test]
fn compiles_a_program_against_a_synthetic_standard_library() {
    let program: Program = stdlib_units()
        .into_iter()
        .chain(std::iter::once(
            SourceFile::new(indoc::indoc! {"
            processor P
            {
                output stream float out;
                void main()
                {
                    let x = teststd::osc::sine (1.0f);
                    let y = teststd::env::attack (1.0f);
                }
            }
        "})
            .with_name("main.cmajor"),
        ))
        .collect();

    assert_eq!(
        program.diagnostics().count(),
        0,
        "expected no diagnostics, got: {:?}",
        program
            .diagnostics()
            .map(|(_, diagnostic)| diagnostic.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn undeclared_stdlib_reference_is_reported() {
    let program: Program = stdlib_units()
        .into_iter()
        .chain(std::iter::once(
            SourceFile::new(indoc::indoc! {"
                processor P
                {
                    output stream float out;
                    void main()
                    {
                        let x = teststd::osc::bogus (1.0f);
                    }
                }
            "})
            .with_name("main.cmajor"),
        ))
        .collect();

    let resolution = cmajor_lang::resolver::resolve(&program);

    let errors: Vec<_> = resolution
        .diagnostics
        .into_iter()
        .map(|(_, diagnostic)| diagnostic.error)
        .collect();
    assert_eq!(
        errors,
        vec![CompileError::UnresolvedSymbol {
            name: "teststd::osc::bogus".into()
        }]
    );
}

#[test]
fn compiling_with_no_standard_library_units_still_works() {
    let program: Program = std::iter::once(
        SourceFile::new("processor P { output stream int out; void main() {} }\n")
            .with_name("main.cmajor"),
    )
    .collect();

    let resolution = cmajor_lang::resolver::resolve(&program);

    assert!(resolution.diagnostics.is_empty());
}
