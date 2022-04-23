use crate::cairo::lang::vm::{
    builtin_runner::{BuiltinRunner, Error as BuiltinRunnerError},
    cairo_runner::CairoRunner,
    memory_segments::MemorySegmentManager,
    relocatable::{MaybeRelocatable, RelocatableValue},
};

use num_bigint::BigInt;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PublicMemoryPage {
    pub start: BigInt,
    pub size: BigInt,
}

#[derive(Debug)]
pub struct OutputBuiltinRunner {
    pub included: bool,
    /// A map from page id to PublicMemoryPage. See add_page() for more details.
    pub pages: HashMap<BigInt, PublicMemoryPage>,
    /// A map from attribute name to its value. Serialized as part of the additional data of the
    /// builtin.
    pub attributes: HashMap<String, ()>,
    pub base: Option<RelocatableValue>,
    pub stop_ptr: Option<RelocatableValue>,
}

impl OutputBuiltinRunner {
    pub fn new(included: bool) -> Self {
        Self {
            included,
            pages: HashMap::new(),
            attributes: HashMap::new(),
            base: None,
            stop_ptr: None,
        }
    }
}

impl BuiltinRunner for OutputBuiltinRunner {
    fn initialize_segments(&mut self, segments: &mut MemorySegmentManager) {
        self.base = Some(segments.add(None));
        self.stop_ptr = None;
    }

    fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        if self.included {
            // TODO: check if it's safe to unwrap here
            vec![self.base.clone().unwrap().into()]
        } else {
            vec![]
        }
    }

    fn final_stack(
        &mut self,
        runner: &CairoRunner,
        pointer: MaybeRelocatable,
    ) -> Result<MaybeRelocatable, BuiltinRunnerError> {
        if self.included {
            let pointer_minus_one = pointer - &BigInt::from(1u32).into();

            let stop_ptr = {
                // We're forcing the conversion to `RelocatableValue` as the Python code seems to
                // assume it's always the case.
                match runner.memory.borrow_mut().index(&pointer_minus_one)? {
                    MaybeRelocatable::RelocatableValue(value) => value,
                    MaybeRelocatable::Int(_) => panic!("expecting RelocatableValue"),
                }
            };
            self.stop_ptr = Some(stop_ptr.clone());
            let used = self.get_used_cells(runner)?;
            {
                let expected = self
                    .base
                    .clone()
                    .ok_or(BuiltinRunnerError::UnexpectedNoneValue)?
                    + &used;
                let found = stop_ptr;
                if found != expected {
                    return Err(BuiltinRunnerError::InvalidStopPointer {
                        builtin_name: String::from("output"),
                        expected,
                        found,
                    });
                }
            }

            Ok(pointer_minus_one)
        } else {
            self.stop_ptr = self.base.clone();
            Ok(pointer)
        }
    }

    fn get_used_cells(&self, runner: &CairoRunner) -> Result<BigInt, BuiltinRunnerError> {
        let size = runner.segments.get_segment_used_size(
            self.base
                .clone()
                .ok_or(BuiltinRunnerError::UnexpectedNoneValue)?
                .segment_index,
        );

        Ok(size?)
    }
}
