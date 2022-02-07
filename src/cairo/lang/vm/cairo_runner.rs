use crate::cairo::lang::{
    compiler::program::ProgramBase,
    instances::CairoLayout,
    vm::{
        builtin_runner::BuiltinRunner, memory_dict::MemoryDict,
        memory_segments::MemorySegmentManager, relocatable::RelocatableValue,
    },
};

use num_bigint::BigInt;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct CairoRunner {
    pub program: ProgramBase,
    pub instance: CairoLayout,
    pub builtin_runners: HashMap<String, BuiltinRunner>,
    pub original_steps: Option<BigInt>,
    pub proof_mode: bool,
    pub allow_missing_builtins: bool,
    pub segments: MemorySegmentManager,
    pub segment_offsets: Option<HashMap<BigInt, BigInt>>,
    pub final_pc: Option<RelocatableValue>,
    /// Flag used to ensure a safe use.
    pub run_ended: bool,
    /// Flag used to ensure a safe use.
    pub segments_finalized: bool,
    /// A set of memory addresses accessed by the VM, after relocation of temporary segments into
    /// real ones.
    pub accessed_addresses: Option<HashSet<RelocatableValue>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Builtins {non_existing_builtins:?} are not present in layout \"{layout}\"")]
    BuiltinsNotPresent {
        non_existing_builtins: Vec<String>,
        layout: String,
    },
}

impl CairoRunner {
    pub fn new(
        program: ProgramBase,
        instance: CairoLayout,
        memory: MemoryDict,
        proof_mode: bool,
        allow_missing_builtins: bool,
    ) -> Result<Self, Error> {
        if !allow_missing_builtins {
            let mut non_existing_builtins = vec![];
            for program_builtin in program.builtins.iter() {
                if !instance.builtins.contains_key(program_builtin) {
                    non_existing_builtins.push(program_builtin.to_owned());
                }
            }
            if !non_existing_builtins.is_empty() {
                return Err(Error::BuiltinsNotPresent {
                    non_existing_builtins,
                    layout: instance.layout_name.to_owned(),
                });
            }
        }

        // TODO: implement the following Python code
        //
        // ```python
        // builtin_factories = dict(
        //     output=lambda name, included: OutputBuiltinRunner(included=included),
        //     pedersen=lambda name, included: HashBuiltinRunner(
        //         name=name,
        //         included=included,
        //         ratio=instance.builtins["pedersen"].ratio,
        //         hash_func=pedersen_hash,
        //     ),
        //     range_check=lambda name, included: RangeCheckBuiltinRunner(
        //         included=included,
        //         ratio=instance.builtins["range_check"].ratio,
        //         inner_rc_bound=2 ** 16,
        //         n_parts=instance.builtins["range_check"].n_parts,
        //     ),
        //     ecdsa=lambda name, included: SignatureBuiltinRunner(
        //         name=name,
        //         included=included,
        //         ratio=instance.builtins["ecdsa"].ratio,
        //         process_signature=process_ecdsa,
        //         verify_signature=verify_ecdsa_sig,
        //     ),
        //     bitwise=lambda name, included: BitwiseBuiltinRunner(
        //         included=included, bitwise_builtin=instance.builtins["bitwise"]
        //     ),
        // )
        //
        // for name in instance.builtins:
        //     factory = builtin_factories.get(name)
        //     assert factory is not None, f"The {name} builtin is not supported."
        //     included = name in self.program.builtins
        //     # In proof mode all the builtin_runners are required.
        //     if included or self.proof_mode:
        //         self.builtin_runners[f"{name}_builtin"] = factory(  # type: ignore
        //             name=name, included=included
        //         )
        //
        // supported_builtin_list = list(builtin_factories.keys())
        // err_msg = (
        //     f"The builtins specified by the %builtins directive must be subsequence of "
        //     f"{supported_builtin_list}. Got {self.program.builtins}."
        // )
        // assert is_subsequence(self.program.builtins, supported_builtin_list), err_msg
        // ```

        let segments = MemorySegmentManager::new(memory, program.prime.clone());

        Ok(Self {
            program,
            instance,
            builtin_runners: HashMap::new(),
            original_steps: None,
            proof_mode,
            allow_missing_builtins,
            segments,
            segment_offsets: None,
            final_pc: None,
            run_ended: false,
            segments_finalized: false,
            accessed_addresses: None,
        })
    }
}
