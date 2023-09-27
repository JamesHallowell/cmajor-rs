use {crate::ffi::ProgramPtr, serde_json::Value};

pub struct Program {
    pub(crate) inner: ProgramPtr,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to parse program {:?}", .0)]
    Parse(String),
}

impl Program {
    pub fn parse(&mut self, program: impl AsRef<str>) -> Result<(), Error> {
        self.inner
            .parse(None, program.as_ref())
            .map_err(|error| Error::Parse(error.to_string().into_owned()))
    }

    pub fn syntax_tree(&self) -> Result<Value, serde_json::Error> {
        self.inner.syntax_tree().to_json()
    }
}
