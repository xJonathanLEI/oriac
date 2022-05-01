use crate::{
    cairo::lang::{
        compiler::program::Program,
        instances::CairoLayout,
        vm::{
            builtin_runner::{BuiltinRunner, Error as BuiltinRunnerError},
            memory_dict::{Error as MemoryDictError, MemoryDict},
            memory_segments::{Error as MemorySegmentError, MemorySegmentManager},
            output_builtin_runner::OutputBuiltinRunner,
            relocatable::{MaybeRelocatable, RelocatableValue},
            utils::RunResources,
            vm_core::{RunContext, VirtualMachine, VirtualMachineError},
            vm_exceptions::VmException,
        },
    },
    hint_support::HintLocals,
};

use num_bigint::BigInt;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::{Arc, Mutex},
};

pub type BuiltinRunnerMap = HashMap<String, Box<dyn BuiltinRunner>>;

type BuiltinRunnerFactory = dyn Fn(&str, bool) -> Box<dyn BuiltinRunner>;

#[derive(Debug)]
pub struct CairoRunner {
    pub program: Rc<Program>,
    pub instance: CairoLayout,
    pub builtin_runners: Rc<RefCell<BuiltinRunnerMap>>,
    pub original_steps: Option<BigInt>,
    pub proof_mode: bool,
    pub allow_missing_builtins: bool,
    pub memory: Arc<Mutex<MemoryDict>>,
    pub segments: Arc<Mutex<MemorySegmentManager>>,
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
    #[error("The {name} builtin is not supported.")]
    BuiltinNotSupported { name: String },
    #[error("The builtins specified by the %builtins directive must be subsequence of {supported_builtin_list:?}. Got {program_builtins:?}.")]
    BuiltinsNotSubsequence {
        supported_builtin_list: Vec<String>,
        program_builtins: Vec<String>,
    },
    #[error("Missing builtin.")]
    MissingBuiltin,
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
    MemoryDictError(MemoryDictError),
    #[error(transparent)]
    MemorySegmentError(MemorySegmentError),
    #[error(transparent)]
    VmError(VmException),
    #[error(transparent)]
    VirtualMachineError(VirtualMachineError),
    #[error(transparent)]
    BuiltinRunnerError(BuiltinRunnerError),
    #[error("end_run called twice")]
    EndRunCalledTwice,
    #[error("Run must be ended before calling read_return_values.")]
    RunNotEnded,
    #[error("The stop pointer of the missing builtin \"{builtin_name}\" must be 0.")]
    NonZeroMissingBuiltinStopPointer { builtin_name: String },
    #[error("Cannot add the return values to the public memory after segment finalization.")]
    CannotAddReturnValuesAfterSegmentFinalization,
    #[error("Unexpected builtin type")]
    UnexpectedBuiltinType,
    #[error("Unexpected None value")]
    UnexpectedNoneValue,
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

        let mut builtin_runners = HashMap::new();

        let mut builtin_factories: HashMap<String, Box<BuiltinRunnerFactory>> = HashMap::new();
        builtin_factories.insert(String::from("output"), Box::new(output_builtin_factory));
        builtin_factories.insert(String::from("pedersen"), Box::new(pedersen_builtin_factory));
        builtin_factories.insert(
            String::from("range_check"),
            Box::new(range_check_builtin_factory),
        );
        builtin_factories.insert(String::from("ecdsa"), Box::new(ecdsa_builtin_factory));
        builtin_factories.insert(String::from("bitwise"), Box::new(bitwise_builtin_factory));

        // TODO: implement the following builtin factories
        //
        // ```python
        // builtin_factories = dict(
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
        // ```

        let supported_builtin_list: Vec<String> = builtin_factories.keys().cloned().collect();
        if program
            .builtins()
            .iter()
            .any(|item| !supported_builtin_list.contains(item))
        {
            return Err(Error::BuiltinsNotSubsequence {
                supported_builtin_list,
                program_builtins: program.builtins().to_vec(),
            });
        }

