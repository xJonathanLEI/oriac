use crate::cairo::lang::vm::memory_segments::MemorySegmentManager;

use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct StaticLocals {
    pub segments: Arc<Mutex<MemorySegmentManager>>,
}
