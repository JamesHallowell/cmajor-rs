pub mod event;
pub mod stream;
pub mod value;

/// An endpoint.
#[derive(Debug, Copy, Clone)]
pub struct Endpoint<T>(pub(crate) T);
