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
}

impl PerformerBuilder {
    pub(crate) fn new(performer: ffi::PerformerPtr, endpoints: Arc<Endpoints>) -> Self {
        Self {
            inner: performer,
            endpoints,
        }
    }

    pub fn build(self) -> (Performer, EndpointHandles) {
        let (endpoint_tx, endpoint_rx) = spsc::channel(8192);
        const SCRATCH_BUFFER_SIZE: usize = 512;
        (
            Performer {
                inner: self.inner,
                endpoints: Arc::clone(&self.endpoints),
                endpoint_rx,
                scratch_buffer: vec![0; SCRATCH_BUFFER_SIZE],
                block_size: None,
            },
            EndpointHandles {
                endpoints: self.endpoints,
                endpoint_tx,
            },
        )
    }
}
