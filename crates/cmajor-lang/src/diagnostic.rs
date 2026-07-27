use crate::{
    lexer::TokenId,
    utils::{Column, Line},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub token: TokenId,
    pub line: Line,
    pub column: Column,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}
