use num_bigint::BigInt;
use std::collections::HashMap;

use crate::cairo::lang::vm::relocatable::{MaybeRelocatable, RelocatableValue};

/// Dictionary used for VM memory. Adds the following checks:
/// * Checks that all memory addresses are valid.
/// * getitem: Checks that the memory address is initialized.
/// * setitem: Checks that memory value is not changed.
#[derive(Debug)]
pub struct MemoryDict {
    pub data: HashMap<MaybeRelocatable, MaybeRelocatable>,
    pub frozen: bool,
    /// A dict of segment relocation rules mapping a segment index to a RelocatableValue. See
    /// add_relocation_rule for more details.
    pub relocation_rules: HashMap<BigInt, RelocatableValue>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{name} must be nonnegative. Got {num}.")]
    NegativeValue { name: &'static str, num: BigInt },
    #[error("Unknown value for memory cell at address {addr}.")]
    UnknownMemory { addr: MaybeRelocatable },
    #[error("Memory is frozen and cannot be changed.")]
    MemoryFrozen,
}

impl MemoryDict {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            frozen: false,
            relocation_rules: HashMap::new(),
        }
    }

    pub fn get(
        &mut self,
        addr: &MaybeRelocatable,
        default_value: Option<MaybeRelocatable>,
    ) -> Option<MaybeRelocatable> {
        let mut value = match self.data.get(addr).cloned() {
            Some(value) => Some(value),
            None => default_value,
        };

        if let Some(relocatable) = value {
            value = Some(self.relocate_value(relocatable));
        }

        value
    }

    // Cannot use the `Index` trait due to return type and &mut
    pub fn index(&mut self, addr: &MaybeRelocatable) -> Result<MaybeRelocatable, Error> {
        self.check_element(addr.to_owned(), "Memory address")?;

        let value = self
            .data
            .get(addr)
            .ok_or_else(|| Error::UnknownMemory {
                addr: addr.to_owned(),
            })?
            .to_owned();

        Ok(self.relocate_value(value))
    }

    pub fn index_set(&mut self, addr: MaybeRelocatable, value: MaybeRelocatable) {
        self.data.insert(addr, value);
    }

    /// Freezes the memory - no changes can be made from now on.
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Relocates a value according to the relocation rules.
    ///
    /// The original value is returned if the relocation rules do not apply to value.
    pub fn relocate_value(&mut self, value: MaybeRelocatable) -> MaybeRelocatable {
        match value {
            MaybeRelocatable::Int(_) => value,
            MaybeRelocatable::RelocatableValue(value) => {
                let segment_idx = value.clone().segment_index;
                if segment_idx >= BigInt::from(0u32) {
                    return value.into();
                }

                match self.relocation_rules.get(&segment_idx).cloned() {
                    Some(relocation) => self.relocate_value(relocation.into()) + &value.offset,
                    None => value.into(),
                }
            }
        }
    }

    /// Relocates the memory according to the relocation rules and clears self.relocation_rules.
    #[allow(clippy::needless_collect)] // Need some refactoring to work around the issue
    pub fn relocate_memory(&mut self) -> Result<(), Error> {
        if self.frozen {
            return Err(Error::MemoryFrozen);
        }

        if self.relocation_rules.is_empty() {
            return Ok(());
        }

        self.data = {
            let items = self
                .data
                .iter()
                .map(|(addr, value)| (addr.to_owned(), value.to_owned()))
                .collect::<Vec<_>>();

            items
                .into_iter()
                .map(|(addr, value)| (self.relocate_value(addr), self.relocate_value(value)))
                .collect::<HashMap<_, _>>()
        };
        self.relocation_rules.clear();

        Ok(())
    }

    /// Checks that num is a valid Cairo value: positive int or relocatable. Currently, does not
    /// check that value < prime.
    fn check_element<T>(&self, num: T, name: &'static str) -> Result<(), Error>
    where
        T: Into<MaybeRelocatable>,
    {
        if let MaybeRelocatable::Int(num) = num.into() {
            if num < BigInt::from(0) {
                Err(Error::NegativeValue { name, num })
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}

impl Default for MemoryDict {
    fn default() -> Self {
        Self::new()
    }
}
