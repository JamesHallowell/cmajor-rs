pub mod event;
pub mod stream;
pub mod value;

use {
    crate::{
        endpoint::{EndpointHandle, EndpointInfo},
        performer::EndpointError,
    },
    std::{fmt, marker::PhantomData},
};

/// An endpoint of a program, bound as a kind of endpoint the program was checked to have.
pub struct Endpoint<K: EndpointKind> {
    handle: EndpointHandle,
    runtime_type: K::RuntimeType,
    _kind: PhantomData<fn() -> K>,
}

impl<K: EndpointKind> Endpoint<K> {
    pub(crate) fn new(handle: EndpointHandle, runtime_type: K::RuntimeType) -> Self {
        Self {
            handle,
            runtime_type,
            _kind: PhantomData,
        }
    }
}

impl<K: EndpointKind> Clone for Endpoint<K> {
    fn clone(&self) -> Self {
        Self::new(self.handle, self.runtime_type.clone())
    }
}

impl<K: EndpointKind> Copy for Endpoint<K> where K::RuntimeType: Copy {}

impl<K: EndpointKind> fmt::Debug for Endpoint<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Endpoint")
            .field("handle", &self.handle)
            .field("runtime_type", &self.runtime_type)
            .finish()
    }
}

#[doc(hidden)]
pub trait EndpointKind: sealed::Sealed {
    /// Any runtime type information required. May be `()` when the type is known at compile time.
    type RuntimeType: Clone + fmt::Debug;

    /// Attempt to bind the endpoint as this kind.
    fn bind(info: &EndpointInfo) -> Result<Self::RuntimeType, EndpointError>;
}

mod sealed {
    pub trait Sealed {}
}

#[cfg(test)]
mod test {
    use crate::endpoint::EndpointId;

    pub fn id(name: &str) -> EndpointId {
        serde_json::from_value(serde_json::json!(name)).unwrap()
    }
}
