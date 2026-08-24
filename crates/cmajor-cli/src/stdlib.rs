use {
    crate::io,
    cmajor_lang::compile::SourceFile,
    std::path::{Path, PathBuf},
};

pub fn collect(dir: &Path) -> impl Iterator<Item = SourceFile> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|err| {
            eprintln!("failed to read stdlib directory {}: {err}", dir.display());
            std::process::exit(1);
        })
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "cmajor"))
        .collect();
    files.sort();

    files.into_iter().map(|path| {
        let source = io::read_file(&path);
        SourceFile {
            name: path.display().to_string(),
            source,
        }
    })
}
