use num_bigint::BigInt;

/// A value in the cairo vm representing an address in some memory segment. This is meant to be
/// replaced by a real memory address (field element) after the VM finished.
#[derive(Debug, Hash)]
pub struct RelocatableValue {
    pub segment_index: BigInt,
    pub offset: BigInt,
}

impl RelocatableValue {
    pub fn new(segment_index: BigInt, offset: BigInt) -> Self {
        Self {
            segment_index,
            offset,
        }
    }
}
