use crate::cairo::lang::{
    compiler::program::{FullProgram, Program},
    vm::{builtin_runner::BuiltinRunner, relocatable::RelocatableValue, vm_core::RunContext},
};

use num_bigint::BigInt;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug)]
pub struct CompiledHint {}

#[derive(Debug)]
pub struct VirtualMachineBase {
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
}

impl VirtualMachineBase {
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
    /// program_base - The pc of the first instruction in program.
    #[allow(unused)]
    pub fn new(
        program: Rc<Program>,
        run_context: Rc<RefCell<RunContext>>,
        hint_locals: HashMap<String, ()>,
        static_locals: Option<HashMap<String, ()>>,
        builtin_runners: Rc<HashMap<String, BuiltinRunner>>,
        program_base: RelocatableValue,
    ) -> Self {
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
}
