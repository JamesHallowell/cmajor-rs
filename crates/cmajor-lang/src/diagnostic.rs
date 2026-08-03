use crate::{
    lexer::{TokenId, TokenStream},
    utils::{self, Column, Line},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub token: TokenId,
    pub line: Line,
    pub column: Column,
    pub message: String,
}

impl Diagnostic {
    pub fn at_token(
        source: &str,
        token_stream: &TokenStream,
        token: TokenId,
        message: impl Into<String>,
    ) -> Self {
        let span = token_stream.span(token);
        let (line, column) = utils::line_col(source, span.start);
        Self {
            token,
            line,
            column,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}
