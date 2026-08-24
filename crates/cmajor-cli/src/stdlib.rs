use {
    crate::io,
    cmajor_lang::compile::SourceFile,
    std::path::{Path, PathBuf},
};

pub fn collect(dir: &Path) -> impl Iterator<Item = SourceFile> {
    let mut files = Vec::new();
    collect_into(dir, &mut files);
    files.sort();

    files.into_iter().map(|path| {
        let source = io::read_file(&path);
        SourceFile {
            name: path.display().to_string(),
            source,
        }
    })
}

fn collect_into(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|err| {
        eprintln!("failed to read stdlib directory {}: {err}", dir.display());
        std::process::exit(1);
    });

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            collect_into(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "cmajor") {
            files.push(path);
        }
    }
}
