use {
    crate::{
        endpoint::{
            endpoint_channel, EndpointConsumer, EndpointId, EndpointMessage, EndpointProducer,
            EndpointType,
        },
        engine::Endpoint,
        ffi::PerformerPtr,
        types::CmajorType,
    },
    std::{collections::HashMap, marker::PhantomData, sync::Arc},
};

pub struct PerformerBuilder {
    inner: PerformerPtr,
    endpoints: Arc<HashMap<EndpointId, Endpoint>>,
    block_size: Option<u32>,
}

pub struct Performer {
    inner: PerformerPtr,
    endpoints: Arc<HashMap<EndpointId, Endpoint>>,
    endpoint_consumer: EndpointConsumer,
    scratch_buffer: Vec<u8>,
}

pub struct Endpoints {
    endpoints: Arc<HashMap<EndpointId, Endpoint>>,
    endpoint_producer: EndpointProducer,
}

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("no block size set")]
    NoBlockSize,
}

impl PerformerBuilder {
    pub(crate) fn new(
        performer: PerformerPtr,
        endpoints: Arc<HashMap<EndpointId, Endpoint>>,
    ) -> Self {
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

    pub fn build(self) -> Result<(Performer, Endpoints), BuilderError> {
        let block_size = self.block_size.ok_or(BuilderError::NoBlockSize)?;
        self.inner.set_block_size(block_size);

        let (endpoint_producer, endpoint_consumer) = endpoint_channel(8192);

        Ok((
            Performer {
                inner: self.inner,
                endpoints: Arc::clone(&self.endpoints),
                endpoint_consumer,
                scratch_buffer: vec![0; 512],
            },
            Endpoints {
                endpoints: self.endpoints,
                endpoint_producer,
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
        let result = self
            .endpoint_consumer
            .read_messages(|message| match message {
                EndpointMessage::Value { handle, data } => {
                    self.inner.set_input_value(handle, data, 0);
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

    pub fn read_value<T>(&mut self, id: impl AsRef<str>) -> Result<T, EndpointError>
    where
        T: CmajorType + Default,
    {
        let endpoint = self
            .endpoints
            .get(id.as_ref())
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Value {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        if endpoint.data_type_matches::<T>().is_none() {
            return Err(EndpointError::DataTypeMismatch);
        }

        unsafe {
            self.inner
                .copy_output_value(endpoint.handle(), self.scratch_buffer.as_mut_slice())
        };

        Ok(T::from_bytes(&self.scratch_buffer))
    }

    pub fn read_stream<T>(
        &mut self,
        id: impl AsRef<str>,
        frames: &mut [T],
    ) -> Result<(), EndpointError>
    where
        T: CmajorType,
    {
        let endpoint = self
            .endpoints
            .get(id.as_ref())
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.data_type_matches::<T>().is_none() {
            return Err(EndpointError::DataTypeMismatch);
        }

        let handle = endpoint.handle();

        self.inner.copy_output_frames(handle, frames);
        Ok(())
    }
}

impl Endpoints {
    pub fn write_value<Value>(
        &mut self,
        id: impl AsRef<str>,
        value: Value,
    ) -> Result<(), EndpointError>
    where
        Value: CmajorType,
    {
        let endpoint = self
            .endpoints
            .get(id.as_ref())
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Value {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        if endpoint.data_type_matches::<Value>().is_none() {
            return Err(EndpointError::DataTypeMismatch);
        }

        let handle = endpoint.handle();

        value
            .to_bytes(|bytes| self.endpoint_producer.send_value(handle, bytes))
            .map_err(|_| EndpointError::FailedToSendValue)
    }

    pub fn post_event<Value>(
        &mut self,
        id: impl AsRef<str>,
        value: Value,
    ) -> Result<(), EndpointError>
    where
        Value: CmajorType,
    {
        let endpoint = self
            .endpoints
            .get(id.as_ref())
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.endpoint_type() != EndpointType::Event {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        let type_index = if let Some(type_index) = endpoint.data_type_matches::<Value>() {
            type_index
        } else {
            return Err(EndpointError::DataTypeMismatch);
        };

        let handle = endpoint.handle();

        value
            .to_bytes(|bytes| {
                self.endpoint_producer
                    .send_event(handle, type_index as u32, bytes)
            })
            .map_err(|_| EndpointError::FailedToSendValue)
    }
}
