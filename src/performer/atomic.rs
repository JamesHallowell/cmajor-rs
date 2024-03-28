use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

#[derive(Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn load(&self, order: Ordering) -> f32 {
        f32::from_bits(self.0.load(order))
    }

    pub fn store(&self, value: f32, order: Ordering) {
        self.0.store(value.to_bits(), order);
    }
}

#[derive(Default)]
pub struct AtomicF64(AtomicU64);

impl AtomicF64 {
    pub fn load(&self, order: Ordering) -> f64 {
        f64::from_bits(self.0.load(order))
    }

    pub fn store(&self, value: f64, order: Ordering) {
        self.0.store(value.to_bits(), order);
    }
}
