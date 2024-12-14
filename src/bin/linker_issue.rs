use cmajor::Cmajor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // see https://github.com/cmajor-lang/cmajor/issues/84
    let code = r#"
        processor Repro {
            input value float in;
            output value float out;
            void main() {
                loop {
                    out <- in ** in;
                    advance();
                }
            }
        }
    "#;

    let cmajor = Cmajor::new_from_env()?;

    let program = cmajor.parse(code)?;
    let _ = cmajor
        .create_default_engine()
        .build()
        .load(&program)?
        .link()?;

    Ok(())
}
