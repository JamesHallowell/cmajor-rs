use {
    crate::{
        endpoint::{EndpointDetails, EndpointDirection, EndpointId, EndpointType},
        ffi::{CmajorStringPtr, EnginePtr},
        performer::PerformerBuilder,
        program::Program,
        types::CmajorType,
    },
    serde::{Deserialize, Serialize},
    serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue},
    std::{
        borrow::Cow,
        collections::HashMap,
        ffi::{CStr, CString},
        slice::Split,
        sync::Arc,
    },
};

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

pub struct EngineBuilder {
    pub(crate) build_settings: JsonMap<String, JsonValue>,
    pub(crate) engine: Engine<Idle>,
}

impl EngineBuilder {
    pub fn with_sample_rate(mut self, sample_rate: impl Into<JsonNumber>) -> Self {
        self.build_settings.insert(
            "frequency".to_string(),
            JsonValue::Number(sample_rate.into()),
        );
        self
    }

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

#[derive(Debug)]
pub struct Engine<State = Idle> {
    inner: EnginePtr,
    _state: State,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to load program: {:#?}", .0)]
    FailedToLoad(Engine<Idle>, String),

    #[error("Failed to link program: {:#?}", .0)]
    FailedToLink(Engine<Loaded>, String),
}

#[derive(Debug)]
pub struct Idle;

#[derive(Debug)]
pub struct Loaded;

#[derive(Debug)]
pub struct Linked {
    endpoints: Arc<HashMap<EndpointId, Endpoint>>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct EndpointHandle(u32);

impl From<u32> for EndpointHandle {
    fn from(handle: u32) -> Self {
        Self(handle)
    }
}

impl From<EndpointHandle> for u32 {
    fn from(handle: EndpointHandle) -> Self {
        handle.0
    }
}

#[derive(Debug)]
pub struct ProgramDetails(JsonValue);

impl ProgramDetails {
    pub fn inputs(&self) -> Vec<EndpointDetails> {
        self.0
            .get("inputs")
            .and_then(JsonValue::as_array)
            .map(|inputs| {
                inputs
                    .iter()
                    .filter_map(|input| serde_json::from_value(input.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn outputs(&self) -> Vec<EndpointDetails> {
        self.0
            .get("outputs")
            .and_then(JsonValue::as_array)
            .map(|outputs| {
                outputs
                    .iter()
                    .filter_map(|output| serde_json::from_value(output.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct Endpoint {
    handle: EndpointHandle,
    direction: EndpointDirection,
    details: EndpointDetails,
}

impl Endpoint {
    pub fn handle(&self) -> EndpointHandle {
        self.handle
    }

    pub fn endpoint_type(&self) -> EndpointType {
        self.details.endpoint_type()
    }

    pub fn data_type_matches<T>(&self) -> bool
    where
        T: CmajorType,
    {
        self.details
            .data_type()
            .any(|data_type| data_type.data_type() == T::TYPE)
    }
}

impl Engine<Idle> {
    pub(crate) fn new(engine: EnginePtr) -> Self {
        Self {
            inner: engine,
            _state: Idle,
        }
    }

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

    pub fn program_details(&self) -> Result<ProgramDetails, serde_json::Error> {
        Ok(ProgramDetails(
            self.inner
                .program_details()
                .as_ref()
                .map(CmajorStringPtr::to_json)
                .transpose()?
                .unwrap_or_default(),
        ))
    }

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

        let inputs = program_details
            .inputs()
            .into_iter()
            .map(|endpoint| (EndpointDirection::Input, endpoint));

        let outputs = program_details
            .outputs()
            .into_iter()
            .map(|endpoint| (EndpointDirection::Output, endpoint));

        let endpoints = inputs
            .chain(outputs)
            .filter_map(|(direction, endpoint)| {
                let handle = self.get_endpoint_handle(endpoint.id().as_str())?;
                Some((
                    endpoint.id().clone(),
                    Endpoint {
                        handle,
                        details: endpoint,
                        direction,
                    },
                ))
            })
            .collect();

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
    pub fn performer(&self) -> PerformerBuilder {
        PerformerBuilder::new(
            self.inner.create_performer(),
            Arc::clone(&self._state.endpoints),
        )
    }
}

impl<T> Engine<T> {
    pub fn unload(self) -> Engine<Idle> {
        self.inner.unload();

        Engine {
            inner: self.inner,
            _state: Idle,
        }
    }
}
