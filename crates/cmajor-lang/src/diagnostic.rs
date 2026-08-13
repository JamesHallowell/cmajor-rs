use crate::{
    lexer::{TokenId, TokenStream},
    utils::source::{Source, SourceLocation},
};

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub token: TokenId,
    pub location: SourceLocation,
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
        let location = Source::new(source).location(span);
        Self {
            token,
            location: location.start,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}
