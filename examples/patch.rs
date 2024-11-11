use {
    cmajor::Cmajor,
    std::{env, fs},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmajor = Cmajor::new_from_env()?;

    let path_to_program = env::args().nth(1).expect("path to program file required");

    let program = cmajor.parse(fs::read_to_string(path_to_program)?)?;

    let _ = cmajor
        .create_default_engine()
        .build()
        .load(&program)?
        .link()?;

    Ok(())
}
