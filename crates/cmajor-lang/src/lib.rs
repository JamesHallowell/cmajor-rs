pub mod ast;
pub mod compile;
mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod resolver;
pub mod test_format;
pub mod utils;

pub use {
    compile::{ParsedUnit, Program},
    diagnostic::Diagnostic,
    utils::source::SourceFile,
};
