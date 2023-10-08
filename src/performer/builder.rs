use {
    crate::{
        engine::Endpoints,
        ffi,
        performer::{spsc, EndpointHandles, Performer},
    },
    std::sync::Arc,
};

pub struct PerformerBuilder {
    inner: ffi::PerformerPtr,
    endpoints: Arc<Endpoints>,
    block_size: Option<u32>,
}

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("no block size set")]
    NoBlockSize,
}

impl PerformerBuilder {
    pub(crate) fn new(performer: ffi::PerformerPtr, endpoints: Arc<Endpoints>) -> Self {
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

        const SCRATCH_BUFFER_SIZE: usize = 512;
        Ok((
            Performer {
                inner: self.inner,
                endpoints: Arc::clone(&self.endpoints),
                endpoint_rx,
                scratch_buffer: vec![0; SCRATCH_BUFFER_SIZE],
            },
            EndpointHandles {
                endpoints: self.endpoints,
                endpoint_tx,
            },
        ))
    }
}
