use crate::cairo::lang::{
    compiler::program::Program,
    vm::{
        builtin_runner::BuiltinRunner, memory_dict::MemoryDict, relocatable::RelocatableValue,
        trace_entry::TraceEntry, virtual_machine_base::VirtualMachineBase,
    },
};

use num_bigint::BigInt;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

/// Contains a complete state of the virtual machine. This includes registers and memory.
#[derive(Debug, Clone)]
pub struct RunContext {
    pub memory: Rc<RefCell<MemoryDict>>,
    pub pc: RelocatableValue,
    pub ap: RelocatableValue,
    pub fp: RelocatableValue,
    pub prime: BigInt,
}

#[derive(Debug)]
pub struct VirtualMachine {
    // Simulate inheritance
    pub base: VirtualMachineBase,
    pub run_context: Rc<RefCell<RunContext>>,
    /// A set to track the memory addresses accessed by actual Cairo instructions (as opposed to
    /// hints), necessary for accurate counting of memory holes.
    pub accessed_addresses: HashSet<RelocatableValue>,
    pub trace: Vec<TraceEntry>,
    /// Current step.
    pub current_step: BigInt,
    /// This flag can be set to true by hints to avoid the execution of the current step in step()
    /// (so that only the hint will be performed, but nothing else will happen).
    pub skip_instruction_execution: bool,
}

impl RunContext {
    pub fn new(
        memory: Rc<RefCell<MemoryDict>>,
        pc: RelocatableValue,
        ap: RelocatableValue,
        fp: RelocatableValue,
        prime: BigInt,
    ) -> Self {
        Self {
            memory,
            pc,
            ap,
            fp,
            prime,
        }
    }
}

impl VirtualMachine {
    /// See documentation in VirtualMachineBase.
    ///
    /// program_base - The pc of the first instruction in program (default is run_context.pc).
    pub fn new(
        program: Rc<Program>,
        run_context: Rc<RefCell<RunContext>>,
        hint_locals: HashMap<String, ()>,
        static_locals: Option<HashMap<String, ()>>,
        builtin_runners: Option<Rc<HashMap<String, BuiltinRunner>>>,
        program_base: Option<RelocatableValue>,
    ) -> Self {
        let program_base = program_base.unwrap_or_else(|| run_context.borrow().pc.clone());
        let builtin_runners = builtin_runners.unwrap_or_else(|| Rc::new(HashMap::new()));

        // Simulate `super().__init__()` due to lack of inheritance
        let base = VirtualMachineBase::new(
            program.clone(),
            run_context.clone(),
            hint_locals,
            static_locals,
            builtin_runners,
            program_base.clone(),
        );

        // A set to track the memory addresses accessed by actual Cairo instructions (as opposed to
        // hints), necessary for accurate counting of memory holes.
        let mut accessed_addresses = HashSet::new();
        for i in 0..program.data().len() {
            accessed_addresses.insert(program_base.clone() + &BigInt::from(i));
        }

        Self {
            base,
            run_context,
            accessed_addresses,
            trace: vec![],
            current_step: BigInt::from(0),
            skip_instruction_execution: false,
        }
    }
}
