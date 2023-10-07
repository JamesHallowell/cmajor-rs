use {
    crate::{
        engine::{EndpointHandle, EndpointType, Endpoints},
        ffi::PerformerPtr,
        spsc::{self, EndpointMessage, EndpointSender},
        value::{IsType, Value, ValueRef},
    },
    std::sync::Arc,
};

pub struct PerformerBuilder {
    inner: PerformerPtr,
    endpoints: Arc<Endpoints>,
    block_size: Option<u32>,
}

pub struct Performer {
    inner: PerformerPtr,
    endpoints: Arc<Endpoints>,
    endpoint_rx: spsc::EndpointReceiver,
    scratch_buffer: Vec<u8>,
}

pub struct EndpointHandles {
    endpoints: Arc<Endpoints>,
    endpoint_tx: EndpointSender,
}

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("no block size set")]
    NoBlockSize,
}

impl PerformerBuilder {
    pub(crate) fn new(performer: PerformerPtr, endpoints: Arc<Endpoints>) -> Self {
        Self {
            inner: performer,
            endpoints,
            block_size: None,
        }
    }

    pub fn with_block_size(mut self, num_frames_per_block: u32) -> Self {
        self.block_size.replace(num_frames_per_block);
        self
    }

    pub fn build(self) -> Result<(Performer, EndpointHandles), BuilderError> {
        let block_size = self.block_size.ok_or(BuilderError::NoBlockSize)?;
        self.inner.set_block_size(block_size);

        let (endpoint_tx, endpoint_rx) = spsc::channel(8192);

        Ok((
            Performer {
                inner: self.inner,
                endpoints: Arc::clone(&self.endpoints),
                endpoint_rx,
                scratch_buffer: vec![0; 512],
            },
            EndpointHandles {
                endpoints: self.endpoints,
                endpoint_tx,
            },
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EndpointError {
    #[error("no such endpoint")]
    EndpointDoesNotExist,

    #[error("type mismatch")]
    EndpointTypeMismatch,

    #[error("data type mismatch")]
    DataTypeMismatch,

    #[error("failed to send value")]
    FailedToSendValue,
}

impl Performer {
    pub fn advance(&mut self) {
        let result = self.endpoint_rx.read_messages(|message| match message {
            EndpointMessage::Value { handle, data } => {
                unsafe { self.inner.set_input_value(handle, data.as_ptr(), 0) };
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

    pub fn read_value(&mut self, id: impl AsRef<str>) -> Result<ValueRef<'_>, EndpointError> {
        let (handle, endpoint) = self
            .endpoints
            .get_output_by_id(id)
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
        id: impl AsRef<str>,
        frames: &mut [T],
    ) -> Result<(), EndpointError>
    where
        T: IsType,
    {
        let (handle, endpoint) = self
            .endpoints
            .get_output_by_id(id)
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

impl EndpointHandles {
    pub fn write_value(
        &mut self,
        id: impl AsRef<str>,
        value: impl Into<Value>,
    ) -> Result<(), EndpointError> {
        let (handle, endpoint) = self
            .endpoints
            .get_input_by_id(id)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Value {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        let value = value.into();

        if !endpoint.value_type().contains(value.ty()) {
            return Err(EndpointError::DataTypeMismatch);
        }

        self.endpoint_tx
            .send_value(handle, value.data())
            .map_err(|_| EndpointError::FailedToSendValue)
    }

    pub fn post_event(
        &mut self,
        id: impl AsRef<str>,
        value: impl Into<Value>,
    ) -> Result<(), EndpointError> {
        let (handle, endpoint) = self
            .endpoints
            .get_input_by_id(id)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Event {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        let value = value.into();

        let index = endpoint
            .value_type()
            .iter()
            .enumerate()
            .find_map(|(index, ty)| (ty == value.ty()).then_some(index))
            .ok_or(EndpointError::DataTypeMismatch)?;

        self.endpoint_tx
            .send_event(handle, index as u32, value.data())
            .map_err(|_| EndpointError::FailedToSendValue)
    }
}
