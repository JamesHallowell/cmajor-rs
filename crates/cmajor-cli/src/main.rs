mod io;
mod parse;
mod stdlib;
mod test;
mod tokenize;

use {
    clap::{Parser, Subcommand},
    std::{path::PathBuf, process::ExitCode},
};

#[derive(Parser)]
#[command(name = "cmajc", about = "Command-line tools for cmajor-lang")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a .cmajor file (or recursively parse every .cmajor file under a directory) and
    /// print the resulting AST.
    Parse {
        path: PathBuf,
        #[arg(short, long)]
        verbose: bool,
        #[arg(long)]
        stdlib: Option<PathBuf>,
    },
    /// Tokenize a .cmajor file and print its token stream.
    Tokenize { path: PathBuf },
    /// Run .cmajtest files (or every .cmajtest/.cmajor file under a directory).
    Test {
        path: PathBuf,
        #[arg(long)]
        stdlib: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let success = match cli.command {
        Command::Parse {
            path,
            verbose,
            stdlib,
        } => parse::run(&path, stdlib.as_deref(), verbose),
        Command::Tokenize { path } => tokenize::run(&path),
        Command::Test { path, stdlib } => test::run(&path, stdlib.as_deref()),
    };

    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
