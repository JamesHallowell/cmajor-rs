use {
    crate::{
        endpoint::{
            endpoint_channel, EndpointConsumer, EndpointId, EndpointMessage, EndpointProducer,
        },
        engine::{Endpoint, EndpointHandle},
        ffi::PerformerPtr,
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
            },
            Endpoints {
                endpoints: self.endpoints,
                endpoint_producer,
            },
        ))
    }
}

pub struct InputStream<T> {
    _marker: PhantomData<T>,
}

pub struct OutputStream<'a, T> {
    handle: EndpointHandle,
    performer: &'a mut Performer,
    _marker: PhantomData<T>,
}

impl<'a, T> OutputStream<'a, T> {
    pub fn copy_frames(&mut self, frames: &mut [T]) {
        self.performer
            .inner
            .copy_output_frames(self.handle.into(), frames);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no such endpoint")]
    EndpointDoesNotExist,
}

impl Performer {
    pub fn advance(&mut self) {
        let result = self
            .endpoint_consumer
            .read_messages(|message| match message {
                EndpointMessage::Value { handle, data } => {
                    self.inner.set_input_value(handle.into(), data, 0);
                }
            });
        debug_assert!(result.is_ok());

        self.inner.advance();
    }

    pub fn output_value(&mut self, id: impl AsRef<str>) -> Result<OutputValue<'_, i32>, Error> {
        let endpoint = self
            .endpoints
            .get(id.as_ref())
            .ok_or(Error::EndpointDoesNotExist)?;

        let handle = endpoint.handle;

        Ok(OutputValue {
            handle,
            performer: self,
            _marker: PhantomData,
        })
    }

    pub fn output_stream(&mut self, id: impl AsRef<str>) -> Result<OutputStream<'_, f32>, Error> {
        let endpoint = self
            .endpoints
            .get(id.as_ref())
            .ok_or(Error::EndpointDoesNotExist)?;

        let handle = endpoint.handle;

        Ok(OutputStream {
            handle,
            performer: self,
            _marker: PhantomData,
        })
    }
}

pub struct InputValue<'a, T> {
    handle: EndpointHandle,
    endpoints: &'a mut Endpoints,
    _marker: PhantomData<T>,
}

pub struct OutputValue<'a, T> {
    handle: EndpointHandle,
    performer: &'a mut Performer,
    _marker: PhantomData<T>,
}

impl InputValue<'_, i32> {
    pub fn send(&mut self, value: i32) {
        let result = self
            .endpoints
            .endpoint_producer
            .send_value(self.handle.into(), &value.to_ne_bytes());

        debug_assert!(result.is_ok());
    }
}

impl OutputValue<'_, i32> {
    pub fn get(&mut self) -> i32 {
        self.performer.inner.copy_output_value(self.handle.into())
    }
}

impl Endpoints {
    pub fn input_value(&mut self, id: impl AsRef<str>) -> Result<InputValue<'_, i32>, Error> {
        let endpoint = self
            .endpoints
            .get(id.as_ref())
            .ok_or(Error::EndpointDoesNotExist)?;

        let handle = endpoint.handle;

        Ok(InputValue {
            handle,
            endpoints: self,
            _marker: PhantomData,
        })
    }
}
