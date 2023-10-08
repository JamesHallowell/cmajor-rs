use {
    crate::{
        engine::program_details::EndpointDetails, ffi::EnginePtr, performer::PerformerBuilder,
        program::Program, value::Type,
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

mod program_details;
pub use program_details::{EndpointId, EndpointType, ProgramDetails};

use crate::{EndpointHandles, Performer};

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
    endpoints: Arc<Endpoints>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
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
pub struct Endpoint {
    id: EndpointId,
    endpoint_type: EndpointType,
    value_type: Vec<Type>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct EndpointTypeIndex(u32);

impl From<u32> for EndpointTypeIndex {
    fn from(index: u32) -> Self {
        Self(index)
    }
}

impl From<EndpointTypeIndex> for u32 {
    fn from(index: EndpointTypeIndex) -> Self {
        index.0
    }
}

impl Endpoint {
    fn new(detail: EndpointDetails) -> Self {
        Self {
            id: detail.id,
            endpoint_type: detail.endpoint_type,
            value_type: detail.value_type,
        }
    }

    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    pub fn endpoint_type(&self) -> EndpointType {
        self.endpoint_type
    }

    pub fn value_type(&self) -> &[Type] {
        self.value_type.as_slice()
    }

    pub fn index_of_value_type(&self, ty: &Type) -> Option<EndpointTypeIndex> {
        self.value_type
            .iter()
            .position(|t| t == ty)
            .map(|index| index as u32)
            .map(EndpointTypeIndex)
    }

    pub fn value_type_at_index(&self, index: EndpointTypeIndex) -> Option<&Type> {
        self.value_type.get(index.0 as usize)
    }
}

#[derive(Debug)]
pub struct Endpoints {
    inputs: HashMap<EndpointHandle, Endpoint>,
    outputs: HashMap<EndpointHandle, Endpoint>,
}

impl Endpoints {
    pub fn get_input(&self, handle: EndpointHandle) -> Option<&Endpoint> {
        self.inputs.get(&handle)
    }

    pub fn get_output(&self, handle: EndpointHandle) -> Option<&Endpoint> {
        self.outputs.get(&handle)
    }

    fn find_matching<'a>(
        id: impl AsRef<str>,
    ) -> impl Fn((&EndpointHandle, &'a Endpoint)) -> Option<EndpointHandle> {
        move |(handle, endpoint): (&EndpointHandle, &Endpoint)| {
            (endpoint.id().as_ref() == id.as_ref()).then_some(*handle)
        }
    }

    pub fn get_input_by_id(&self, id: impl AsRef<str>) -> Option<(EndpointHandle, &Endpoint)> {
        self.inputs
            .iter()
            .find_map(Self::find_matching(id))
            .and_then(|handle| self.get_input(handle).map(|endpoint| (handle, endpoint)))
    }

    pub fn get_output_by_id(&self, id: impl AsRef<str>) -> Option<(EndpointHandle, &Endpoint)> {
        self.outputs
            .iter()
            .find_map(Self::find_matching(id))
            .and_then(|handle| self.get_output(handle).map(|endpoint| (handle, endpoint)))
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
        let program_details = self
            .inner
            .program_details()
            .expect("failed to get program details");

        serde_json::from_str(program_details.to_string().as_ref())
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
            .inputs
            .into_iter()
            .map(|endpoint| {
                let handle = self.get_endpoint_handle(&endpoint.id).unwrap();
                (handle, Endpoint::new(endpoint))
            })
            .collect();

        let outputs = program_details
            .outputs
            .into_iter()
            .map(|endpoint| {
                let handle = self.get_endpoint_handle(&endpoint.id).unwrap();
                (handle, Endpoint::new(endpoint))
            })
            .collect();

        match self.inner.link() {
            Ok(_) => {
                let linked = Linked {
                    endpoints: Arc::new(Endpoints { inputs, outputs }),
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
    pub fn performer(&self) -> (Performer, EndpointHandles) {
        PerformerBuilder::new(
            self.inner.create_performer(),
            Arc::clone(&self._state.endpoints),
        )
        .build()
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
