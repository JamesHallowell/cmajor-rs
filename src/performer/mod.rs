//! The Cmajor performer for running programs.

mod handle;
mod spsc;

pub use handle::{EndpointError, PerformerHandle};
use {
    crate::{
        endpoint::{Endpoint, EndpointHandle, Endpoints},
        ffi::PerformerPtr,
        performer::spsc::EndpointMessage,
        value::ValueRef,
    },
    std::sync::Arc,
};

/// A Cmajor performer.
pub struct Performer {
    pub(super) inner: PerformerPtr,
    pub(super) endpoints: Arc<Endpoints>,
    pub(super) endpoint_rx: spsc::EndpointReceiver,
    pub(super) scratch_buffer: Vec<u8>,
}

impl Performer {
    pub(crate) fn new(
        performer: PerformerPtr,
        endpoints: Arc<Endpoints>,
    ) -> (Self, PerformerHandle) {
        let (endpoint_tx, endpoint_rx) = spsc::channel(8192);

        (
            Performer {
                inner: performer,
                endpoints: Arc::clone(&endpoints),
                endpoint_rx,
                scratch_buffer: vec![0; 512],
            },
            PerformerHandle {
                endpoints,
                endpoint_tx,
            },
        )
    }

    /// Sets the block size of the performer.
    pub fn set_block_size(&mut self, num_frames: u32) {
        self.inner.set_block_size(num_frames);
    }

    /// Renders the next block of frames.
    pub fn advance(&mut self) {
        let result = self.endpoint_rx.read_messages(|message| match message {
            EndpointMessage::Value {
                handle,
                data,
                num_frames_to_reach_value,
            } => {
                unsafe {
                    self.inner
                        .set_input_value(handle, data.as_ptr(), num_frames_to_reach_value)
                };
            }
            EndpointMessage::Event {
                handle,
                type_index,
                data,
            } => self.inner.add_input_event(handle, type_index, data),
        });
        debug_assert!(result.is_ok());

        self.inner.advance();
    }

    /// Returns the [`EndpointHandle`] for the endpoint with the given ID.
    pub fn get_output(&self, id: impl AsRef<str>) -> Option<(EndpointHandle, &Endpoint)> {
        self.endpoints.get_output_by_id(id)
    }

    /// Reads the value of an endpoint.
    pub fn read_value(&mut self, handle: EndpointHandle) -> Result<ValueRef<'_>, EndpointError> {
        let endpoint = self
            .endpoints
            .get_output(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        let endpoint = if let Endpoint::Value(endpoint) = endpoint {
            endpoint
        } else {
            return Err(EndpointError::EndpointTypeMismatch);
        };

        self.inner
            .copy_output_value(handle, self.scratch_buffer.as_mut_slice());

        Ok(ValueRef::new_from_slice(
            endpoint.ty().as_ref(),
            &self.scratch_buffer,
        ))
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
            .get_output(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        let endpoint = if let Endpoint::Event(endpoint) = endpoint {
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
