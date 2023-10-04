use crate::ffi::ProgramPtr;

pub struct Program {
    pub(crate) inner: ProgramPtr,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to parse program {:?}", .0)]
    Parse(String),
}

impl Program {
    pub(crate) fn parse(&mut self, program: impl AsRef<str>) -> Result<(), Error> {
        self.inner
            .parse(None, program.as_ref())
            .map_err(|error| Error::Parse(error.to_string().into_owned()))
    }
}
