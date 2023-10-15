//! The Cmajor engine for compiling programs.

mod annotation;
pub(crate) mod endpoint;
mod program_details;

use {
    crate::{
        engine::{endpoint::Endpoints, program_details::ProgramDetails},
        ffi::EnginePtr,
        performer::{Performer, PerformerHandle},
        program::Program,
    },
    serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue},
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        slice::Split,
        sync::Arc,
    },
};
pub use {
    annotation::Annotation,
    endpoint::{
        Endpoint, EndpointHandle, EndpointId, EndpointTypeIndex, EventEndpoint, StreamEndpoint,
        ValueEndpoint,
    },
};

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
    pub(crate) build_settings: JsonMap<String, JsonValue>,
    pub(crate) engine: Engine<Idle>,
}

impl EngineBuilder {
    /// Set the sample rate (in Hertz) to use.
    pub fn with_sample_rate(mut self, sample_rate: impl Into<JsonNumber>) -> Self {
        self.build_settings.insert(
            "frequency".to_string(),
            JsonValue::Number(sample_rate.into()),
        );
        self
    }

    /// Build the engine.
    pub fn build(self) -> Engine {
        let Self {
            build_settings,
            engine,
        } = self;

        let build_settings =
            serde_json::to_string(&JsonValue::Object(build_settings)).expect("valid json");
        let build_settings =
            CString::new(build_settings).expect("failed to convert build settings JSON to CString");

        engine.inner.set_build_settings(build_settings.as_c_str());
        engine
    }
}

/// A Cmajor engine.
#[derive(Debug)]
pub struct Engine<State = Idle> {
    inner: EnginePtr,
    _state: State,
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
pub struct Loaded;

#[doc(hidden)]
#[derive(Debug)]
pub struct Linked {
    endpoints: Arc<Endpoints>,
}

impl Engine<Idle> {
    pub(crate) fn new(engine: EnginePtr) -> Self {
        Self {
            inner: engine,
            _state: Idle,
        }
    }

    /// Load a program into the engine.
    pub fn load(self, program: &Program) -> Result<Engine<Loaded>, Error> {
        match self.inner.load(&program.inner) {
            Ok(_) => Ok(Engine {
                inner: self.inner,
                _state: Loaded,
            }),
            Err(error) => Err(Error::FailedToLoad(self, error.to_string().into_owned())),
        }
    }
}

impl Engine<Loaded> {
    fn get_endpoint_handle(&self, id: impl AsRef<str>) -> Option<EndpointHandle> {
        let id = CString::new(id.as_ref()).ok()?;

        self.inner.get_endpoint_handle(id.as_c_str())
    }

    /// Returns the details of the program loaded into the engine.
    pub fn program_details(&self) -> Result<ProgramDetails, serde_json::Error> {
        let program_details = self
            .inner
            .program_details()
            .expect("failed to get program details");

        serde_json::from_str(program_details.to_string().as_ref())
    }

    /// Link the program loaded into the engine.
    pub fn link(self) -> Result<Engine<Linked>, Error> {
        let program_details = match self.program_details() {
            Ok(program_details) => program_details,
            Err(error) => {
                return Err(Error::FailedToLink(
                    self,
                    format!("failed to get program details: {}", error),
                ))
            }
        };

        let inputs = program_details.inputs().map(|endpoint| {
            let handle = self.get_endpoint_handle(endpoint.id()).unwrap();
            (handle, endpoint)
        });

        let outputs = program_details.outputs().map(|endpoint| {
            let handle = self.get_endpoint_handle(endpoint.id()).unwrap();
            (handle, endpoint)
        });

        let endpoints = Endpoints::default()
            .with_inputs(inputs)
            .with_outputs(outputs);

        match self.inner.link() {
            Ok(_) => {
                let linked = Linked {
                    endpoints: Arc::new(endpoints),
                };
                Ok(Engine {
                    inner: self.inner,
                    _state: linked,
                })
            }
            Err(error) => Err(Error::FailedToLink(self, error.to_string().into_owned())),
        }
    }
}

impl Engine<Linked> {
    /// Create a performer for the linked program.
    pub fn performer(&self) -> (Performer, PerformerHandle) {
        Performer::new(
            self.inner.create_performer(),
            Arc::clone(&self._state.endpoints),
        )
    }
}

impl<T> Engine<T> {
    /// Unload the program, resetting the engine.
    pub fn unload(self) -> Engine<Idle> {
        self.inner.unload();

        Engine {
            inner: self.inner,
            _state: Idle,
        }
    }
}
