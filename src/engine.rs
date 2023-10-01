use {
    crate::{
        ffi::{CmajorStringPtr, EnginePtr},
        performer::Performer,
        program::Program,
    },
    serde_json::{Map, Number, Value},
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        slice::Split,
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
    pub(crate) build_settings: Map<String, Value>,
    pub(crate) engine: Engine<Idle>,
}

impl EngineBuilder {
    pub fn with_sample_rate(mut self, sample_rate: impl Into<Number>) -> Self {
        self.build_settings
            .insert("frequency".to_string(), Value::Number(sample_rate.into()));
        self
    }

    pub fn build(self) -> Engine {
        let Self {
            build_settings,
            engine,
        } = self;

        let build_settings =
            serde_json::to_string(&Value::Object(build_settings)).expect("valid json");
        let build_settings =
            CString::new(build_settings).expect("failed to convert build settings JSON to CString");

        engine.inner.set_build_settings(build_settings.as_c_str());
        engine
    }
}

pub struct EndpointHandle(pub(crate) u32);

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
pub struct Linked;

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
    pub fn get_endpoint_handle(&self, id: impl AsRef<str>) -> Option<EndpointHandle> {
        let id = CString::new(id.as_ref()).ok()?;

        self.inner
            .get_endpoint_handle(id.as_c_str())
            .map(EndpointHandle)
    }

    pub fn program_details(&self) -> Result<Value, serde_json::Error> {
        Ok(self
            .inner
            .program_details()
            .as_ref()
            .map(CmajorStringPtr::to_json)
            .transpose()?
            .unwrap_or_default())
    }

    pub fn link(self) -> Result<Engine<Linked>, Error> {
        match self.inner.link() {
            Ok(_) => Ok(Engine {
                inner: self.inner,
                _state: Linked,
            }),
            Err(error) => Err(Error::FailedToLink(self, error.to_string().into_owned())),
        }
    }
}

impl Engine<Linked> {
    pub fn create_performer(&self) -> Performer {
        self.inner.create_performer().into()
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
