use std::path::Path;

pub fn read_file(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("failed to read {}: {err}", path.display());
        std::process::exit(1);
    })
}
