use crate::cairo::lang::vm::{
    memory_dict::{Error as MemoryDictError, MemoryDict},
    relocatable::{MaybeRelocatable, RelocatableValue},
};

use num_bigint::BigInt;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

/// A proxy to MemoryDict which validates memory values in specific segments upon writing to it.
///
/// Validation is done according to the validation rules.
#[derive(Debug)]
pub struct ValidatedMemoryDict {
    pub memory: Rc<RefCell<MemoryDict>>,
    /// validation_rules contains a mapping from a segment index to a list of functions (and a tuple
    /// of additional arguments) that may try to validate the value of memory cells in the segment
    /// (sometimes based on other memory cells).
    pub validation_rules: HashMap<BigInt, Vec<()>>,
    /// A list of addresses which were already validated.
    pub validated_addresses: HashSet<RelocatableValue>,
}

impl ValidatedMemoryDict {
    pub fn new(memory: Rc<RefCell<MemoryDict>>) -> Self {
        Self {
            memory,
            validation_rules: HashMap::new(),
            validated_addresses: HashSet::new(),
        }
    }

    pub fn get(
        &mut self,
        addr: &MaybeRelocatable,
        default_value: Option<MaybeRelocatable>,
    ) -> Option<MaybeRelocatable> {
        self.memory.borrow_mut().get(addr, default_value)
    }

    pub fn index(&mut self, addr: &MaybeRelocatable) -> Result<MaybeRelocatable, MemoryDictError> {
        self.memory.borrow_mut().index(addr)
    }
}
