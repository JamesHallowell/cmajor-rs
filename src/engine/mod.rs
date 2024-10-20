//! The Cmajor engine for compiling programs.

mod annotation;
mod externals;
mod program_details;

use {
    crate::{
        endpoint::{EndpointHandle, EndpointInfo},
        ffi::EnginePtr,
        performer::{Endpoint, EndpointError, EndpointType, Performer},
        program::Program,
    },
    std::{
        borrow::Cow,
        collections::HashMap,
        ffi::{CStr, CString},
        slice::Split,
    },
};
pub use {annotation::Annotation, externals::Externals, program_details::ProgramDetails};

/// The set of supported engine types.
pub struct EngineTypes<'a> {
    engine_types: Split<'a, u8, fn(&u8) -> bool>,
}

impl<'a> EngineTypes<'a> {
    pub(crate) fn new(engine_types: &'a CStr) -> Self {
        Self {
            engine_types: engine_types.to_bytes().split(|&b| b == b' '),
        }
    }
}

impl<'a> Iterator for EngineTypes<'a> {
    type Item = EngineType;

    fn next(&mut self) -> Option<Self::Item> {
        self.engine_types
            .next()
            .map(String::from_utf8_lossy)
            .map(Cow::into_owned)
            .map(EngineType)
    }
}

/// An engine type.
#[derive(Clone)]
pub struct EngineType(String);

impl EngineType {
    pub(crate) fn to_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn default_engine_type() -> Self {
        // Empty string is the default engine type.
        Self(String::new())
    }
}

impl PartialEq<str> for EngineType {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl std::fmt::Debug for EngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A builder for a [`Engine`].
pub struct EngineBuilder {
    pub(crate) sample_rate: f64,
    pub(crate) engine: Engine<Idle>,
}

impl EngineBuilder {
    /// Set the sample rate (in Hertz) to use.
    pub fn with_sample_rate(mut self, sample_rate: f64) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    /// Build the engine.
    pub fn build(self) -> Engine {
        let Self {
            sample_rate,
            engine,
        } = self;

        let build_settings = CString::new(
            serde_json::json!(
                {
                    "frequency": sample_rate
                }
            )
            .to_string(),
        )
        .expect("failed to convert build settings to C string");

        engine.inner.set_build_settings(build_settings.as_c_str());
        engine
    }
}

/// A Cmajor engine.
#[derive(Debug)]
pub struct Engine<State = Idle> {
    inner: EnginePtr,
    state: State,
}

/// An error from the engine.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The engine failed to load the program.
    #[error("Failed to load program: {:#?}", .0)]
    FailedToLoad(Engine<Idle>, String),

    /// The engine failed to link the program.
    #[error("Failed to link program: {:#?}", .0)]
    FailedToLink(Engine<Loaded>, String),
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Idle;

#[doc(hidden)]
#[derive(Debug)]
pub struct Loaded {
    program_details: ProgramDetails,
    endpoints: HashMap<EndpointHandle, EndpointInfo>,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct Linked {
    endpoints: HashMap<EndpointHandle, EndpointInfo>,
}

impl Engine<Idle> {
    pub(crate) fn new(engine: EnginePtr) -> Self {
        Self {
            inner: engine,
            state: Idle,
        }
    }

    /// Load a program into the engine.
    pub fn load(self, program: &Program) -> Result<Engine<Loaded>, Error> {
        self.load_with_externals(program, Externals::default())
    }

    /// Load a program into the engine and resolve external definitions.
    pub fn load_with_externals(
        self,
        program: &Program,
        externals: Externals,
    ) -> Result<Engine<Loaded>, Error> {
        match self.inner.load(&program.inner, externals) {
            Ok(_) => {
                let program_details = self
                    .inner
                    .program_details()
                    .expect("failed to get program details");

                let program_details = program_details.to_string();
                let program_details = serde_json::from_str(program_details.as_ref())
                    .expect("failed to parse program details");

                Ok(Engine {
                    inner: self.inner,
                    state: Loaded {
                        program_details,
                        endpoints: HashMap::default(),
                    },
                })
            }
            Err(error) => Err(Error::FailedToLoad(self, error.to_string().into_owned())),
        }
    }
}

impl Engine<Loaded> {
    /// Returns an endpoint handle.
    pub fn endpoint<T>(&mut self, id: impl AsRef<str>) -> Result<Endpoint<T>, EndpointError>
    where
        T: EndpointType,
    {
        let id = id.as_ref();

        let info = self
            .state
            .program_details
            .endpoints()
            .find(|endpoint| endpoint.id() == id)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        let id = CString::new(id).expect("invalid endpoint id");
        let handle = self
            .inner
            .get_endpoint_handle(id.as_c_str())
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        self.state.endpoints.insert(handle, info.clone());

        EndpointType::make(handle, info)
    }

    /// Returns the details of the program loaded into the engine.
    pub fn program_details(&self) -> &ProgramDetails {
        &self.state.program_details
    }

    /// Link the program loaded into the engine.
    pub fn link(self) -> Result<Engine<Linked>, Error> {
        match self.inner.link() {
            Ok(_) => {
                let linked = Linked {
                    endpoints: self.state.endpoints,
                };
                Ok(Engine {
                    inner: self.inner,
                    state: linked,
                })
            }
            Err(error) => Err(Error::FailedToLink(self, error.to_string().into_owned())),
        }
    }
}

impl Engine<Linked> {
    /// Create a performer for the linked program.
    pub fn performer(&self) -> Performer {
        Performer::new(self.inner.create_performer(), self.state.endpoints.clone())
    }
}

impl<T> Engine<T> {
    /// Unload the program, resetting the engine.
    pub fn unload(self) -> Engine<Idle> {
        self.inner.unload();

        Engine {
            inner: self.inner,
            state: Idle,
        }
    }
}
