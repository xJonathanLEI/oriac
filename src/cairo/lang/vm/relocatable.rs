use std::fmt::Display;

use num_bigint::BigInt;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
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

impl std::ops::Add<&MaybeRelocatable> for MaybeRelocatable {
    type Output = MaybeRelocatable;

    fn add(self, rhs: &MaybeRelocatable) -> Self::Output {
        match self {
            MaybeRelocatable::Int(lhs) => match rhs {
                MaybeRelocatable::Int(rhs) => MaybeRelocatable::Int(lhs + rhs),
                MaybeRelocatable::RelocatableValue(rhs) => {
                    MaybeRelocatable::RelocatableValue(rhs.to_owned() + &lhs)
                }
            },
            MaybeRelocatable::RelocatableValue(lhs) => match rhs {
                MaybeRelocatable::Int(rhs) => MaybeRelocatable::RelocatableValue(lhs + rhs),
                MaybeRelocatable::RelocatableValue(rhs) => {
                    panic!("Cannot add two relocatable values: {lhs} + {rhs}.")
                }
            },
        }
    }
}

impl std::ops::Sub<&MaybeRelocatable> for MaybeRelocatable {
    type Output = MaybeRelocatable;

    fn sub(self, rhs: &MaybeRelocatable) -> Self::Output {
        match self {
            MaybeRelocatable::Int(_) => {
                panic!("unsupported operand type(s) for -: 'int' and 'RelocatableValue'")
            }
            MaybeRelocatable::RelocatableValue(lhs) => lhs - rhs,
        }
    }
}

impl std::ops::Add<&BigInt> for MaybeRelocatable {
    type Output = MaybeRelocatable;

    fn add(self, rhs: &BigInt) -> Self::Output {
        match self {
            MaybeRelocatable::Int(int) => MaybeRelocatable::Int(int + rhs),
            MaybeRelocatable::RelocatableValue(value) => {
                MaybeRelocatable::RelocatableValue(value + rhs)
            }
        }
    }
}

impl std::ops::Rem<&BigInt> for MaybeRelocatable {
    type Output = MaybeRelocatable;

    fn rem(self, rhs: &BigInt) -> Self::Output {
        match self {
            MaybeRelocatable::Int(int) => MaybeRelocatable::Int(int % rhs),
            MaybeRelocatable::RelocatableValue(value) => {
                MaybeRelocatable::RelocatableValue(value % rhs)
            }
        }
    }
}

impl std::cmp::PartialEq<BigInt> for MaybeRelocatable {
    fn eq(&self, other: &BigInt) -> bool {
        match self {
            MaybeRelocatable::Int(int) => int == other,
            &MaybeRelocatable::RelocatableValue(_) => false,
        }
    }
}

impl std::cmp::PartialEq<RelocatableValue> for MaybeRelocatable {
    fn eq(&self, other: &RelocatableValue) -> bool {
        match self {
            MaybeRelocatable::Int(_) => false,
            MaybeRelocatable::RelocatableValue(value) => value == other,
        }
    }
}

impl Display for MaybeRelocatable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaybeRelocatable::Int(value) => write!(f, "{}", value),
            MaybeRelocatable::RelocatableValue(value) => write!(f, "{}", value),
        }
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

impl std::ops::Sub<&MaybeRelocatable> for RelocatableValue {
    type Output = MaybeRelocatable;

    fn sub(self, rhs: &MaybeRelocatable) -> Self::Output {
        match rhs {
            MaybeRelocatable::Int(rhs) => MaybeRelocatable::RelocatableValue(
                RelocatableValue::new(self.segment_index, self.offset - rhs),
            ),
            MaybeRelocatable::RelocatableValue(rhs) => {
                if self.segment_index != rhs.segment_index {
                    // TODO: switch to proper error handling?
                    panic!(
                        "Can only subtract two relocatable values of the same segment ({} != {}).",
                        self.segment_index, rhs.segment_index
                    );
                }

                MaybeRelocatable::Int(self.offset - &rhs.offset)
            }
        }
    }
}

impl std::ops::Rem<&BigInt> for RelocatableValue {
    type Output = RelocatableValue;

    fn rem(self, rhs: &BigInt) -> Self::Output {
        RelocatableValue::new(self.segment_index, self.offset % rhs)
    }
}

impl Display for RelocatableValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.segment_index, self.offset)
    }
}
