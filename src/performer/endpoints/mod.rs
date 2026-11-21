mod endpoint;
pub mod event;
pub mod stream;
pub mod value;

pub use endpoint::Endpoint;
use {
    crate::{endpoint::EndpointInfo, performer::EndpointError},
    std::fmt,
};

#[doc(hidden)]
pub trait EndpointKind: sealed::Sealed {
    /// The binding required for this kind of endpoint.
    type Binding: Clone + fmt::Debug;

    /// Attempt to bind the endpoint as this kind.
    fn bind(info: &EndpointInfo) -> Result<Self::Binding, EndpointError>;
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
