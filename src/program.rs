use crate::{diagnostic::DiagnosticMessage, ffi::ProgramPtr};

#[derive(Debug)]
pub struct Program {
    pub(crate) inner: ProgramPtr,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Error parsing program: {0:?}")]
    ParserError(DiagnosticMessage),

    #[error(transparent)]
    FailedToParseError(#[from] serde_json::Error),
}

impl Program {
    pub(crate) fn parse(&mut self, program: impl AsRef<str>) -> Result<(), ParseError> {
        let result = self.inner.parse(None, program.as_ref());

        match result {
            Ok(_) => Ok(()),
            Err(error) => {
                let parser_error: DiagnosticMessage =
                    serde_json::from_str(error.to_string().as_ref())?;
                Err(ParseError::ParserError(parser_error))
            }
        }
    }
}
