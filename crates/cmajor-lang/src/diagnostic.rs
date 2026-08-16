use crate::utils::{source::SourceLocation, span::Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub location: Span<SourceLocation>,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.location.start, self.message)
    }
}
