use num_bigint::BigInt;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum MaybeRelocatable {
    Int(BigInt),
    RelocatableValue(RelocatableValue),
}

/// A value in the cairo vm representing an address in some memory segment. This is meant to be
/// replaced by a real memory address (field element) after the VM finished.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RelocatableValue {
    pub segment_index: BigInt,
    pub offset: BigInt,
}

impl From<BigInt> for MaybeRelocatable {
    fn from(value: BigInt) -> Self {
        MaybeRelocatable::Int(value)
    }
}

impl From<RelocatableValue> for MaybeRelocatable {
    fn from(value: RelocatableValue) -> Self {
        MaybeRelocatable::RelocatableValue(value)
    }
}

impl RelocatableValue {
    pub fn new(segment_index: BigInt, offset: BigInt) -> Self {
        Self {
            segment_index,
            offset,
        }
    }
}

impl std::ops::Add<&BigInt> for RelocatableValue {
    type Output = RelocatableValue;

    fn add(self, rhs: &BigInt) -> Self::Output {
        RelocatableValue::new(self.segment_index, self.offset + rhs)
    }
}
