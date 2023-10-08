use {
    crate::engine::EndpointHandle,
    serde::{Deserialize, Serialize},
    std::io::Read,
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
    sender: rtrb::Producer<u8>,
}

#[derive(Debug)]
pub struct EndpointReceiver {
    receiver: rtrb::Consumer<u8>,
    buffer: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Serialize(#[from] bincode::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn channel(capacity: usize) -> (EndpointSender, EndpointReceiver) {
    let (producer, consumer) = rtrb::RingBuffer::new(capacity);
    (
        EndpointSender { sender: producer },
        EndpointReceiver {
            receiver: consumer,
            buffer: vec![0; capacity],
        },
    )
}

impl EndpointSender {
    pub fn send_value(&mut self, endpoint: EndpointHandle, data: &[u8]) -> Result<(), Error> {
        self.write(&EndpointMessage::Value {
            handle: endpoint,
            data,
        })
    }

    pub fn send_event(
        &mut self,
        endpoint: EndpointHandle,
        type_index: u32,
        data: &[u8],
    ) -> Result<(), Error> {
        self.write(&EndpointMessage::Event {
            handle: endpoint,
            type_index,
            data,
        })
    }

    fn write<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let size = bincode::serialized_size(value)?;
        bincode::serialize_into(&mut self.sender, &(size, value))?;
        Ok(())
    }
}

impl EndpointReceiver {
    pub fn read_messages(&mut self, callback: impl FnMut(EndpointMessage)) -> Result<usize, Error> {
        self.read_all(callback)
    }

    fn read_all<'de, 'this: 'de, T>(
        &'this mut self,
        mut callback: impl FnMut(T),
    ) -> Result<usize, Error>
    where
        T: Deserialize<'de>,
    {
        if self.receiver.is_empty() {
            return Ok(0);
        }

        let read = self.receiver.read(&mut self.buffer)?;

        let mut scratch_buffer = &self.buffer[..read];

        let mut count = 0;
        while !scratch_buffer.is_empty() {
            let size = bincode::deserialize::<u64>(scratch_buffer)? as usize;
            scratch_buffer = &scratch_buffer[std::mem::size_of::<u64>()..];

            let value = bincode::deserialize::<T>(&scratch_buffer[..size])?;
            callback(value);

            scratch_buffer = &scratch_buffer[size..];
            count += 1;
        }

        Ok(count)
    }
}

mod test {
    use super::*;

    #[test]
    fn can_read_and_write_values_into_shared_buffer_without_allocating() {
        use assert_no_alloc::*;

        #[cfg(debug_assertions)]
        #[global_allocator]
        static ALLOCATOR: AllocDisabler = AllocDisabler;

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct S<'a> {
            flag: bool,
            buffer: &'a [u8],
        }

        let a = S {
            flag: true,
            buffer: &[1, 2, 3, 4, 5],
        };

        let (mut producer, mut consumer) = channel(1024);
        let count = assert_no_alloc(|| {
            producer.write(&a).unwrap();

            consumer
                .read_all(|b: S| {
                    assert_eq!(a, b);
                })
                .unwrap()
        });
        assert_eq!(count, 1);
    }
}
