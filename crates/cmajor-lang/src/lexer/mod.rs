mod cursor;
#[allow(clippy::module_inception)]
mod lexer;
mod token;
mod token_stream;

pub use {
    lexer::tokenize,
    token::{Keyword, Literal, Token, TokenKind, Trivia},
    token_stream::{TokenId, TokenStream, TokenStreamIterator},
};
