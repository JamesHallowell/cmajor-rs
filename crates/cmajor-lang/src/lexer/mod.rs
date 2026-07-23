mod cursor;
#[allow(clippy::module_inception)]
mod lexer;
mod token;

pub use lexer::tokenize;
pub use token::{SyntaxKind, Token};
