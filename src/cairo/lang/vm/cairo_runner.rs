use crate::cairo::lang::{
    compiler::program::Program,
    instances::CairoLayout,
    vm::{
        builtin_runner::BuiltinRunner,
        memory_dict::MemoryDict,
        memory_segments::MemorySegmentManager,
        relocatable::{MaybeRelocatable, RelocatableValue},
        utils::RunResources,
        vm_core::{RunContext, VirtualMachine, VirtualMachineError},
        vm_exceptions::VmException,
    },
};

use num_bigint::BigInt;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

#[derive(Debug)]
pub struct CairoRunner {
    pub program: Rc<Program>,
    pub instance: CairoLayout,
    pub builtin_runners: Rc<HashMap<String, BuiltinRunner>>,
    pub original_steps: Option<BigInt>,
    pub proof_mode: bool,
    pub allow_missing_builtins: bool,
    pub memory: Rc<RefCell<MemoryDict>>,
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
    pub vm: Option<VirtualMachine>,
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
    #[error("Function entrypoint not initialized.")]
    FunctionEntrypointNotInitialized,
    #[error("State not initialized.")]
    StateNotInitialized,
    #[error("VM not initialized.")]
    VmNotInitialized,
    #[error(transparent)]
    VmError(VmException),
    #[error(transparent)]
    VirtualMachineError(VirtualMachineError),
}

