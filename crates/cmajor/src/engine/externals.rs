use {crate::value::Value, std::collections::HashMap};

/// Externals definitions for a Cmajor program.
#[derive(Debug, Default)]
pub struct Externals {
    pub(crate) variables: HashMap<String, Value>,
}

impl Externals {
    /// Define an external variable that will be loaded into the engine.
    pub fn set_variable(&mut self, name: impl AsRef<str>, value: impl Into<Value>) {
        self.variables
            .insert(name.as_ref().to_string(), value.into());
    }

    /// Define an external variable that will be loaded into the engine.
    pub fn with_variable(mut self, name: impl AsRef<str>, value: impl Into<Value>) -> Self {
        self.set_variable(name, value);
        self
    }
}
