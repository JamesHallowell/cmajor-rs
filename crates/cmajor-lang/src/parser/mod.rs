#[allow(clippy::module_inception)]
mod parser;
mod precedence;

pub use parser::parse;
