pub mod input_event;
pub mod input_value;
pub mod output_value;
pub mod stream;

/// An endpoint.
pub struct Endpoint<T> {
    inner: T,
}