        for (name, _) in instance.builtins.iter() {
            let factory = builtin_factories
                .get(name)
                .ok_or(Error::BuiltinNotSupported {
                    name: name.to_owned(),
                })?;
            let included = program.builtins().contains(name);

            // In proof mode all the builtin_runners are required.
            if included || proof_mode {
                builtin_runners.insert(format!("{}_builtin", &name), factory(name, included));
            }
        }

        let memory = Arc::new(Mutex::new(memory));

        let segments = MemorySegmentManager::new(memory.clone(), program.prime().clone());

        Ok(Self {
            program,
            instance,
            builtin_runners: Rc::new(RefCell::new(builtin_runners)),
            original_steps: None,
            proof_mode,
            allow_missing_builtins,
            memory,
            segments: Arc::new(Mutex::new(segments)),
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
        self.program_base = Some(self.segments.lock().unwrap().add(None));

        // Execution segment.
        self.execution_base = Some(self.segments.lock().unwrap().add(None));

        // Builtin segments.
        for builtin_runner in self.builtin_runners.borrow_mut().values_mut() {
            builtin_runner.initialize_segments(&mut self.segments.lock().unwrap());
        }
    }

    /// Initializes state for running a program from the main() entrypoint. If self.proof_mode ==
    /// True, the execution starts from the start label rather then the main() function.
    ///
    /// Returns the value of the program counter after returning from main.
    pub fn initialize_main_entrypoint(&mut self) -> Result<RelocatableValue, Error> {
        self.execution_public_memory = Some(vec![]);

        let mut stack: Vec<MaybeRelocatable> = vec![];
        for builtin_name in self.program.builtins().iter() {
            match self
                .builtin_runners
                .borrow_mut()
                .get_mut(&format!("{}_builtin", builtin_name))
            {
                Some(builtin_runner) => {
                    for item in builtin_runner.initial_stack().into_iter() {
                        stack.push(item);
                    }
                }
                None => {
                    if !self.allow_missing_builtins {
                        return Err(Error::MissingBuiltin);
                    } else {
                        stack.push(MaybeRelocatable::Int(BigInt::from(0u8)));
                    }
                }
            }
        }

        if self.proof_mode {
            // TODO: implement the following Python code
            //
            // ```python
            // # Add the dummy last fp and pc to the public memory, so that the verifier can enforce
            // # [fp - 2] = fp.
            // stack_prefix: List[MaybeRelocatable] = [self.execution_base + 2, 0]
            // stack = stack_prefix + stack
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
            let return_fp = self.segments.lock().unwrap().add(None);

            match self.program.main() {
                Some(main) => self.initialize_function_entrypoint(&main, stack, return_fp.into()),
                None => Err(Error::MissingMain),
            }
        }
    }

    pub fn initialize_function_entrypoint(
        &mut self,
        entrypoint: &BigInt,
        args: Vec<MaybeRelocatable>,
        return_fp: MaybeRelocatable,
    ) -> Result<RelocatableValue, Error> {
        let end = self.segments.lock().unwrap().add(None);
        let mut stack = args;
        stack.push(return_fp);
        stack.push(end.clone().into());

        self.initialize_state(entrypoint, &stack)?;
        self.initial_fp = Some(self.execution_base()?.to_owned() + &BigInt::from(stack.len()));
        self.initial_ap = self.initial_fp.clone();
        self.final_pc = Some(end.clone());

        Ok(end)
    }

    pub fn initialize_state(
        &mut self,
        entrypoint: &BigInt,
        stack: &[MaybeRelocatable],
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
            &stack.iter().map(|item| item.to_owned()).collect::<Vec<_>>(),
        );

        Ok(())
    }

