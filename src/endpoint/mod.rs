//! Endpoints for passing data between a program and its host.

use {
    crate::{
        engine::Annotation,
        value::types::{Type, TypeRef},
    },
    serde::{Deserialize, Serialize},
    std::{borrow::Borrow, collections::HashMap},
};

/// An endpoint identifier.
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

/// A handle used to reference an endpoint.
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

/// An endpoint.
#[derive(Debug)]
pub enum Endpoint {
    /// A stream endpoint.
    Stream(StreamEndpoint),

    /// An event endpoint.
    Event(EventEndpoint),

    /// A value endpoint.
    Value(ValueEndpoint),
}

/// A stream endpoint.
#[derive(Debug)]
pub struct StreamEndpoint {
    id: EndpointId,
    ty: Type,
    annotation: Annotation,
}

impl From<StreamEndpoint> for Endpoint {
    fn from(endpoint: StreamEndpoint) -> Self {
        Self::Stream(endpoint)
    }
}

/// An event endpoint.
#[derive(Debug)]
pub struct EventEndpoint {
    id: EndpointId,
    ty: Vec<Type>,
    annotation: Annotation,
}

impl From<EventEndpoint> for Endpoint {
    fn from(endpoint: EventEndpoint) -> Self {
        Self::Event(endpoint)
    }
}

/// A value endpoint.
#[derive(Debug)]
pub struct ValueEndpoint {
    id: EndpointId,
    ty: Type,
    annotation: Annotation,
}

impl From<ValueEndpoint> for Endpoint {
    fn from(endpoint: ValueEndpoint) -> Self {
        Self::Value(endpoint)
    }
}

impl Endpoint {
    /// The endpoint's identifier (or name).
    pub fn id(&self) -> &EndpointId {
        match self {
            Self::Stream(endpoint) => &endpoint.id,
            Self::Event(endpoint) => &endpoint.id,
            Self::Value(endpoint) => &endpoint.id,
        }
    }

    /// The endpoint's annotation.
    pub fn annotation(&self) -> &Annotation {
        match self {
            Self::Stream(endpoint) => &endpoint.annotation,
            Self::Event(endpoint) => &endpoint.annotation,
            Self::Value(endpoint) => &endpoint.annotation,
        }
    }
}

impl ValueEndpoint {
    pub(crate) fn new(id: EndpointId, ty: Type, annotation: Annotation) -> Self {
        Self { id, ty, annotation }
    }

    /// The endpoint's identifier (or name).
    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    /// The type of the endpoint's value.
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    /// The endpoint's annotation.
    pub fn annotation(&self) -> &Annotation {
        &self.annotation
    }
}

impl StreamEndpoint {
    pub(crate) fn new(id: EndpointId, ty: Type, annotation: Annotation) -> Self {
        Self { id, ty, annotation }
    }

    /// The endpoint's identifier (or name).
    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    /// The type of the endpoint's value.
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    /// The endpoint's annotation.
    pub fn annotation(&self) -> &Annotation {
        &self.annotation
    }
}

impl EventEndpoint {
    pub(crate) fn new(id: EndpointId, ty: Vec<Type>, annotation: Annotation) -> Self {
        assert!(!ty.is_empty());
        Self { id, ty, annotation }
    }

    /// The endpoint's identifier (or name).
    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    /// The types of the endpoint's events.
    pub fn types(&self) -> &[Type] {
        &self.ty
    }

    /// The endpoint's annotation.
    pub fn annotation(&self) -> &Annotation {
        &self.annotation
    }

    /// The index of the given type in the endpoint's type list.
    pub fn type_index(&self, ty: TypeRef<'_>) -> Option<EndpointTypeIndex> {
        self.ty
            .iter()
            .position(|t| t.as_ref() == ty)
            .map(|index| index as u32)
            .map(EndpointTypeIndex::from)
    }

    /// The type at the given index in the endpoint's type list.
    pub fn get_type(&self, index: EndpointTypeIndex) -> Option<&Type> {
        self.ty.get(index.0 as usize)
    }
}

/// An index into an event endpoint's type list.
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

/// A collection of endpoints.
#[derive(Debug)]
pub struct Endpoints {
    inputs: HashMap<EndpointHandle, Endpoint>,
    outputs: HashMap<EndpointHandle, Endpoint>,
}

impl Endpoints {
    pub(crate) fn new() -> Self {
        Self {
            inputs: HashMap::default(),
            outputs: HashMap::default(),
        }
    }

    pub(crate) fn with_inputs(
        mut self,
        inputs: impl IntoIterator<Item = (EndpointHandle, Endpoint)>,
    ) -> Self {
        self.inputs = inputs.into_iter().collect();
        self
    }

    pub(crate) fn with_outputs(
        mut self,
        outputs: impl IntoIterator<Item = (EndpointHandle, Endpoint)>,
    ) -> Self {
        self.outputs = outputs.into_iter().collect();
        self
    }

    /// Get an input endpoint by its handle.
    pub fn get_input(&self, handle: EndpointHandle) -> Option<&Endpoint> {
        self.inputs.get(&handle)
    }

    /// Get an output endpoint by its handle.
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

    /// Get an input endpoint by its identifier.
    pub fn get_input_by_id(&self, id: impl AsRef<str>) -> Option<(EndpointHandle, &Endpoint)> {
        self.inputs
            .iter()
            .find_map(Self::find_matching(id))
            .and_then(|handle| self.get_input(handle).map(|endpoint| (handle, endpoint)))
    }

    /// Get an output endpoint by its identifier.
    pub fn get_output_by_id(&self, id: impl AsRef<str>) -> Option<(EndpointHandle, &Endpoint)> {
        self.outputs
            .iter()
            .find_map(Self::find_matching(id))
            .and_then(|handle| self.get_output(handle).map(|endpoint| (handle, endpoint)))
    }
}
