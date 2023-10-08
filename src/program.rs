use crate::ffi::ProgramPtr;

pub struct Program {
    pub(crate) inner: ProgramPtr,
}

#[derive(thiserror::Error, Debug)]
pub enum ProgramError {
    #[error("Failed to parse program {:?}", .0)]
    FailedToParse(String),
}

impl Program {
    pub(crate) fn parse(&mut self, program: impl AsRef<str>) -> Result<(), ProgramError> {
        self.inner
            .parse(None, program.as_ref())
            .map_err(|error| ProgramError::FailedToParse(error.to_string().into_owned()))
    }
}
