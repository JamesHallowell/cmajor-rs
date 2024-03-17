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
pub struct EndpointHandle(pub(crate) u32);

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
pub enum EndpointType {
    /// A stream endpoint.
    Stream(StreamEndpoint),

    /// An event endpoint.
    Event(EventEndpoint),

    /// A value endpoint.
    Value(ValueEndpoint),
}

/// The direction of an endpoint.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EndpointDirection {
    /// An input endpoint.
    Input,

    /// An output endpoint.
    Output,
}

/// A stream endpoint.
#[derive(Debug)]
pub struct StreamEndpoint {
    id: EndpointId,
    direction: EndpointDirection,
    ty: Type,
    annotation: Annotation,
}

impl From<StreamEndpoint> for EndpointType {
    fn from(endpoint: StreamEndpoint) -> Self {
        Self::Stream(endpoint)
    }
}

/// An event endpoint.
#[derive(Debug)]
pub struct EventEndpoint {
    id: EndpointId,
    direction: EndpointDirection,
    ty: Vec<Type>,
    annotation: Annotation,
}

impl From<EventEndpoint> for EndpointType {
    fn from(endpoint: EventEndpoint) -> Self {
        Self::Event(endpoint)
    }
}

/// A value endpoint.
#[derive(Debug)]
pub struct ValueEndpoint {
    id: EndpointId,
    direction: EndpointDirection,
    ty: Type,
    annotation: Annotation,
}

impl From<ValueEndpoint> for EndpointType {
    fn from(endpoint: ValueEndpoint) -> Self {
        Self::Value(endpoint)
    }
}

impl EndpointType {
    /// The endpoint's identifier (or name).
    pub fn id(&self) -> &EndpointId {
        match self {
            Self::Stream(endpoint) => &endpoint.id,
            Self::Event(endpoint) => &endpoint.id,
            Self::Value(endpoint) => &endpoint.id,
        }
    }

    /// The endpoint's direction.
    pub fn direction(&self) -> EndpointDirection {
        match self {
            Self::Stream(endpoint) => endpoint.direction,
            Self::Event(endpoint) => endpoint.direction,
            Self::Value(endpoint) => endpoint.direction,
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

    /// Get the endpoint as a value endpoint.
    pub fn as_stream(&self) -> Option<&StreamEndpoint> {
        match self {
            Self::Stream(endpoint) => Some(endpoint),
            _ => None,
        }
    }

    /// Get the endpoint as an event endpoint.
    pub fn as_event(&self) -> Option<&EventEndpoint> {
        match self {
            Self::Event(endpoint) => Some(endpoint),
            _ => None,
        }
    }

    /// Get the endpoint as a value endpoint.
    pub fn as_value(&self) -> Option<&ValueEndpoint> {
        match self {
            Self::Value(endpoint) => Some(endpoint),
            _ => None,
        }
    }
}

impl ValueEndpoint {
    pub(crate) fn new(
        id: EndpointId,
        direction: EndpointDirection,
        ty: Type,
        annotation: Annotation,
    ) -> Self {
        Self {
            id,
            direction,
            ty,
            annotation,
        }
    }

    /// The endpoint's identifier (or name).
    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    /// The endpoint's direction.
    pub fn direction(&self) -> EndpointDirection {
        self.direction
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
    pub(crate) fn new(
        id: EndpointId,
        direction: EndpointDirection,
        ty: Type,
        annotation: Annotation,
    ) -> Self {
        Self {
            id,
            direction,
            ty,
            annotation,
        }
    }

    /// The endpoint's identifier (or name).
    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    /// The endpoint's direction.
    pub fn direction(&self) -> EndpointDirection {
        self.direction
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
    pub(crate) fn new(
        id: EndpointId,
        direction: EndpointDirection,
        ty: Vec<Type>,
        annotation: Annotation,
    ) -> Self {
        assert!(!ty.is_empty());
        Self {
            id,
            direction,
            ty,
            annotation,
        }
    }

    /// The endpoint's identifier (or name).
    pub fn id(&self) -> &EndpointId {
        &self.id
    }

    /// The endpoint's direction.
    pub fn direction(&self) -> EndpointDirection {
        self.direction
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
            .map(EndpointTypeIndex::from)
    }

    /// The type at the given index in the endpoint's type list.
    pub fn get_type(&self, index: EndpointTypeIndex) -> Option<&Type> {
        self.ty.get(usize::from(index))
    }
}

/// An index into an event endpoint's type list.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct EndpointTypeIndex(usize);

impl From<usize> for EndpointTypeIndex {
    fn from(index: usize) -> Self {
        Self(index)
    }
}

impl From<EndpointTypeIndex> for usize {
    fn from(index: EndpointTypeIndex) -> Self {
        index.0
    }
}

/// A collection of endpoints.
#[derive(Debug)]
pub struct ProgramEndpoints {
    endpoints: HashMap<EndpointHandle, EndpointType>,
    ids: HashMap<EndpointId, EndpointHandle>,
}

impl ProgramEndpoints {
    pub(crate) fn new(endpoints: impl IntoIterator<Item = (EndpointHandle, EndpointType)>) -> Self {
        let endpoints: HashMap<_, _> = endpoints.into_iter().collect();
        let ids = endpoints
            .iter()
            .map(|(handle, endpoint)| (endpoint.id().clone(), *handle))
            .collect();

        Self { endpoints, ids }
    }

    /// Get an iterator over the input endpoints.
    pub fn inputs(&self) -> impl Iterator<Item = &EndpointType> {
        self.endpoints
            .values()
            .filter(|endpoint| endpoint.direction() == EndpointDirection::Input)
    }

    /// Get an interator over the output endpoints.
    pub fn outputs(&self) -> impl Iterator<Item = &EndpointType> {
        self.endpoints
            .values()
            .filter(|endpoint| endpoint.direction() == EndpointDirection::Output)
    }

    /// Get an endpoint by its handle.
    pub fn get(&self, handle: EndpointHandle) -> Option<&EndpointType> {
        self.endpoints.get(&handle)
    }

    /// Get an endpoint by its ID.
    pub fn get_by_id(&self, id: impl AsRef<str>) -> Option<(EndpointHandle, &EndpointType)> {
        let handle = self.ids.get(id.as_ref()).copied()?;
        self.endpoints
            .get(&handle)
            .map(|endpoint| (handle, endpoint))
    }

    /// Get an endpoint's handle by its ID.
    pub fn get_handle(&self, id: impl AsRef<str>) -> Option<EndpointHandle> {
        self.ids.get(id.as_ref()).copied()
    }
}
