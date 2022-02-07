use crate::cairo::lang::{
    compiler::program::ProgramBase,
    instances::CairoLayout,
    vm::{
        builtin_runner::BuiltinRunner,
        memory_dict::MemoryDict,
        memory_segments::MemorySegmentManager,
        relocatable::{MaybeRelocatable, RelocatableValue},
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
    pub program_base: Option<RelocatableValue>,
    pub execution_base: Option<RelocatableValue>,
    pub execution_public_memory: Option<Vec<BigInt>>,
    pub initial_pc: Option<RelocatableValue>,
    pub initial_ap: Option<RelocatableValue>,
    pub initial_fp: Option<RelocatableValue>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Builtins {non_existing_builtins:?} are not present in layout \"{layout}\"")]
    BuiltinsNotPresent {
        non_existing_builtins: Vec<String>,
        layout: String,
    },
    #[error("Missing main().")]
    MissingMain,
    #[error("Segments not initialized.")]
    SegmentsNotInitialized,
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
            program_base: None,
            execution_base: None,
            execution_public_memory: None,
            initial_pc: None,
            initial_ap: None,
            initial_fp: None,
        })
    }

    pub fn initialize_segments(&mut self) {
        // Program segment.
        self.program_base = Some(self.segments.add(None));

        // Execution segment.
        self.execution_base = Some(self.segments.add(None));

        // TODO: implement the following Python code
        //
        // ```python
        // # Builtin segments.
        // for builtin_runner in self.builtin_runners.values():
        //     builtin_runner.initialize_segments(self)
        // ```
    }

    /// Initializes state for running a program from the main() entrypoint. If self.proof_mode ==
    /// True, the execution starts from the start label rather then the main() function.
    ///
    /// Returns the value of the program counter after returning from main.
    pub fn initialize_main_entrypoint(&mut self) -> Result<RelocatableValue, Error> {
        self.execution_public_memory = Some(vec![]);

        let stack: Vec<RelocatableValue> = vec![];
        // TODO: implement the following Python code
        //
        // ```python
        // for builtin_name in self.program.builtins:
        //     builtin_runner = self.builtin_runners.get(f"{builtin_name}_builtin")
        //     if builtin_runner is None:
        //         assert self.allow_missing_builtins, "Missing builtin."
        //         stack += [0]
        //     else:
        //         stack += builtin_runner.initial_stack()
        // ```

        if self.proof_mode {
            // TODO: implement the following Python code
            //
            // ```python
            // # Add the dummy last fp and pc to the public memory, so that the verifier can enforce
            // # [fp - 2] = fp.
            // stack = [self.execution_base + 2, 0] + stack
            // self.execution_public_memory = list(range(len(stack)))
            //
            // assert isinstance(
            //     self.program, Program
            // ), "--proof_mode cannot be used with a StrippedProgram."
            // self.initialize_state(self.program.start, stack)
            // self.initial_fp = self.initial_ap = self.execution_base + 2
            // return self.program_base + self.program.get_label("__end__")
            // ```
            todo!()
        } else {
            let return_fp = self.segments.add(None);

            match self.program.main.clone() {
                Some(main) => self.initialize_function_entrypoint(&main, stack, return_fp),
                None => Err(Error::MissingMain),
            }
        }
    }

    pub fn initialize_function_entrypoint(
        &mut self,
        entrypoint: &BigInt,
        args: Vec<RelocatableValue>,
        return_fp: RelocatableValue,
    ) -> Result<RelocatableValue, Error> {
        let execution_base = self
            .execution_base
            .clone()
            .ok_or(Error::SegmentsNotInitialized)?;

        let end = self.segments.add(None);
        let mut stack = args;
        stack.push(return_fp);
        stack.push(end.clone());

        self.initialize_state(entrypoint, &stack)?;
        self.initial_fp = Some(execution_base + &BigInt::from(stack.len()));
        self.initial_ap = self.initial_fp.clone();
        self.final_pc = Some(end.clone());

        Ok(end)
    }

    pub fn initialize_state(
        &mut self,
        entrypoint: &BigInt,
        stack: &[RelocatableValue],
    ) -> Result<(), Error> {
        let program_base = self
            .program_base
            .clone()
            .ok_or(Error::SegmentsNotInitialized)?;
        let execution_base = self
            .execution_base
            .clone()
            .ok_or(Error::SegmentsNotInitialized)?;

        self.initial_pc = Some(program_base.clone() + entrypoint);

        // Load program.
        self.load_data(
            program_base,
            &self
                .program
                .data
                .iter()
                .map(|item| item.to_owned().into())
                .collect::<Vec<_>>(),
        );

        // Load stack.
        self.load_data(
            execution_base,
            &stack
                .iter()
                .map(|item| item.to_owned().into())
                .collect::<Vec<_>>(),
        );

        Ok(())
    }

    /// Writes data into the memory at address ptr and returns the first address after the data.
    pub fn load_data(
        &mut self,
        ptr: RelocatableValue,
        data: &[MaybeRelocatable],
    ) -> RelocatableValue {
        self.segments.load_data(ptr, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::cairo::lang::compiler::program::Program;

    #[test]
    fn run() {
        let program = serde_json::from_str::<Program>(include_str!(
            "../../../../test-data/artifacts/run_past_end.json"
        ))
        .unwrap();

        let mut runner = CairoRunner::new(
            program.into(),
            CairoLayout::plain_instance(),
            MemoryDict::new(),
            false,
            false,
        )
        .unwrap();

        runner.initialize_segments();
        let end = runner.initialize_main_entrypoint().unwrap();

        dbg!(end);
    }
}