    #[allow(unused)]
    pub fn initialize_vm(
        &mut self,
        hint_locals: HashMap<String, ()>,
        static_locals: (),
    ) -> Result<(), Error> {
        let context = RunContext::new(
            self.memory.clone(),
            self.initial_pc()?.to_owned().into(),
            self.initial_ap()?.to_owned().into(),
            self.initial_fp()?.to_owned().into(),
            self.program.prime().clone(),
        );

        let static_locals = HintLocals {
            segments: self.segments.clone(),
        };

        self.vm = Some(VirtualMachine::new(
            self.program.clone(),
            Rc::new(RefCell::new(context)),
            hint_locals,
            static_locals,
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

    pub fn end_run(
        &mut self,
        disable_trace_padding: bool,
        disable_finalize_all: bool,
    ) -> Result<(), Error> {
        if self.run_ended {
            return Err(Error::EndRunCalledTwice);
        }

        self.accessed_addresses = {
            let mut vm_memory = self.memory.lock().unwrap();
            Some(
                self.vm()?
                    .accessed_addresses
                    .iter()
                    .map(|addr| match vm_memory.relocate_value(addr.to_owned()) {
                        MaybeRelocatable::Int(_) => {
                            panic!("unexpected variant: MaybeRelocatable::Int")
                        }
                        MaybeRelocatable::RelocatableValue(value) => value,
                    })
                    .collect::<HashSet<_>>(),
            )
        };
        self.memory.lock().unwrap().relocate_memory()?;
        self.vm_mut()?.end_run()?;

        if disable_finalize_all {
            // For tests.
            return Ok(());
        }

        // Freeze to enable caching; No changes in memory should be made from now on.
        self.memory.lock().unwrap().freeze();
        // Deduce the size of each segment from its usage.
        self.segments
            .lock()
            .unwrap()
            .compute_effective_sizes(false)?;

        if self.proof_mode && !disable_trace_padding {
            // TODO: implement the following Python code
            //
            // ```python
            // self.run_until_next_power_of_2()
            // while not self.check_used_cells():
            //     self.run_for_steps(1)
            //     self.run_until_next_power_of_2()
            // ```

            todo!()
        }

        self.run_ended = true;

        Ok(())
    }

    /// Reads builtin return values (end pointers) and adds them to the public memory.
    /// Note: end_run() must precede a call to this method.
    pub fn read_return_values(&self) -> Result<(), Error> {
        if !self.run_ended {
            return Err(Error::RunNotEnded);
        }

        let mut pointer = self.vm()?.run_context.borrow().ap.clone();
        for builtin_name in self.program.builtins().iter().rev() {
            match self
                .builtin_runners
                .borrow_mut()
                .get_mut(&format!("{}_builtin", builtin_name))
            {
                Some(builtin_runner) => {
                    pointer = builtin_runner.final_stack(self, pointer)?;
                }
                None => {
                    if !self.allow_missing_builtins {
                        return Err(Error::MissingBuiltin);
                    }
                    pointer = pointer - &BigInt::from(1u32).into();
                    if self.memory.lock().unwrap().index(&pointer)?
                        != MaybeRelocatable::Int(BigInt::from(0u32))
                    {
                        return Err(Error::NonZeroMissingBuiltinStopPointer {
                            builtin_name: builtin_name.to_owned(),
                        });
                    }
                }
            }
        }

        if self.segments_finalized {
            return Err(Error::CannotAddReturnValuesAfterSegmentFinalization);
        }

        // TODO: implement the following Python code
        //
        // ```python
        // # Add return values to public memory.
        // self.execution_public_memory += list(
        //     range(pointer - self.execution_base, self.vm.run_context.ap - self.execution_base)
        // )
        // ```

        Ok(())
    }

    /// Writes data into the memory at address ptr and returns the first address after the data.
    pub fn load_data(
        &mut self,
        ptr: MaybeRelocatable,
        data: &[MaybeRelocatable],
    ) -> MaybeRelocatable {
        self.segments.lock().unwrap().load_data(ptr, data)
    }

    // TODO: implement `output_callback`
    pub fn print_output(&self) -> Result<(), Error> {
        if let Some(output_runner) = self.builtin_runners.borrow().get("output_builtin") {
            let output_runner = output_runner
                .as_any()
                .downcast_ref::<OutputBuiltinRunner>()
                .ok_or(Error::UnexpectedBuiltinType)?;

            println!("Program output:");

            let (_, size) = output_runner.get_used_cells_and_allocated_size(self)?;
            let mut i = BigInt::from(0u32);
            while i < size {
                match self.memory.lock().unwrap().get(
                    &(output_runner
                        .base
                        .clone()
                        .ok_or(Error::UnexpectedNoneValue)?
                        + &i)
                        .into(),
                    None,
                ) {
                    Some(val) => {
                        println!("  {}", val);
                    }
                    None => {
                        println!("  <missing>");
                    }
                }

                i += BigInt::from(1u32);
            }

            println!();
        }

        Ok(())
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

impl From<MemoryDictError> for Error {
    fn from(value: MemoryDictError) -> Self {
        Self::MemoryDictError(value)
    }
}

impl From<MemorySegmentError> for Error {
    fn from(value: MemorySegmentError) -> Self {
        Self::MemorySegmentError(value)
    }
}

impl From<VirtualMachineError> for Error {
    fn from(value: VirtualMachineError) -> Self {
        Self::VirtualMachineError(value)
    }
}

impl From<BuiltinRunnerError> for Error {
    fn from(value: BuiltinRunnerError) -> Self {
        Self::BuiltinRunnerError(value)
    }
}

fn output_builtin_factory(_name: &str, included: bool) -> Box<dyn BuiltinRunner> {
    Box::new(OutputBuiltinRunner::new(included))
}

fn pedersen_builtin_factory(_name: &str, _included: bool) -> Box<dyn BuiltinRunner> {
    todo!()
}

fn range_check_builtin_factory(_name: &str, _included: bool) -> Box<dyn BuiltinRunner> {
    todo!()
}

fn ecdsa_builtin_factory(_name: &str, _included: bool) -> Box<dyn BuiltinRunner> {
    todo!()
}

fn bitwise_builtin_factory(_name: &str, _included: bool) -> Box<dyn BuiltinRunner> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::cairo::lang::compiler::program::FullProgram;

    #[test]
    fn test_run_past_end() {
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

        runner.initialize_vm(HashMap::new(), ()).unwrap();

        runner.run_until_pc(end.into(), None).unwrap();

        runner.end_run(false, false).unwrap();

        runner.read_return_values().unwrap();
    }

    #[test]
    fn test_bad_stop_ptr() {
        let program = serde_json::from_str::<FullProgram>(include_str!(
            "../../../../test-data/artifacts/bad_stop_ptr.json"
        ))
        .unwrap();

        let mut runner = CairoRunner::new(
            Rc::new(program.into()),
            CairoLayout::small_instance(),
            MemoryDict::new(),
            false,
            false,
        )
        .unwrap();

        runner.initialize_segments();
        let end = runner.initialize_main_entrypoint().unwrap();

        runner.initialize_vm(HashMap::new(), ()).unwrap();

        runner.run_until_pc(end.into(), None).unwrap();

        runner.end_run(false, false).unwrap();

        match runner.read_return_values() {
            Err(Error::BuiltinRunnerError(BuiltinRunnerError::InvalidStopPointer {
                builtin_name,
                expected,
                found,
            })) => {
                assert_eq!(builtin_name, "output");
                assert_eq!(
                    expected,
                    RelocatableValue {
                        segment_index: BigInt::from(2u8),
                        offset: BigInt::from(1u8)
                    }
                );
                assert_eq!(
                    found,
                    RelocatableValue {
                        segment_index: BigInt::from(2u8),
                        offset: BigInt::from(3u8)
                    }
                );
            }
            _ => panic!("unexpected result"),
        }
    }
}
