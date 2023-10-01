use serde::{Deserialize, Serialize};

mod buffer;
mod details;

pub use details::{EndpointDataType, EndpointDetails, EndpointDirection, EndpointId, EndpointType};

use crate::engine::EndpointHandle;

#[derive(Debug)]
pub struct EndpointProducer {
    buffer: buffer::Producer,
}

#[derive(Debug)]
pub struct EndpointConsumer {
    buffer: buffer::Consumer,
}

pub fn endpoint_channel(capacity: usize) -> (EndpointProducer, EndpointConsumer) {
    let (producer, consumer) = buffer::make_buffer(capacity);
    (
        EndpointProducer { buffer: producer },
        EndpointConsumer { buffer: consumer },
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EndpointMessage<'a> {
    Value {
        handle: EndpointHandle,
        data: &'a [u8],
    },
}

impl EndpointProducer {
    pub fn send_value(
        &mut self,
        endpoint: EndpointHandle,
        data: &[u8],
    ) -> Result<(), buffer::Error> {
        self.buffer.write(&EndpointMessage::Value {
            handle: endpoint,
            data,
        })
    }
}

impl EndpointConsumer {
    pub fn read_messages(
        &mut self,
        callback: impl FnMut(EndpointMessage),
    ) -> Result<usize, buffer::Error> {
        self.buffer.read_all(callback)
    }
}
