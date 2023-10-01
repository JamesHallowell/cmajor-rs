use {
    serde::{Deserialize, Serialize},
    std::io::Read,
};

pub struct Producer {
    inner: rtrb::Producer<u8>,
}

pub struct Consumer {
    inner: rtrb::Consumer<u8>,
    scratch_buffer: Vec<u8>,
}

pub fn buffer(capacity: usize) -> (Producer, Consumer) {
    let (producer, consumer) = rtrb::RingBuffer::new(capacity);
    (
        Producer { inner: producer },
        Consumer {
            inner: consumer,
            scratch_buffer: vec![0; capacity],
        },
    )
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Serialize(#[from] bincode::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Producer {
    pub fn write<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        let size = bincode::serialized_size(value)?;
        bincode::serialize_into(&mut self.inner, &(size, value))?;
        Ok(())
    }
}

impl Consumer {
    pub fn read_all<'de, 'this: 'de, T>(
        &'this mut self,
        mut callback: impl FnMut(&T),
    ) -> Result<usize, Error>
    where
        T: Deserialize<'de>,
    {
        let read = self.inner.read(&mut self.scratch_buffer)?;

        let mut scratch_buffer = &self.scratch_buffer[..read];

        let mut count = 0;
        while !scratch_buffer.is_empty() {
            let size = bincode::deserialize::<u64>(scratch_buffer)? as usize;
            scratch_buffer = &scratch_buffer[std::mem::size_of::<u64>()..];

            let value = bincode::deserialize::<T>(&scratch_buffer[..size])?;
            callback(&value);

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

        let (mut producer, mut consumer) = buffer(1024);
        let count = assert_no_alloc(|| {
            producer.write(&a).unwrap();

            consumer
                .read_all(|b: &S| {
                    assert_eq!(a, *b);
                })
                .unwrap()
        });
        assert_eq!(count, 1);
    }
}
