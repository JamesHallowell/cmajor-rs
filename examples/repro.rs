use cmajor::Cmajor;

const PROGRAM: &str = r#"
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

#[no_mangle]
pub extern "C" fn powf(x: f32, y: f32) -> f32 {
    x.powf(y)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmajor = Cmajor::new_from_env()?;

    let program = cmajor.parse(PROGRAM)?;
    let _ = cmajor
        .create_default_engine()
        .build()
        .load(&program)?
        .link()?;

    Ok(())
}
