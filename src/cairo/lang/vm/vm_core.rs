use crate::cairo::lang::{
    compiler::{
        encode::decode_instruction,
        instruction::Instruction,
        program::{FullProgram, Program},
    },
    vm::{
        builtin_runner::BuiltinRunner,
        memory_dict::MemoryDict,
        relocatable::{MaybeRelocatable, RelocatableValue},
        trace_entry::TraceEntry,
        virtual_machine_base::CompiledHint,
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
    // //////////
    // START: Fields from `VirtualMachineBase` in Python
    // //////////
    pub prime: BigInt,
    pub builtin_runners: Rc<HashMap<String, BuiltinRunner>>,
    pub exec_scopes: Vec<HashMap<String, ()>>,
    pub hints: HashMap<RelocatableValue, Vec<CompiledHint>>,
    /// A map from hint id to pc and index (index is required when there is more than one hint for a
    /// single pc).
    pub hint_pc_and_index: HashMap<BigInt, (RelocatableValue, BigInt)>,
    pub instruction_debug_info: (),
    pub debug_file_contents: (),
    pub error_message_attributes: (),
    pub program: Rc<Program>,
    pub validated_memory: (),
    /// auto_deduction contains a mapping from a memory segment index to a list of functions (and a
    /// tuple of additional arguments) that may try to automatically deduce the value of memory
    /// cells in the segment (based on other memory cells).
    pub auto_deduction: (),
    /// This flag can be set to true by hints to avoid the execution of the current step in step()
    /// (so that only the hint will be performed, but nothing else will happen).
    pub skip_instruction_execution: bool,
    // //////////
    // END: Fields from `VirtualMachineBase` in Python
    // //////////
    pub run_context: Rc<RefCell<RunContext>>,
    /// A set to track the memory addresses accessed by actual Cairo instructions (as opposed to
    /// hints), necessary for accurate counting of memory holes.
    pub accessed_addresses: HashSet<RelocatableValue>,
    pub trace: Vec<TraceEntry>,
    /// Current step.
    pub current_step: BigInt,
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

    /// Returns the encoded instruction (the value at pc) and the immediate value (the value at pc +
    /// 1, if it exists in the memory).
    pub fn get_instruction_encoding(&self) -> (BigInt, Option<BigInt>) {
        let memory = self.memory.borrow();

        // TODO: check if it's safe to call unwrap here
        let instruction_encoding = memory.get(&self.pc).unwrap();
        let instruction_encoding = match instruction_encoding {
            MaybeRelocatable::Int(int) => int,
            // TODO: switch to proper error handling
            MaybeRelocatable::RelocatableValue(_) => panic!("Instruction should be an int"),
        };

        let imm_addr = (self.pc.clone() + &BigInt::from(1)) % &self.prime;
        let optional_imm = memory.get(&imm_addr);
        let optional_imm = match optional_imm {
            Some(imm) => match imm {
                MaybeRelocatable::Int(int) => Some(int),
                MaybeRelocatable::RelocatableValue(_) => None,
            },
            None => None,
        };

        (instruction_encoding.to_owned(), optional_imm.cloned())
    }
}

impl VirtualMachine {
    /// hints - a dictionary from memory addresses to an executable object.
    ///   When the pc points to the memory address, before the execution of the instruction, the
    ///   executable object will be run. Executable objects are anything that can be placed inside
    ///   exec. For example, 'a=5', or compile('a=5').
    ///
    /// hint_locals - dictionary holding local values for execution of hints.
    ///   Passed as locals parameter for the exec function.
    ///
    /// static_locals - dictionary holding static values for execution. They are available in all
    ///   scopes.
    ///
    /// program_base - The pc of the first instruction in program (default is run_context.pc).
    #[allow(unused)]
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

        // A set to track the memory addresses accessed by actual Cairo instructions (as opposed to
        // hints), necessary for accurate counting of memory holes.
        let mut accessed_addresses = HashSet::new();
        for i in 0..program.data().len() {
            accessed_addresses.insert(program_base.clone() + &BigInt::from(i));
        }

        // //////////
        // START: `VirtualMachineBase` ctor logic
        // //////////

        let mut vm = Self {
            prime: program.prime().clone(),
            builtin_runners,
            exec_scopes: vec![],
            hints: HashMap::new(),
            hint_pc_and_index: HashMap::new(),
            instruction_debug_info: (),
            debug_file_contents: (),
            error_message_attributes: (),
            program: program.clone(),
            validated_memory: (),
            auto_deduction: (),
            skip_instruction_execution: false,
            run_context,
            accessed_addresses,
            trace: vec![],
            current_step: BigInt::from(0),
        };

        vm.enter_scope(Some(hint_locals));

        // If program is a StrippedProgram, there are no hints or debug information to load.
        if let Program::Full(program) = program.as_ref() {
            vm.load_program(program, program_base);
        }

        // TODO: implement the following Python code
        //
        // ```python
        // self.static_locals = static_locals.copy() if static_locals is not None else {}
        // self.static_locals.update(
        //     {
        //         "PRIME": self.prime,
        //         "fadd": lambda a, b, p=self.prime: (a + b) % p,
        //         "fsub": lambda a, b, p=self.prime: (a - b) % p,
        //         "fmul": lambda a, b, p=self.prime: (a * b) % p,
        //         "fdiv": lambda a, b, p=self.prime: math_utils.div_mod(a, b, p),
        //         "fpow": lambda a, b, p=self.prime: pow(a, b, p),
        //         "fis_quad_residue": lambda a, p=self.prime: math_utils.is_quad_residue(a, p),
        //         "fsqrt": lambda a, p=self.prime: math_utils.sqrt(a, p),
        //         "safe_div": math_utils.safe_div,
        //     }
        // )
        // ```

        // //////////
        // END: `VirtualMachineBase` ctor logic
        // //////////

        vm
    }

    /// Starts a new scope of user-defined local variables available to hints.
    ///
    /// Note that variables defined in outer scopes will not be available in the new scope. A
    /// dictionary of locals that should be available in the new scope should be passed in
    /// new_scope_locals.
    ///
    /// The scope starts only from the next hint.
    ///
    /// exit_scope() must be called to resume the previous scope.
    pub fn enter_scope(&mut self, new_scope_locals: Option<HashMap<String, ()>>) {
        let mut new_scope = HashMap::new();

        if let Some(new_scope_locals) = new_scope_locals {
            for (key, _) in new_scope_locals.iter() {
                new_scope.insert(key.to_owned(), ());
            }
        }

        // TODO: add builtin_runners to hint scope

        self.exec_scopes.push(new_scope);
    }

    pub fn step(&mut self) {
        self.skip_instruction_execution = false;

        // Hints not yet implemented
        // TODO: implement the following Python code
        //
        // ```python
        // # Execute hints.
        // for hint_index, hint in enumerate(self.hints.get(self.run_context.pc, [])):
        //     exec_locals = self.exec_scopes[-1]
        //     exec_locals["memory"] = memory = self.validated_memory
        //     exec_locals["ap"] = ap = self.run_context.ap
        //     exec_locals["fp"] = fp = self.run_context.fp
        //     exec_locals["pc"] = pc = self.run_context.pc
        //     exec_locals["current_step"] = self.current_step
        //     exec_locals["ids"] = hint.consts(pc, ap, fp, memory)
        //
        //     exec_locals["vm_load_program"] = self.load_program
        //     exec_locals["vm_enter_scope"] = self.enter_scope
        //     exec_locals["vm_exit_scope"] = self.exit_scope
        //     exec_locals.update(self.static_locals)
        //
        //     self.exec_hint(hint.compiled, exec_locals, hint_index=hint_index)
        //
        //     # Clear ids (which will be rewritten by the next hint anyway) to make the VM instance
        //     # smaller and faster to copy.
        //     del exec_locals["ids"]
        //     del exec_locals["memory"]
        //
        //     if self.skip_instruction_execution:
        //         return
        // ```

        // Decode.
        let instruction = self.decode_current_instruction();

        // Run.
        self.run_instruction(&instruction);
    }

    #[allow(unused)]
    pub fn load_program(&mut self, program: &FullProgram, program_base: RelocatableValue) {
        // TODO: change to use `Result` for graceful error handling
        if self.prime != program.prime {
            panic!(
                "Unexpected prime for loaded program: {} != {}.",
                program.prime, self.prime
            );
        }

        // TODO: implement the following Python code
        //
        // ```python
        // self.load_debug_info(program.debug_info, program_base)
        // self.load_hints(program, program_base)
        // self.error_message_attributes.extend(
        //     VmAttributeScope.from_attribute_scope(attr=attr, program_base=program_base)
        //     for attr in program.attributes
        //     if attr.name == ERROR_MESSAGE_ATTRIBUTE
        // )
        // ```
    }

    #[allow(clippy::let_and_return)] // Doing this on purpose to mimic Python code
    pub fn decode_current_instruction(&self) -> Instruction {
        let (instruction_encoding, imm) = self.run_context.borrow().get_instruction_encoding();

        let instruction = decode_instruction(instruction_encoding, imm);

        instruction
    }

    #[allow(unused)]
    pub fn run_instruction(&self, instruction: &Instruction) {
        todo!()
    }
}
