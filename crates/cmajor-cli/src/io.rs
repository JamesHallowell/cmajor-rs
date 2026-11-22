use std::path::{Path, PathBuf};

pub fn read_file(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("failed to read {}: {err}", path.display());
        std::process::exit(1);
    })
}

pub fn read_file_or_directory(path: &Path) -> String {
    if !path.is_dir() {
        return read_file(path);
    }

    let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
        .unwrap_or_else(|err| {
            eprintln!("failed to read {}: {err}", path.display());
            std::process::exit(1);
        })
        .map(|entry| {
            entry.map(|entry| entry.path()).unwrap_or_else(|err| {
                eprintln!("failed to read {}: {err}", path.display());
                std::process::exit(1);
            })
        })
        .collect();
    entries.sort();

    entries
        .iter()
        .map(|entry| read_file_or_directory(entry))
        .collect()
}
