use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
pub struct AtomicIdGenerator {
    curr_id: AtomicU32,
}

impl AtomicIdGenerator {
    pub fn new() -> Self {
        Self {
            curr_id: AtomicU32::new(1),
        }
    }

    pub fn next_id(&self) -> u32 {
        self.curr_id.fetch_add(1, Ordering::SeqCst)
    }
}
