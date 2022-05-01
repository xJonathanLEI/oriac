use crate::cairo::lang::vm::{
    cairo_runner::CairoRunner,
    memory_dict::Error as MemoryError,
    memory_segments::{Error as MemorySegmentError, MemorySegmentManager},
    relocatable::{MaybeRelocatable, RelocatableValue},
};

use num_bigint::BigInt;
use std::{any::Any, sync::MutexGuard};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    MemoryError(MemoryError),
    #[error(transparent)]
    MemorySegmentError(MemorySegmentError),
    #[error("unexpected None value")]
    UnexpectedNoneValue,
    #[error("Invalid stop pointer for {builtin_name}. Expected: {expected}, found: {found}")]
    InvalidStopPointer {
        builtin_name: String,
        expected: RelocatableValue,
        found: RelocatableValue,
    },
}

pub trait BuiltinRunner: std::fmt::Debug {
    /// Adds memory segments for the builtin.
    fn initialize_segments(&mut self, segments: &mut MutexGuard<MemorySegmentManager>);

    /// Returns the initial stack elements enforced by this builtin.
    fn initial_stack(&self) -> Vec<MaybeRelocatable>;

    /// Reads values from the end of the stack ([pointer - 1], [pointer - 2], ...), and returns
    /// the updated pointer (e.g., pointer - 2 if two values were read).
    /// This function may also do builtin specific validation of said values.
    fn final_stack(
        &mut self,
        runner: &CairoRunner,
        pointer: MaybeRelocatable,
    ) -> Result<MaybeRelocatable, Error>;

    /// Returns the number of used cells.
    fn get_used_cells(&self, runner: &CairoRunner) -> Result<BigInt, Error>;

    /// Returns the number of used cells and the allocated size, and raises
    /// InsufficientAllocatedCells if there are more used cells than allocated cells.
    fn get_used_cells_and_allocated_size(
        &self,
        runner: &CairoRunner,
    ) -> Result<(BigInt, BigInt), Error>;

    fn as_any(&self) -> &dyn Any;
}

impl From<MemoryError> for Error {
    fn from(value: MemoryError) -> Self {
        Self::MemoryError(value)
    }
}

impl From<MemorySegmentError> for Error {
    fn from(value: MemorySegmentError) -> Self {
        Self::MemorySegmentError(value)
    }
}
