use {
    crate::value::types::Type,
    serde::{Deserialize, Serialize},
    std::{borrow::Borrow, collections::HashMap},
};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
pub struct EndpointId(String);

impl AsRef<str> for EndpointId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for EndpointId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for EndpointId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
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
pub enum Endpoint {
    Stream(StreamEndpoint),
    Event(EventEndpoint),
    Value(ValueEndpoint),
}

#[derive(Debug)]
pub struct StreamEndpoint {
    id: EndpointId,
    ty: Type,
}

impl From<StreamEndpoint> for Endpoint {
    fn from(endpoint: StreamEndpoint) -> Self {
        Self::Stream(endpoint)
    }
}

#[derive(Debug)]
pub struct EventEndpoint {
    id: EndpointId,
    ty: Vec<Type>,
}

impl From<EventEndpoint> for Endpoint {
    fn from(endpoint: EventEndpoint) -> Self {
        Self::Event(endpoint)
    }
}

#[derive(Debug)]
pub struct ValueEndpoint {
    id: EndpointId,
    ty: Type,
}

impl From<ValueEndpoint> for Endpoint {
    fn from(endpoint: ValueEndpoint) -> Self {
        Self::Value(endpoint)
    }
}

impl Endpoint {
    pub fn id(&self) -> &EndpointId {
        match self {
            Self::Stream(endpoint) => &endpoint.id,
            Self::Event(endpoint) => &endpoint.id,
            Self::Value(endpoint) => &endpoint.id,
        }
    }
}

impl ValueEndpoint {
    pub(crate) fn new(id: EndpointId, ty: Type) -> Self {
        Self { id, ty }
    }

    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }
}

impl StreamEndpoint {
    pub(crate) fn new(id: EndpointId, ty: Type) -> Self {
        Self { id, ty }
    }

    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }
}

impl EventEndpoint {
    pub(crate) fn new(id: EndpointId, ty: Vec<Type>) -> Self {
        assert!(!ty.is_empty());
        Self { id, ty }
    }

    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    pub fn types(&self) -> &[Type] {
        &self.ty
    }

    pub fn type_index(&self, ty: &Type) -> Option<EndpointTypeIndex> {
        self.ty
            .iter()
            .position(|t| t == ty)
            .map(|index| index as u32)
            .map(EndpointTypeIndex::from)
    }

    pub fn get_type(&self, index: EndpointTypeIndex) -> Option<&Type> {
        self.ty.get(index.0 as usize)
    }
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

#[derive(Debug, Default)]
pub struct Endpoints {
    inputs: HashMap<EndpointHandle, Endpoint>,
    outputs: HashMap<EndpointHandle, Endpoint>,
}

impl Endpoints {
    pub fn with_inputs(
        mut self,
        inputs: impl IntoIterator<Item = (EndpointHandle, Endpoint)>,
    ) -> Self {
        self.inputs = inputs.into_iter().collect();
        self
    }

    pub fn with_outputs(
        mut self,
        outputs: impl IntoIterator<Item = (EndpointHandle, Endpoint)>,
    ) -> Self {
        self.outputs = outputs.into_iter().collect();
        self
    }

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
