use crate::{engine::EndpointHandle, ffi::PerformerPtr};

pub struct Performer {
    inner: PerformerPtr,
    block_size: Option<u32>,
}

impl From<PerformerPtr> for Performer {
    fn from(inner: PerformerPtr) -> Self {
        Self {
            inner,
            block_size: None,
        }
    }
}

impl Performer {
    pub fn set_block_size(&mut self, num_frames_per_block: u32) {
        self.inner.set_block_size(num_frames_per_block);
        self.block_size.replace(num_frames_per_block);
    }

    pub fn advance(&mut self) {
        assert!(
            self.block_size.is_some(),
            "block size must be set before advancing"
        );

        self.inner.advance();
    }

    pub fn copy_output_frames(&self, endpoint: &EndpointHandle, frames: &mut [f32]) {
        assert_eq!(
            self.block_size,
            Some(frames.len() as u32),
            "frames must be the same size as the block size"
        );

        self.inner.copy_output_frames(endpoint.0, frames);
    }
}
