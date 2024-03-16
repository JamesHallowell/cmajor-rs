//! The Cmajor performer for running programs.

use {
    crate::{
        endpoint::{Endpoint, EndpointDirection, EndpointHandle, Endpoints},
        ffi::PerformerPtr,
        value::{Value, ValueRef},
    },
    std::sync::Arc,
};

/// A Cmajor performer.
pub struct Performer {
    pub(super) inner: PerformerPtr,
    pub(super) endpoints: Arc<Endpoints>,
    pub(super) scratch_buffer: Vec<u8>,
}

impl Performer {
    pub(crate) fn new(performer: PerformerPtr, endpoints: Arc<Endpoints>) -> Self {
        Performer {
            inner: performer,
            endpoints: Arc::clone(&endpoints),
            scratch_buffer: vec![0; 512],
        }
    }

    /// Returns the endpoints of the performer.
    pub fn endpoints(&self) -> &Endpoints {
        &self.endpoints
    }

    /// Sets the block size of the performer.
    pub fn set_block_size(&mut self, num_frames: u32) {
        self.inner.set_block_size(num_frames);
    }

    /// Renders the next block of frames.
    pub fn advance(&mut self) {
        self.inner.advance();
    }

    /// Reads the value of an endpoint.
    pub fn get_value(&mut self, handle: EndpointHandle) -> Result<ValueRef<'_>, EndpointError> {
        let endpoint = self
            .endpoints
            .get(handle)
            .filter(|endpoint| endpoint.direction() == EndpointDirection::Output)
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

    /// Set the value of an endpoint.
    pub fn set_value(
        &mut self,
        handle: EndpointHandle,
        value: impl Into<Value>,
    ) -> Result<(), EndpointError> {
        let endpoint = self
            .endpoints
            .get(handle)
            .filter(|endpoint| endpoint.direction() == EndpointDirection::Input)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        let endpoint = if let Endpoint::Value(value) = endpoint {
            value
        } else {
            return Err(EndpointError::EndpointTypeMismatch);
        };

        let value = value.into();

        if endpoint.ty().as_ref() != value.ty() {
            return Err(EndpointError::DataTypeMismatch);
        }

        value.with_bytes(|bytes| {
            unsafe { self.inner.set_input_value(handle, bytes.as_ptr(), 0) };
        });

        Ok(())
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

    /// Post an event to an endpoint.
    pub fn post_event(
        &mut self,
        handle: EndpointHandle,
        value: impl Into<Value>,
    ) -> Result<(), EndpointError> {
        let endpoint = self
            .endpoints
            .get(handle)
            .filter(|endpoint| endpoint.direction() == EndpointDirection::Input)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        let endpoint = if let Endpoint::Event(endpoint) = endpoint {
            endpoint
        } else {
            return Err(EndpointError::EndpointTypeMismatch);
        };

        let value = value.into();

        let type_index = endpoint
            .type_index(value.ty())
            .ok_or(EndpointError::DataTypeMismatch)?;

        value.with_bytes(|bytes| {
            self.inner
                .add_input_event(handle, type_index, bytes.as_ref())
        });

        Ok(())
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
            .filter(|endpoint| endpoint.direction() == EndpointDirection::Output)
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
