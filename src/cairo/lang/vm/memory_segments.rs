use std::collections::HashMap;

use num_bigint::BigInt;

use crate::cairo::lang::vm::memory_dict::MemoryDict;

/// Manages the list of memory segments, and allows relocating them once their sizes are known.
#[derive(Debug)]
pub struct MemorySegmentManager {
    pub memory: MemoryDict,
    pub prime: BigInt,
    /// Number of segments.
    pub n_segments: BigInt,
    /// A map from segment index to its size.
    pub segment_sizes: HashMap<BigInt, BigInt>,
    pub segment_used_sizes: Option<HashMap<BigInt, BigInt>>,
    /// A map from segment index to a list of pairs (offset, page_id) that constitute the public
    /// memory. Note that the offset is absolute (not based on the page_id).
    pub public_memory_offsets: HashMap<BigInt, Vec<[BigInt; 2]>>,
    /// The number of temporary segments, see 'add_temp_segment' for more details.
    pub n_temp_segments: BigInt,
}

impl MemorySegmentManager {
    pub fn new(memory: MemoryDict, prime: BigInt) -> Self {
        Self {
            memory,
            prime,
            n_segments: 0u32.into(),
            segment_sizes: HashMap::new(),
            segment_used_sizes: None,
            public_memory_offsets: HashMap::new(),
            n_temp_segments: 0u32.into(),
        }
    }
}
