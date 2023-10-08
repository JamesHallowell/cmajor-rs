use {
    crate::{
        engine::{EndpointHandle, EndpointType, Endpoints},
        ffi::PerformerPtr,
        performer::{spsc, spsc::EndpointMessage},
        value::{IsType, ValueRef},
        EndpointError,
    },
    std::sync::Arc,
};

pub struct Performer {
    pub(super) inner: PerformerPtr,
    pub(super) endpoints: Arc<Endpoints>,
    pub(super) endpoint_rx: spsc::EndpointReceiver,
    pub(super) scratch_buffer: Vec<u8>,
}

impl Performer {
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

    pub fn read_stream<T>(
        &mut self,
        handle: EndpointHandle,
        frames: &mut [T],
    ) -> Result<(), EndpointError>
    where
        T: IsType,
    {
        let endpoint = self
            .endpoints
            .get_output(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if !endpoint.value_type().contains(&T::get_type()) {
            return Err(EndpointError::DataTypeMismatch);
        }

        self.inner.copy_output_frames(handle, frames);
        Ok(())
    }

    pub fn read_events(
        &mut self,
        id: impl AsRef<str>,
        mut callback: impl FnMut(EndpointHandle, ValueRef<'_>),
    ) -> Result<(), EndpointError> {
        let (handle, endpoint) = self
            .endpoints
            .get_output_by_id(id)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Event {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        self.inner
            .iterate_output_events(handle, |e, type_index, data| {
                let ty = endpoint
                    .value_type()
                    .iter()
                    .nth(type_index as usize)
                    .expect("type index out of bounds");

                callback(e, ValueRef::from_bytes(ty, data))
            });

        Ok(())
    }
}
