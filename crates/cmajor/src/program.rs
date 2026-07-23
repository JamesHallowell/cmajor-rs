use crate::{diagnostic::DiagnosticMessage, ffi::ProgramPtr};

/// A Cmajor program.
#[derive(Debug)]
pub struct Program {
    pub(crate) inner: ProgramPtr,
}

/// An error that can occur when parsing a Cmajor program.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// An error occurred while parsing a program.
    #[error("Error parsing program: {0:?}")]
    ParserError(Box<DiagnosticMessage>),

    /// An error occurred whilst parsing the error from the library.
    #[error(transparent)]
    FailedToParseError(#[from] serde_json::Error),
}

impl Program {
    pub(crate) fn parse(&mut self, program: impl AsRef<str>) -> Result<(), ParseError> {
        let file_name: Option<&str> = None;

        match self.inner.parse(file_name, program) {
            Ok(()) => Ok(()),
            Err(error) => {
                let parser_error = serde_json::from_str(error.to_str())?;
                Err(ParseError::ParserError(Box::new(parser_error)))
            }
        }
    }
}
