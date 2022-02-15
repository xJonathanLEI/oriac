use crate::cairo::lang::vm::{
    memory_dict::MemoryDict,
    relocatable::{MaybeRelocatable, RelocatableValue},
};

use num_bigint::BigInt;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// Manages the list of memory segments, and allows relocating them once their sizes are known.
#[derive(Debug)]
pub struct MemorySegmentManager {
    pub memory: Rc<RefCell<MemoryDict>>,
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Memory has to be frozen before calculating effective size.")]
    MemoryNotFrozen,
}

impl MemorySegmentManager {
    pub fn new(memory: Rc<RefCell<MemoryDict>>, prime: BigInt) -> Self {
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

    /// Adds a new segment and returns its starting location as a RelocatableValue. If size is not
    /// None the segment is finalized with the given size.
    pub fn add(&mut self, size: Option<BigInt>) -> RelocatableValue {
        let segment_index = self.n_segments.clone();
        self.n_segments += BigInt::from(1);

        if let Some(size) = size {
            self.finalize(segment_index.clone(), Some(size), vec![]);
        }

        RelocatableValue::new(segment_index, 0u32.into())
    }

    /// Writes the following information for the given segment:
    /// * size - The size of the segment (to be used in relocate_segments).
    /// * public_memory - A list of offsets for memory cells that will be considered as public
    /// memory.
    pub fn finalize(
        &mut self,
        segment_index: BigInt,
        size: Option<BigInt>,
        public_memory: Vec<[BigInt; 2]>,
    ) {
        if let Some(size) = size {
            self.segment_sizes.insert(segment_index.clone(), size);
        }

        self.public_memory_offsets
            .insert(segment_index, public_memory);
    }

    /// Computes the current used size of the segments, and caches it. include_tmp_segments should
    /// be used for tests only.
    pub fn compute_effective_sizes(&mut self, include_tmp_segments: bool) -> Result<(), Error> {
        if self.segment_used_sizes.is_some() {
            // segment_sizes is already cached.
            return Ok(());
        }

        if !self.memory.borrow().is_frozen() {
            return Err(Error::MemoryNotFrozen);
        }

        let first_segment_index = if include_tmp_segments {
            -&self.n_temp_segments
        } else {
            BigInt::from(0u32)
        };
        self.segment_used_sizes = {
            let mut segment_used_sizes = HashMap::<BigInt, BigInt>::new();

            let mut index = first_segment_index;
            while index < self.n_segments {
                segment_used_sizes.insert(index.clone(), BigInt::from(0));
                index += BigInt::from(1u32);
            }

            for (addr, _) in self.memory.borrow().data.iter() {
                match addr {
                    // TODO: check if memory addresses are ALWAYS `Relocatable`
                    MaybeRelocatable::Int(_) => {
                        panic!("Expected memory address to be relocatable value. Found: int")
                    }
                    MaybeRelocatable::RelocatableValue(addr) => {
                        // TODO: check if unwrap() is safe here
                        let previous_max_size = segment_used_sizes
                            .get(&addr.segment_index)
                            .unwrap()
                            .to_owned();

                        segment_used_sizes.insert(
                            addr.segment_index.to_owned(),
                            previous_max_size.max(addr.offset.to_owned() + &BigInt::from(1u32)),
                        );
                    }
                }
            }

            Some(segment_used_sizes)
        };

        Ok(())
    }

    /// Writes data into the memory at address ptr and returns the first address after the data.
    pub fn load_data(
        &mut self,
        ptr: MaybeRelocatable,
        data: &[MaybeRelocatable],
    ) -> MaybeRelocatable {
        for (i, v) in data.iter().enumerate() {
            self.memory
                .borrow_mut()
                .index_set(ptr.clone() + &BigInt::from(i), v.to_owned());
        }
        ptr + &BigInt::from(data.len())
    }
}
