//! The Cmajor performer for running programs.

mod endpoints;

#[doc(hidden)]
pub use endpoints::{event::EventType, value::ValueType, EndpointKind};
pub use endpoints::{
    event::{InputEvent, OutputEvent},
    stream::{InputStream, OutputStream, StreamType},
    value::{InputValue, OutputValue},
    Endpoint,
};

use crate::{
    engine::ProgramDetails,
    ffi::PerformerPtr,
    value::{StringHandle, ValueRef},
};

/// A Cmajor performer.
pub struct Performer {
    ptr: PerformerPtr,
    buffer: Vec<u8>,
    console: Option<Endpoint<OutputEvent>>,
}

impl Performer {
    pub(crate) fn new(
        performer: PerformerPtr,
        program_details: &ProgramDetails,
        console: Option<Endpoint<OutputEvent>>,
    ) -> Self {
        let size_of_largest_type = program_details
            .endpoints()
            .flat_map(|endpoint| endpoint.types().iter().map(|ty| ty.size()).max())
            .max()
            .unwrap_or(0);

        Performer {
            ptr: performer,
            buffer: vec![0; size_of_largest_type],
            console,
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

        if let Some(console) = &self.console {
            self.fetch(console, |_, value| match value {
                ValueRef::String(StringHandle(handle)) => {
                    println!("{}", self.ptr.get_string_for_handle(handle).unwrap_or("?"));
                }
                value => println!("{value:?}"),
            });
        }
    }

    /// Set the value of an endpoint.
    pub fn set<T>(&mut self, endpoint: &Endpoint<InputValue<T>>, value: T) -> T::SetResult
    where
        T: ValueType,
    {
        T::set(self, endpoint, value)
    }

    /// Get the value of an endpoint.
    pub fn get<'a, T>(&'a mut self, endpoint: &'a Endpoint<OutputValue<T>>) -> T::Value<'a>
    where
        T: ValueType,
    {
        T::get(self, endpoint)
    }

    /// Post an event to an endpoint.
    pub fn post<T>(
        &mut self,
        endpoint: &Endpoint<InputEvent<T>>,
        event: T::Event<'_>,
    ) -> T::PostResult
    where
        T: EventType,
    {
        T::post(self, endpoint, event)
    }

    /// Fetch the events received from an endpoint.
    pub fn fetch<T>(
        &self,
        endpoint: &Endpoint<OutputEvent<T>>,
        callback: impl FnMut(usize, T::Event<'_>),
    ) where
        T: EventType,
    {
        T::fetch(self, endpoint, callback)
    }

    /// Read frames from an output stream.
    pub fn read<T>(&self, endpoint: &Endpoint<OutputStream<T>>, buffer: &mut [T])
    where
        T: StreamType,
    {
        endpoint.read(self, buffer)
    }

    /// Write frames to an input stream.
    pub fn write<T>(&self, endpoint: &Endpoint<InputStream<T>>, buffer: &[T])
    where
        T: StreamType,
    {
        endpoint.write(self, buffer)
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

    /// Returns the string associated with a handle.
    pub fn get_string(&self, StringHandle(value): StringHandle) -> Option<&str> {
        self.ptr.get_string_for_handle(value)
    }
}

/// An error that can occur when binding an endpoint.
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

/// A runtime type mismatch when interacting with an endpoint.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("value type does not match endpoint type")]
pub struct TypeMismatch;
