//! The Cmajor performer for running programs.

mod endpoints;

pub use endpoints::{
    event::{InputEvent, OutputEvent},
    stream::{InputStream, OutputStream},
    value::{InputValue, OutputValue},
    Endpoint,
};
use {
    crate::{
        endpoint::{EndpointHandle, EndpointInfo},
        ffi::PerformerPtr,
        performer::endpoints::{
            event::{fetch_events, post_event},
            stream::{read_stream, write_stream, StreamType},
            value::{GetOutputValue, SetInputValue},
        },
        value::ValueRef,
    },
    sealed::sealed,
    std::collections::HashMap,
};

/// A Cmajor performer.
pub struct Performer {
    ptr: PerformerPtr,
    endpoints: HashMap<EndpointHandle, EndpointInfo>,
    buffer: Vec<u8>,
}

impl Performer {
    pub(crate) fn new(
        performer: PerformerPtr,
        endpoints: HashMap<EndpointHandle, EndpointInfo>,
    ) -> Self {
        Performer {
            ptr: performer,
            endpoints,
            buffer: vec![0; 512],
        }
    }
}

impl Performer {
    /// Sets the block size of the performer.
    pub fn set_block_size(&mut self, num_frames: u32) {
        self.ptr.set_block_size(num_frames);
    }

    /// Renders the next block of frames.
    pub fn advance(&mut self) {
        self.ptr.advance();
    }

    /// Returns information about a given endpoint.
    pub fn endpoint_info<T>(&self, Endpoint(endpoint): Endpoint<T>) -> Option<&EndpointInfo>
    where
        T: EndpointType,
    {
        self.endpoints.get(&endpoint.handle())
    }

    /// Set the value of an endpoint.
    pub fn set<T>(&mut self, endpoint: Endpoint<InputValue<T>>, value: T) -> T::Output
    where
        T: SetInputValue,
    {
        SetInputValue::set_input_value(self, endpoint, value)
    }

    /// Get the value of an endpoint.
    pub fn get<T>(&mut self, endpoint: Endpoint<OutputValue<T>>) -> T::Output<'_>
    where
        T: GetOutputValue,
    {
        T::get_output_value(self, endpoint)
    }

    /// Post an event to an endpoint.
    pub fn post<'a>(
        &mut self,
        endpoint: Endpoint<InputEvent>,
        event: impl Into<ValueRef<'a>>,
    ) -> Result<(), EndpointError> {
        post_event(self, endpoint, event.into())
    }

    /// Fetch the events received from an endpoint.
    pub fn fetch(
        &mut self,
        endpoint: Endpoint<OutputEvent>,
        callback: impl FnMut(usize, ValueRef<'_>),
    ) -> Result<usize, EndpointError> {
        fetch_events(self, endpoint, callback)
    }

    /// Read frames from an input stream.
    pub fn read<T>(&self, endpoint: Endpoint<OutputStream<T>>, buffer: &mut [T])
    where
        T: StreamType,
    {
        read_stream(self, endpoint, buffer)
    }

    /// Write frames to an output stream.
    pub fn write<T>(&self, endpoint: Endpoint<InputStream<T>>, buffer: &[T])
    where
        T: StreamType,
    {
        write_stream(self, endpoint, buffer)
    }

    /// Returns the number of times the performer has over/under-run.
    pub fn get_xruns(&self) -> usize {
        self.ptr.get_xruns()
    }

    /// Returns the maximum number of frames that can be processed in a single call to `advance`.
    pub fn get_max_block_size(&self) -> u32 {
        self.ptr.get_max_block_size()
    }

    /// Returns the performers internal latency in frames.
    pub fn get_latency(&self) -> f64 {
        self.ptr.get_latency()
    }
}

/// An error that can occur when interacting with performer endpoints.
#[derive(Debug, thiserror::Error)]
pub enum EndpointError {
    /// The endpoint does not exist.
    #[error("no such endpoint")]
    EndpointDoesNotExist,

    /// The direction of the endpoint does not match the expected direction.
    #[error("direction mismatch")]
    DirectionMismatch,

    /// The type of the endpoint does not match the expected type.
    #[error("type mismatch")]
    EndpointTypeMismatch,

    /// The data type does not match the expected type.
    #[error("data type mismatch")]
    DataTypeMismatch,
}

#[doc(hidden)]
#[sealed(pub(crate))]
pub trait EndpointType {
    fn make(
        handle: EndpointHandle,
        endpoint: EndpointInfo,
    ) -> Result<Endpoint<Self>, EndpointError>
    where
        Self: Sized;

    fn handle(&self) -> EndpointHandle;
}
