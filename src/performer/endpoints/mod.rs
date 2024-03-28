mod input_event;
mod input_value;
mod output_value;

/// An endpoint.
pub struct Endpoint<T> {
    inner: T,
}

pub(crate) use input_value::CachedInputValues;
pub use {input_event::InputEvent, input_value::InputValue, output_value::OutputValue};
