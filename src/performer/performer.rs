use {
    crate::{
        engine::{EndpointHandle, EndpointType, Endpoints},
        ffi::PerformerPtr,
        performer::{spsc, spsc::EndpointMessage},
        value::ValueRef,
        EndpointError,
    },
    std::sync::Arc,
};

pub struct Performer {
    pub(super) inner: PerformerPtr,
    pub(super) endpoints: Arc<Endpoints>,
    pub(super) endpoint_rx: spsc::EndpointReceiver,
    pub(super) scratch_buffer: Vec<u8>,
    pub(super) block_size: Option<u32>,
}

impl Performer {
    /// Renders the next block of frames.
    pub fn advance(&mut self, num_frames: u32) {
        if self.block_size != Some(num_frames) {
            self.inner.set_block_size(num_frames);
            self.block_size.replace(num_frames);
        }

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

    pub fn get_output(&self, id: impl AsRef<str>) -> Option<EndpointHandle> {
        self.endpoints
            .get_output_by_id(id)
            .map(|(handle, _)| handle)
    }

    pub fn read_value(&mut self, handle: EndpointHandle) -> Result<ValueRef<'_>, EndpointError> {
        let endpoint = self
            .endpoints
            .get_output(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Value {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        let value_type = endpoint.value_type().first().unwrap();

        self.inner
            .copy_output_value(handle, self.scratch_buffer.as_mut_slice());

        Ok(ValueRef::from_bytes(value_type, &self.scratch_buffer))
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

    pub fn read_events(
        &mut self,
        handle: EndpointHandle,
        mut callback: impl FnMut(usize, EndpointHandle, ValueRef<'_>),
    ) -> Result<(), EndpointError> {
        let endpoint = self
            .endpoints
            .get_output(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Event {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        self.inner
            .iterate_output_events(handle, |frame_offset, handle, type_index, data| {
                let ty = endpoint
                    .value_type_at_index(type_index)
                    .expect("type index out of bounds");

                callback(frame_offset, handle, ValueRef::from_bytes(ty, data))
            });

        Ok(())
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
