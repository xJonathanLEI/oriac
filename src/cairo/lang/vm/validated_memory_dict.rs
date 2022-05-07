use crate::cairo::lang::vm::{
    memory_dict::{Error as MemoryDictError, MemoryDict},
    relocatable::{MaybeRelocatable, RelocatableValue},
};

use num_bigint::BigInt;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

pub struct ValidationRule {
    pub inner: fn(&MutexGuard<MemoryDict>, &RelocatableValue, &()) -> HashSet<RelocatableValue>,
}

/// A proxy to MemoryDict which validates memory values in specific segments upon writing to it.
///
/// Validation is done according to the validation rules.
#[derive(Debug)]
pub struct ValidatedMemoryDict {
    pub memory: Arc<Mutex<MemoryDict>>,
    /// validation_rules contains a mapping from a segment index to a list of functions (and a tuple
    /// of additional arguments) that may try to validate the value of memory cells in the segment
    /// (sometimes based on other memory cells).
    pub validation_rules: HashMap<BigInt, Vec<(ValidationRule, ())>>,
    /// A list of addresses which were already validated.
    pub validated_addresses: HashSet<RelocatableValue>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    MemoryDictError(MemoryDictError),
    #[error("Unable to lock mutex")]
    MutexLockError,
}

impl Debug for ValidationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "|Closure|")
    }
}

impl ValidatedMemoryDict {
    pub fn new(memory: Arc<Mutex<MemoryDict>>) -> Self {
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
    ) -> Result<Option<MaybeRelocatable>, Error> {
        Ok(self.memory.lock()?.get(addr, default_value))
    }

    pub fn index(&mut self, addr: &MaybeRelocatable) -> Result<MaybeRelocatable, Error> {
        Ok(self.memory.lock()?.index(addr)?)
    }

    pub fn index_set(
        &mut self,
        addr: MaybeRelocatable,
        value: MaybeRelocatable,
    ) -> Result<(), Error> {
        self.memory.lock()?.index_set(addr.clone(), value.clone());
        self.validate_memory_cell(addr, value)?;
        Ok(())
    }

    fn validate_memory_cell(
        &mut self,
        addr: MaybeRelocatable,
        _value: MaybeRelocatable,
    ) -> Result<(), Error> {
        if let MaybeRelocatable::RelocatableValue(addr) = addr {
            if !self.validated_addresses.contains(&addr) {
                if let Some(rules) = self.validation_rules.get(&addr.segment_index) {
                    for (rule, args) in rules.iter() {
                        let validated_addresses =
                            (rule.inner)(&self.memory.as_ref().lock()?, &addr, args);
                        for addr in validated_addresses.into_iter() {
                            self.validated_addresses.insert(addr);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl From<MemoryDictError> for Error {
    fn from(value: MemoryDictError) -> Self {
        Self::MemoryDictError(value)
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Self {
        Self::MutexLockError
    }
}