impl CairoRunner {
    pub fn new(
        program: Rc<Program>,
        instance: CairoLayout,
        memory: MemoryDict,
        proof_mode: bool,
        allow_missing_builtins: bool,
    ) -> Result<Self, Error> {
        if !allow_missing_builtins {
            let mut non_existing_builtins = vec![];
            for program_builtin in program.builtins().iter() {
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

        let memory = Rc::new(RefCell::new(memory));

        let segments = MemorySegmentManager::new(memory.clone(), program.prime().clone());

        Ok(Self {
            program,
            instance,
            builtin_runners: Rc::new(HashMap::new()),
            original_steps: None,
            proof_mode,
            allow_missing_builtins,
            memory,
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
            vm: None,
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

            match self.program.main() {
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
        let end = self.segments.add(None);
        let mut stack = args;
        stack.push(return_fp);
        stack.push(end.clone());

        self.initialize_state(entrypoint, &stack)?;
        self.initial_fp = Some(self.execution_base()?.to_owned() + &BigInt::from(stack.len()));
        self.initial_ap = self.initial_fp.clone();
        self.final_pc = Some(end.clone());

        Ok(end)
    }

    pub fn initialize_state(
        &mut self,
        entrypoint: &BigInt,
        stack: &[RelocatableValue],
    ) -> Result<(), Error> {
        self.initial_pc = Some(self.program_base()?.to_owned() + entrypoint);

        // Load program.
        self.load_data(
            self.program_base()?.to_owned().into(),
            &self
                .program
                .data()
                .iter()
                .map(|item| item.to_owned().into())
                .collect::<Vec<_>>(),
        );

        // Load stack.
        self.load_data(
            self.execution_base()?.to_owned().into(),
            &stack
                .iter()
                .map(|item| item.to_owned().into())
                .collect::<Vec<_>>(),
        );

        Ok(())
    }

    pub fn initialize_vm(
        &mut self,
        hint_locals: HashMap<String, ()>,
        static_locals: Option<HashMap<String, ()>>,
    ) -> Result<(), Error> {
        let context = RunContext::new(
            self.memory.clone(),
            self.initial_pc()?.to_owned().into(),
            self.initial_ap()?.to_owned().into(),
            self.initial_fp()?.to_owned().into(),
            self.program.prime().clone(),
        );

        let static_locals = static_locals.unwrap_or_else(HashMap::new);

        self.vm = Some(VirtualMachine::new(
            self.program.clone(),
            Rc::new(RefCell::new(context)),
            hint_locals,
            Some(static_locals),
            Some(self.builtin_runners.clone()),
            Some(self.program_base()?.to_owned().into()),
        ));

        // TODO: implement the following Python code
        //
        // ```python
        // for builtin_runner in self.builtin_runners.values():
        //     builtin_runner.add_validation_rules(self)
        //     builtin_runner.add_auto_deduction_rules(self)
        //
        // self.vm.validate_existing_memory()
        // ```

        Ok(())
    }

    /// Runs the VM until pc reaches 'addr', and stop right before that instruction is executed.
    pub fn run_until_pc(
        &mut self,
        addr: MaybeRelocatable,
        run_resources: Option<RunResources>,
    ) -> Result<(), Error> {
        let mut run_resources = run_resources.unwrap_or(RunResources { n_steps: None });

        while self.vm()?.run_context.borrow().pc != addr && !run_resources.consumed() {
            self.vm_step()?;
            run_resources.consume_step();
        }

        if self.vm()?.run_context.borrow().pc != addr {
            // TODO: implement `as_vm_exception` on `vm` and switch over
            //       Error: End of program was not reached
            Err(Error::VmError(VmException {}))
        } else {
            Ok(())
        }
    }

    pub fn vm_step(&mut self) -> Result<(), Error> {
        if &self.vm()?.run_context.borrow().pc == self.final_pc()? {
            // TODO: implement `as_vm_exception` on `vm` and switch over
            //       Error: Execution reached the end of the program.
            return Err(Error::VmError(VmException {}));
        }

        self.vm_mut()?.step()?;

        Ok(())
    }

    /// Writes data into the memory at address ptr and returns the first address after the data.
    pub fn load_data(
        &mut self,
        ptr: MaybeRelocatable,
        data: &[MaybeRelocatable],
    ) -> MaybeRelocatable {
        self.segments.load_data(ptr, data)
    }

    fn program_base(&self) -> Result<&RelocatableValue, Error> {
        self.program_base
            .as_ref()
            .ok_or(Error::SegmentsNotInitialized)
    }

    fn execution_base(&self) -> Result<&RelocatableValue, Error> {
        self.execution_base
            .as_ref()
            .ok_or(Error::SegmentsNotInitialized)
    }

    fn final_pc(&self) -> Result<&RelocatableValue, Error> {
        self.final_pc
            .as_ref()
            .ok_or(Error::FunctionEntrypointNotInitialized)
    }

    fn initial_pc(&self) -> Result<&RelocatableValue, Error> {
        self.initial_pc.as_ref().ok_or(Error::StateNotInitialized)
    }

    fn initial_ap(&self) -> Result<&RelocatableValue, Error> {
        self.initial_ap.as_ref().ok_or(Error::StateNotInitialized)
    }

    fn initial_fp(&self) -> Result<&RelocatableValue, Error> {
        self.initial_fp.as_ref().ok_or(Error::StateNotInitialized)
    }

    fn vm(&self) -> Result<&VirtualMachine, Error> {
        self.vm.as_ref().ok_or(Error::VmNotInitialized)
    }

    fn vm_mut(&mut self) -> Result<&mut VirtualMachine, Error> {
        self.vm.as_mut().ok_or(Error::VmNotInitialized)
    }
}

impl From<VirtualMachineError> for Error {
    fn from(value: VirtualMachineError) -> Self {
        Self::VirtualMachineError(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::cairo::lang::compiler::program::FullProgram;

    #[test]
    fn run() {
        let program = serde_json::from_str::<FullProgram>(include_str!(
            "../../../../test-data/artifacts/run_past_end.json"
        ))
        .unwrap();

        let mut runner = CairoRunner::new(
            Rc::new(program.into()),
            CairoLayout::plain_instance(),
            MemoryDict::new(),
            false,
            false,
        )
        .unwrap();

        runner.initialize_segments();
        let end = runner.initialize_main_entrypoint().unwrap();

        runner.initialize_vm(HashMap::new(), None).unwrap();

        runner.run_until_pc(end.into(), None).unwrap();
    }
}
