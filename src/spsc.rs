use {
    crate::{buffer, engine::EndpointHandle},
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Serialize, Deserialize)]
pub enum EndpointMessage<'a> {
    Value {
        handle: EndpointHandle,
        data: &'a [u8],
    },
    Event {
        handle: EndpointHandle,
        type_index: u32,
        data: &'a [u8],
    },
}

#[derive(Debug)]
pub struct EndpointSender {
    buffer: buffer::Producer,
}

#[derive(Debug)]
pub struct EndpointReceiver {
    buffer: buffer::Consumer,
}

pub fn channel(capacity: usize) -> (EndpointSender, EndpointReceiver) {
    let (sender, receiver) = buffer::make_buffer(capacity);
    (
        EndpointSender { buffer: sender },
        EndpointReceiver { buffer: receiver },
    )
}

impl EndpointSender {
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

    pub fn send_event(
        &mut self,
        endpoint: EndpointHandle,
        type_index: u32,
        data: &[u8],
    ) -> Result<(), buffer::Error> {
        self.buffer.write(&EndpointMessage::Event {
            handle: endpoint,
            type_index,
            data,
        })
    }
}

impl EndpointReceiver {
    pub fn read_messages(
        &mut self,
        callback: impl FnMut(EndpointMessage),
    ) -> Result<usize, buffer::Error> {
        self.buffer.read_all(callback)
    }
}
