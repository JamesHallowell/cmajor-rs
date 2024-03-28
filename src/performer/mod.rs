//! The Cmajor performer for running programs.

mod atomic;
mod endpoints;

pub use endpoints::{Endpoint, InputEvent, InputValue, OutputValue};
use {
    crate::{
        endpoint::{EndpointDirection, EndpointHandle, EndpointType, ProgramEndpoints},
        ffi::PerformerPtr,
        value::ValueRef,
    },
    endpoints::CachedInputValues,
    sealed::sealed,
    std::sync::Arc,
};

/// A Cmajor performer.
pub struct Performer<Streams = ((), ())> {
    inner: PerformerPtr,
    endpoints: Arc<ProgramEndpoints>,
    inputs: Vec<EndpointHandler>,
    outputs: Vec<EndpointHandler>,
    cached_input_values: CachedInputValues,
    streams: Streams,
}

pub(crate) type EndpointHandler = Box<dyn FnMut(&mut PerformerPtr) + Send>;

impl Performer<((), ())> {
    pub(crate) fn new(performer: PerformerPtr, endpoints: Arc<ProgramEndpoints>) -> Self {
        Performer {
            inner: performer,
            endpoints: Arc::clone(&endpoints),
            inputs: vec![],
            outputs: vec![],
            cached_input_values: CachedInputValues::default(),
            streams: ((), ()),
        }
    }
}

impl<Streams> Performer<Streams> {
    /// Returns an endpoint of the performer.
    pub fn endpoint<T>(&mut self, id: impl AsRef<str>) -> Result<Endpoint<T>, EndpointError>
    where
        T: PerformerEndpoint,
    {
        PerformerEndpoint::make(id.as_ref(), self)
    }

    /// Returns the endpoints of the performer.
    pub fn endpoints(&self) -> &ProgramEndpoints {
        &self.endpoints
    }

    /// Sets the block size of the performer.
    pub fn set_block_size(&mut self, num_frames: u32) {
        self.inner.set_block_size(num_frames);
    }

    /// Renders the next block of frames.
    pub fn advance(&mut self) {
        let Self {
            inner,
            inputs,
            outputs,
            ..
        } = self;

        for input in inputs {
            input(inner);
        }

        inner.advance();

        for output in outputs {
            output(inner);
        }
    }

    /// Reads the output frames of an endpoint into the given slice.
    ///
    /// # Safety
    ///
    /// To avoid overhead in the real-time audio thread this function does not perform any checks
    /// against the inputs and passes them directly to the Cmajor library.
    ///
    /// The caller is responsible for ensuring that the type of the endpoint matches the type of the
    /// given slice.
    pub unsafe fn read_stream_unchecked<T>(&mut self, handle: EndpointHandle, frames: &mut [T])
    where
        T: Copy,
    {
        self.inner.copy_output_frames(handle, frames);
    }

    /// Writes the input frames to an endpoint from the given slice.
    ///
    /// # Safety
    ///
    /// To avoid overhead in the real-time audio thread this function does not perform any checks
    /// against the inputs and passes them directly to the Cmajor library.
    ///
    /// The caller is responsible for ensuring that the type of the endpoint matches the type of the
    /// given slice.
    pub unsafe fn write_stream_unchecked<T>(&mut self, handle: EndpointHandle, frames: &[T])
    where
        T: Copy,
    {
        self.inner.set_input_frames(handle, frames);
    }

    /// Iterates over the events of an endpoint.
    pub fn read_events(
        &mut self,
        handle: EndpointHandle,
        mut callback: impl FnMut(usize, EndpointHandle, ValueRef<'_>),
    ) -> Result<usize, EndpointError> {
        let endpoint = self
            .endpoints
            .get(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.direction() != EndpointDirection::Output {
            return Err(EndpointError::DirectionMismatch);
        }

        let endpoint = if let EndpointType::Event(endpoint) = endpoint {
            endpoint
        } else {
            return Err(EndpointError::EndpointTypeMismatch);
        };

        let mut events = 0;
        self.inner
            .iterate_output_events(handle, |frame_offset, handle, type_index, data| {
                let ty = endpoint.get_type(type_index);
                debug_assert!(ty.is_some(), "Invalid type index from Cmajor");

                if let Some(ty) = endpoint.get_type(type_index) {
                    callback(
                        frame_offset,
                        handle,
                        ValueRef::new_from_slice(ty.as_ref(), data),
                    );
                    events += 1;
                }
            });

        Ok(events)
    }

    /// Returns the number of times the performer has over/under-run.
    pub fn get_xruns(&self) -> usize {
        self.inner.get_xruns()
    }

    /// Returns the maximum number of frames that can be processed in a single call to `advance`.
    pub fn get_max_block_size(&self) -> u32 {
        self.inner.get_max_block_size()
    }

    /// Returns the performers internal latency in frames.
    pub fn get_latency(&self) -> f64 {
        self.inner.get_latency()
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

    /// Failed to send a message to the performer.
    #[error("failed to send message to performer")]
    FailedToSendMessageToPerformer,
}

#[doc(hidden)]
#[sealed(pub(crate))]
pub trait PerformerEndpoint {
    fn make<Streams>(
        id: &str,
        performer: &mut Performer<Streams>,
    ) -> Result<Endpoint<Self>, EndpointError>
    where
        Self: Sized;
}
