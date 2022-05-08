use crate::cairo::lang::vm::memory_segments::MemorySegmentManager;

use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub struct StaticLocals {
    pub segments: Rc<RefCell<MemorySegmentManager>>,
}
