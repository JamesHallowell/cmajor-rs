use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProgramId(u64);

impl ProgramId {
    pub(crate) fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn each_next_id_is_distinct() {
        assert_ne!(ProgramId::next(), ProgramId::next());
    }
}
