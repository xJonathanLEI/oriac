use crate::{
    cairo::lang::{
        compiler::{
            encode::decode_instruction,
            instruction::{
                ApUpdate, FpUpdate, Instruction, Op1Addr, Opcode, PcUpdate, Register, Res,
            },
            program::{FullProgram, Program},
        },
        vm::{
            cairo_runner::BuiltinRunnerMap,
            memory_dict::{Error as MemoryDictError, MemoryDict},
            relocatable::{MaybeRelocatable, RelocatableValue},
            trace_entry::TraceEntry,
            validated_memory_dict::ValidatedMemoryDict,
            virtual_machine_base::CompiledHint,
            vm_exceptions::PureValueError,
        },
    },
    hint_support::StaticLocals,
};

use num_bigint::BigInt;
use once_cell::unsync::OnceCell;
use rustpython_vm::{Interpreter, PyObjectRef, PyPayload};
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Debug,
    rc::Rc,
};

pub struct Rule {
    pub inner: fn(&VirtualMachine, &RelocatableValue, &()) -> Option<BigInt>,
}

/// Values of the operands.
#[derive(Debug)]
pub struct Operands {
    pub dst: MaybeRelocatable,
    pub res: Option<MaybeRelocatable>,
    pub op0: MaybeRelocatable,
    pub op1: MaybeRelocatable,
}

/// Contains a complete state of the virtual machine. This includes registers and memory.
#[derive(Debug, Clone)]
pub struct RunContext {
    pub memory: Rc<RefCell<MemoryDict>>,
    pub pc: MaybeRelocatable,
    pub ap: MaybeRelocatable,
    pub fp: MaybeRelocatable,
    pub prime: BigInt,
}

#[derive(Debug, thiserror::Error)]
pub enum RunContextError {
    #[error("In immediate mode, off2 should be 1.")]
    InvalidOff2Value,
    #[error("op0 must be known in double dereference.")]
    UnknownOp0,
}

pub struct VirtualMachine {
    // //////////
    // START: Fields from `VirtualMachineBase` in Python
    // //////////
    pub prime: BigInt,
    pub builtin_runners: Rc<RefCell<BuiltinRunnerMap>>,
    pub exec_scopes: Vec<HashMap<String, ()>>,
    pub hints: HashMap<MaybeRelocatable, Vec<CompiledHint>>,
    /// A map from hint id to pc and index (index is required when there is more than one hint for a
    /// single pc).
    pub hint_pc_and_index: HashMap<BigInt, (MaybeRelocatable, BigInt)>,
    pub instruction_debug_info: (),
    pub debug_file_contents: (),
    pub error_message_attributes: (),
    pub program: Rc<Program>,
    pub validated_memory: ValidatedMemoryDict,
    /// auto_deduction contains a mapping from a memory segment index to a list of functions (and a
    /// tuple of additional arguments) that may try to automatically deduce the value of memory
    /// cells in the segment (based on other memory cells).
    pub auto_deduction: HashMap<BigInt, Vec<(Rule, ())>>,
    pub static_locals: StaticLocals,
    /// This flag can be set to true by hints to avoid the execution of the current step in step()
    /// (so that only the hint will be performed, but nothing else will happen).
    pub skip_instruction_execution: bool,
    // //////////
    // END: Fields from `VirtualMachineBase` in Python
    // //////////
    pub run_context: Rc<RefCell<RunContext>>,
    /// A set to track the memory addresses accessed by actual Cairo instructions (as opposed to
    /// hints), necessary for accurate counting of memory holes.
    pub accessed_addresses: HashSet<MaybeRelocatable>,
    pub trace: Vec<TraceEntry<MaybeRelocatable>>,
    /// Current step.
    pub current_step: BigInt,
    pub python_interpreter: OnceCell<Interpreter>,
}

#[derive(Debug, thiserror::Error)]
pub enum VirtualMachineError {
    #[error(transparent)]
    RunContextError(RunContextError),
    #[error(transparent)]
    MemoryDictError(MemoryDictError),
    #[error(transparent)]
    PureValueError(PureValueError),
    #[error("Res.UNCONSTRAINED cannot be used with Opcode.ASSERT_EQ")]
    AssertEqWithUnconstrained,
    #[error("An ASSERT_EQ instruction failed: {dst} != {res}.")]
    AssertEqFailed {
        dst: MaybeRelocatable,
        res: MaybeRelocatable,
    },
    #[error("Call failed to write return-pc (inconsistent op0): {op0} != {return_pc}. Did you forget to increment ap?")]
    InconsistentOp0 {
        op0: MaybeRelocatable,
        return_pc: MaybeRelocatable,
    },
    #[error("Call failed to write return-fp (inconsistent dst): {dst} != {return_fp}. Did you forget to increment ap?")]
    InconsistentDst {
        dst: MaybeRelocatable,
        return_fp: MaybeRelocatable,
    },
    #[error("Res.UNCONSTRAINED cannot be used with ApUpdate.ADD")]
    AddWithUnconstrained,
    #[error("Res.UNCONSTRAINED cannot be used with PcUpdate.JUMP")]
    JumpWithUnconstrained,
    #[error("Res.UNCONSTRAINED cannot be used with PcUpdate.JUMP_REL")]
    JumpRelWithUnconstrained,
    #[error("Every enter_scope() requires a corresponding exit_scope().")]
    EnterExitScopeMismatch,
    #[error("Inconsistent auto deduction rule at address {addr}. {current_value} != {new_value}.")]
    InconsistentAutoDeduction {
        addr: RelocatableValue,
        current_value: MaybeRelocatable,
        new_value: MaybeRelocatable,
    },
    #[error("Call failed to write return-pc (inconsistent op0): {op0} != {return_pc}. Did you forget to increment ap?")]
    FailedToWriteReturnPc {
        op0: MaybeRelocatable,
        return_pc: MaybeRelocatable,
    },
    #[error("Call failed to write return-fp (inconsistent dst): {dst} != {return_fp}. Did you forget to increment ap?")]
    FailedToWriteReturnFp {
        dst: MaybeRelocatable,
        return_fp: MaybeRelocatable,
    },
    #[error(transparent)]
    HintCompileError(rustpython_vm::compile::CompileError),
    #[error("Got an exception while executing a hint ({hint_index}): {exception}")]
    HintExecuteError {
        hint_index: usize,
        exception: String,
    },
}

impl Debug for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "|Closure|")
    }
}

impl RunContext {
    pub fn new(
        memory: Rc<RefCell<MemoryDict>>,
        pc: MaybeRelocatable,
        ap: MaybeRelocatable,
        fp: MaybeRelocatable,
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
    pub fn get_instruction_encoding(&mut self) -> (BigInt, Option<BigInt>) {
        let mut memory = self.memory.as_ref().borrow_mut();

        // TODO: check if it's safe to call unwrap here (probably not, change to proper error
        //       handling)
        let instruction_encoding = memory.index(&self.pc).unwrap();
        let instruction_encoding = match instruction_encoding {
            MaybeRelocatable::Int(int) => int,
            // TODO: switch to proper error handling
            MaybeRelocatable::RelocatableValue(_) => panic!("Instruction should be an int"),
        };

        let imm_addr = (self.pc.clone() + &BigInt::from(1)) % &self.prime;
        let optional_imm = memory.get(&imm_addr, None);
        let optional_imm = match optional_imm {
            Some(imm) => match imm {
                MaybeRelocatable::Int(int) => Some(int),
                MaybeRelocatable::RelocatableValue(_) => None,
            },
            None => None,
        };

        (instruction_encoding, optional_imm)
    }

    pub fn compute_dst_addr(&self, instruction: &Instruction) -> MaybeRelocatable {
        let base_addr = match instruction.dst_register {
            Register::AP => self.ap.clone(),
            Register::FP => self.fp.clone(),
        };
        (base_addr + &BigInt::from(instruction.off0)) % &self.prime
    }

    pub fn compute_op0_addr(&self, instruction: &Instruction) -> MaybeRelocatable {
        let base_addr = match instruction.op0_register {
            Register::AP => self.ap.clone(),
            Register::FP => self.fp.clone(),
        };
        (base_addr + &BigInt::from(instruction.off1)) % &self.prime
    }

    pub fn compute_op1_addr(
        &self,
        instruction: &Instruction,
        op0: Option<MaybeRelocatable>,
    ) -> Result<MaybeRelocatable, RunContextError> {
        let base_addr = match instruction.op1_addr {
            Op1Addr::FP => self.fp.clone(),
            Op1Addr::AP => self.ap.clone(),
            Op1Addr::IMM => {
                if instruction.off2 != 1 {
                    return Err(RunContextError::InvalidOff2Value);
                }
                self.pc.clone()
            }
            Op1Addr::OP0 => match op0 {
                Some(op0) => op0,
                None => {
                    return Err(RunContextError::UnknownOp0);
                }
            },
        };
        Ok((base_addr + &BigInt::from(instruction.off2)) % &self.prime)
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
        static_locals: StaticLocals,
        builtin_runners: Option<Rc<RefCell<BuiltinRunnerMap>>>,
        program_base: Option<MaybeRelocatable>,
    ) -> Self {
        let program_base = program_base.unwrap_or_else(|| run_context.borrow().pc.clone());
        let builtin_runners =
            builtin_runners.unwrap_or_else(|| Rc::new(RefCell::new(HashMap::new())));

        // A set to track the memory addresses accessed by actual Cairo instructions (as opposed to
        // hints), necessary for accurate counting of memory holes.
        let mut accessed_addresses = HashSet::new();
        for i in 0..program.data().len() {
            accessed_addresses.insert(program_base.clone() + &BigInt::from(i));
        }

        // //////////
        // START: `VirtualMachineBase` ctor logic
        // //////////

        let validated_memory = ValidatedMemoryDict::new(run_context.borrow().memory.clone());

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
            validated_memory,
            auto_deduction: HashMap::new(),
            static_locals,
            skip_instruction_execution: false,
            run_context,
            accessed_addresses,
            trace: vec![],
            current_step: BigInt::from(0),
            python_interpreter: OnceCell::new(),
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

    pub fn step(&mut self) -> Result<(), VirtualMachineError> {
        self.skip_instruction_execution = false;

        // Execute hints.
        if let Some(hints) = self.hints.get(&self.run_context.borrow().pc) {
            for (hint_index, hint) in hints.iter().enumerate() {
                // TODO: implement the following Python code
                //
                // ```python
                // exec_locals = self.exec_scopes[-1]
                // exec_locals["memory"] = memory = self.validated_memory
                // exec_locals["ap"] = ap = self.run_context.ap
                // exec_locals["fp"] = fp = self.run_context.fp
                // exec_locals["pc"] = pc = self.run_context.pc
                // exec_locals["current_step"] = self.current_step
                // exec_locals["ids"] = hint.consts(pc, ap, fp, memory)
                //
                // exec_locals["vm_load_program"] = self.load_program
                // exec_locals["vm_enter_scope"] = self.enter_scope
                // exec_locals["vm_exit_scope"] = self.exit_scope
                // exec_locals.update(self.static_locals)
                // ```

                // This will almost always fail as globals injection has not been fully implemented
                self.python_interpreter
                    .get_or_init(Interpreter::default)
                    .enter(|vm| {
                        let scope = vm.new_scope_with_builtins();

                        // Injects hint context variables
                        {
                            let ctx_segments = self.static_locals.segments.clone();

                            let memory_segment_manager_cls = vm.ctx.new_class(
                                None,
                                "MemorySegmentManager",
                                &vm.ctx.types.object_type,
                                Default::default(),
                            );
                            memory_segment_manager_cls.set_str_attr(
                                "add",
                                vm.ctx.new_method(
                                    "add",
                                    memory_segment_manager_cls.clone(),
                                    move |_self: PyObjectRef| {
                                        ctx_segments.as_ref().borrow_mut().add(None);
                                    },
                                ),
                            );

                            let segments_obj =
                                vm.ctx.new_base_object(memory_segment_manager_cls, None);
                            scope
                                .globals
                                .set_item("segments", segments_obj, vm)
                                .unwrap();
                        }

                        match vm.run_code_obj(
                            rustpython_vm::builtins::PyCode::new(
                                vm.map_codeobj(hint.compiled.clone()),
                            )
                            .into_ref(vm),
                            scope,
                        ) {
                            Ok(value) => Ok(value),
                            Err(err) => {
                                // unwrap() here should be safe
                                let mut err_str = String::new();
                                vm.write_exception(&mut err_str, &err).unwrap();

                                Err(VirtualMachineError::HintExecuteError {
                                    hint_index,
                                    exception: err_str,
                                })
                            }
                        }
                    })?;

                // TODO: implement the following Python code
                //
                // ```python
                // # Clear ids (which will be rewritten by the next hint anyway) to make the VM instance
                // # smaller and faster to copy.
                // del exec_locals["ids"]
                // del exec_locals["memory"]
                // ```

                if self.skip_instruction_execution {
                    return Ok(());
                }
            }
        }

        // Decode.
        let instruction = self.decode_current_instruction();

        // Run.
        self.run_instruction(&instruction)
    }

    pub fn load_hints(
        &mut self,
        program: &FullProgram,
        program_base: MaybeRelocatable,
    ) -> Result<(), VirtualMachineError> {
        // TODO: change to only compile the hint when no Rust port is available

        for (pc, hints) in program.hints.iter() {
            let mut compiled_hints = vec![];
            for (hint_index, hint) in hints.iter().enumerate() {
                let hint_id = self.hint_pc_and_index.len();
                self.hint_pc_and_index.insert(
                    hint_id.into(),
                    (
                        MaybeRelocatable::Int(pc.to_owned()) + &program_base,
                        hint_index.into(),
                    ),
                );
                compiled_hints.push(CompiledHint {
                    compiled: rustpython_vm::compile::compile(
                        &hint.code,
                        rustpython_vm::compile::Mode::Exec,
                        format!("<hint{}>", hint_id),
                        rustpython_vm::compile::CompileOpts::default(),
                    )?,
                    consts: (),
                });

                // TODO: implement the following Python code
                //
                // ```python
                // # Use hint=hint in the lambda's arguments to capture this value (otherwise,
                // # it will use the same hint object for all iterations).
                // consts=lambda pc, ap, fp, memory, hint=hint: VmConsts(
                //     context=VmConstsContext(
                //         identifiers=program.identifiers,
                //         evaluator=ExpressionEvaluator(
                //             self.prime, ap, fp, memory, program.identifiers
                //         ).eval,
                //         reference_manager=program.reference_manager,
                //         flow_tracking_data=hint.flow_tracking_data,
                //         memory=memory,
                //         pc=pc,
                //     ),
                //     accessible_scopes=hint.accessible_scopes,
                // ),
                // ```
            }
            self.hints.insert(
                MaybeRelocatable::Int(pc.to_owned()) + &program_base,
                compiled_hints,
            );
        }

        Ok(())
    }

    pub fn load_program(
        &mut self,
        program: &FullProgram,
        program_base: MaybeRelocatable,
    ) -> Result<(), VirtualMachineError> {
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
        // ```

        self.load_hints(program, program_base)?;

        // TODO: implement the following Python code
        //
        // ```python
        // self.error_message_attributes.extend(
        //     VmAttributeScope.from_attribute_scope(attr=attr, program_base=program_base)
        //     for attr in program.attributes
        //     if attr.name == ERROR_MESSAGE_ATTRIBUTE
        // )
        // ```

        Ok(())
    }

    pub fn update_registers(
        &mut self,
        instruction: &Instruction,
        operands: &Operands,
    ) -> Result<(), VirtualMachineError> {
        // Update fp.
        let new_fp_value = match instruction.fp_update {
            FpUpdate::AP_PLUS2 => Some(self.run_context.borrow().ap.clone() + &BigInt::from(2u32)),
            FpUpdate::DST => Some(operands.dst.clone()),
            FpUpdate::REGULAR => None,
        };
        if let Some(new_fp_value) = new_fp_value {
            self.run_context.as_ref().borrow_mut().fp = new_fp_value;
        }

        // Update ap.
        let new_ap_value = match instruction.ap_update {
            ApUpdate::ADD => match &operands.res {
                Some(res) => {
                    Some(self.run_context.borrow().ap.clone() + &(res.to_owned() % &self.prime))
                }
                None => return Err(VirtualMachineError::AddWithUnconstrained),
            },
            ApUpdate::ADD1 => Some(self.run_context.borrow().ap.clone() + &BigInt::from(1)),
            ApUpdate::ADD2 => Some(self.run_context.borrow().ap.clone() + &BigInt::from(2)),
            ApUpdate::REGULAR => None,
        };
        let new_ap_value = match new_ap_value {
            Some(new_ap_value) => new_ap_value % &self.prime,
            None => self.run_context.borrow().ap.clone() % &self.prime,
        };
        self.run_context.as_ref().borrow_mut().ap = new_ap_value;

        // Update pc.
        // The pc update should be done last so that we will have the correct pc in case of an
        // exception during one of the updates above.
        let new_pc_value = match instruction.pc_update {
            PcUpdate::REGULAR => {
                Some(self.run_context.borrow().pc.clone() + &BigInt::from(instruction.size()))
            }
            PcUpdate::JUMP => match &operands.res {
                Some(res) => Some(res.to_owned()),
                None => return Err(VirtualMachineError::JumpWithUnconstrained),
            },
            PcUpdate::JUMP_REL => match &operands.res {
                Some(res) => match res {
                    MaybeRelocatable::Int(res) => Some(self.run_context.borrow().pc.clone() + res),
                    &MaybeRelocatable::RelocatableValue(_) => {
                        return Err(VirtualMachineError::PureValueError(PureValueError {}))
                    }
                },
                None => return Err(VirtualMachineError::JumpRelWithUnconstrained),
            },
            PcUpdate::JNZ => {
                if is_zero(&operands.dst)? {
                    Some(self.run_context.borrow().pc.clone() + &BigInt::from(instruction.size()))
                } else {
                    Some(self.run_context.borrow().pc.clone() + &operands.op1)
                }
            }
        };
        let new_pc_value = match new_pc_value {
            Some(new_pc_value) => new_pc_value % &self.prime,
            None => self.run_context.borrow().pc.clone() % &self.prime,
        };
        self.run_context.as_ref().borrow_mut().pc = new_pc_value;

        Ok(())
    }

    /// Returns a tuple (deduced_op0, deduced_res).
    /// Deduces the value of op0 if possible (based on dst and op1). Otherwise, returns None.
    /// If res was already deduced, returns its deduced value as well.
    #[allow(unused)]
    pub fn deduce_op0(
        &self,
        instruction: &Instruction,
        dst: Option<MaybeRelocatable>,
        op1: Option<MaybeRelocatable>,
    ) -> (Option<MaybeRelocatable>, Option<MaybeRelocatable>) {
        match instruction.opcode {
            Opcode::CALL => (
                Some(self.run_context.borrow().pc.clone() + &BigInt::from(instruction.size())),
                None,
            ),
            Opcode::ASSERT_EQ => {
                if let (Res::ADD, Some(dst), Some(op1)) =
                    (&instruction.res, dst.clone(), op1.clone())
                {
                    (Some((dst.clone() - &op1) % &self.prime), Some(dst))
                } else if let (
                    Res::MUL,
                    Some(MaybeRelocatable::Int(dst)),
                    Some(MaybeRelocatable::Int(op1)),
                ) = (&instruction.res, dst, op1)
                {
                    if op1 != BigInt::from(0u32) {
                        // TODO: implement the following Python code
                        //
                        // ```python
                        // return div_mod(dst, op1, self.prime), dst
                        // ```
                        todo!()
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            }
            _ => (None, None),
        }
    }

    /// Returns a tuple (deduced_op1, deduced_res).
    /// Deduces the value of op1 if possible (based on dst and op0). Otherwise, returns None.
    /// If res was already deduced, returns its deduced value as well.
    pub fn deduce_op1(
        &self,
        instruction: &Instruction,
        dst: Option<MaybeRelocatable>,
        op0: Option<MaybeRelocatable>,
    ) -> (Option<MaybeRelocatable>, Option<MaybeRelocatable>) {
        match instruction.opcode {
            Opcode::ASSERT_EQ => {
                if let (Res::OP1, Some(dst)) = (&instruction.res, dst.clone()) {
                    (Some(dst.clone()), Some(dst))
                } else if let (Res::ADD, Some(dst), Some(op0)) =
                    (&instruction.res, dst.clone(), op0.clone())
                {
                    (Some((dst.clone() - &op0) % &self.prime), Some(dst))
                } else if let (
                    Res::MUL,
                    Some(MaybeRelocatable::Int(_)),
                    Some(MaybeRelocatable::Int(op0)),
                ) = (&instruction.res, &dst, op0)
                {
                    if op0 != BigInt::from(0u32) {
                        // TODO: implement the following Python code
                        //
                        // ```python
                        // return div_mod(dst, op0, self.prime), dst
                        // ```
                        todo!()
                    } else {
                        (None, None)
                    }
                } else {
                    todo!()
                }
            }
            _ => (None, None),
        }
    }

    /// Computes the value of res if possible.
    pub fn compute_res(
        &self,
        instruction: &Instruction,
        op0: MaybeRelocatable,
        op1: MaybeRelocatable,
    ) -> Result<Option<MaybeRelocatable>, VirtualMachineError> {
        Ok(match instruction.res {
            Res::OP1 => Some(op1),
            Res::ADD => Some((op0 + &op1) % &self.prime),
            Res::MUL => {
                if let (MaybeRelocatable::Int(op0), MaybeRelocatable::Int(op1)) = (op0, op1) {
                    Some(((op0 * op1) % &self.prime).into())
                } else {
                    return Err(VirtualMachineError::PureValueError(PureValueError {}));
                }
            }
            Res::UNCONSTRAINED => {
                // In this case res should be the inverse of dst.
                // For efficiency, we do not compute it here.
                None
            }
        })
    }

    /// Computes the values of the operands. Deduces dst if needed.
    ///
    /// Returns:
    /// - operands - an Operands instance with the values of the operands.
    /// - mem_addresses - the memory addresses for the 3 memory units used (dst, op0, op1).
    ///
    /// NOTE: the type of `mem_addresses` elements has been changed from `int` in Python to
    /// `MaybeRelocatable`, as it seems to be a mistake.
    pub fn compute_operands(
        &mut self,
        instruction: &Instruction,
    ) -> Result<(Operands, Vec<MaybeRelocatable>), VirtualMachineError> {
        // Try to fetch dst, op0, op1.
        // op0 throughout this function represents the value at op0_addr.
        // If op0 is set, this implies that we are going to set memory at op0_addr to that value.
        // Same for op1, dst.
        let dst_addr = self.run_context.borrow().compute_dst_addr(instruction);
        let mut dst = self.validated_memory.get(&dst_addr, None);
        let op0_addr = self.run_context.borrow().compute_op0_addr(instruction);
        let mut op0 = self.validated_memory.get(&op0_addr, None);
        let op1_addr = self
            .run_context
            .borrow()
            .compute_op1_addr(instruction, op0.clone())?;
        let mut op1 = self.validated_memory.get(&op1_addr, None);

        // res throughout this function represents the computation on op0,op1
        // as defined in decode.py.
        // If it is set, this implies that compute_res(...) will return this value.
        // If it is set without invoking compute_res(), this is an optimization, but should not
        // yield a different result.
        // In particular, res may be different than dst, even in ASSERT_EQ. In this case,
        // The ASSERT_EQ validation will fail in opcode_assertions().
        let mut res: Option<MaybeRelocatable> = None;

        // Auto deduction rules.
        // Note: This may fail to deduce if 2 auto deduction rules are needed to be used in
        // a different order.
        if matches!(op0, None) {
            op0 = self.deduce_memory_cell(&op0_addr);
        }
        if matches!(op1, None) {
            op1 = self.deduce_memory_cell(&op1_addr);
        }

        let should_update_dst = dst.is_none();
        let should_update_op0 = op0.is_none();
        let should_update_op1 = op1.is_none();

        // Deduce op0 if needed.
        if op0.is_none() {
            let temp = self.deduce_op0(instruction, dst.clone(), op1.clone());
            op0 = temp.0;
            let deduced_res = temp.1;
            if res.is_none() {
                res = deduced_res;
            }
        }

        // Deduce op1 if needed.
        if op1.is_none() {
            let temp = self.deduce_op1(instruction, dst.clone(), op0.clone());
            op1 = temp.0;
            let deduced_res = temp.1;
            if res.is_none() {
                res = deduced_res;
            }
        }

        // Force pulling op0, op1 from memory for soundness test
        // and to get an informative error message if they were not computed.
        let op0 = match op0 {
            Some(op0) => op0,
            None => self.validated_memory.borrow_mut().index(&op0_addr)?,
        };
        let op1 = match op1 {
            Some(op1) => op1,
            None => self.validated_memory.borrow_mut().index(&op0_addr)?,
        };

        // Compute res if needed.
        if res.is_none() {
            res = self.compute_res(instruction, op0.clone(), op1.clone())?;
        }

        // Deduce dst.
        if dst.is_none() {
            if let (Opcode::ASSERT_EQ, Some(res)) = (&instruction.opcode, &res) {
                dst = Some(res.to_owned());
            } else if matches!(instruction.opcode, Opcode::CALL) {
                dst = Some(self.run_context.borrow().fp.clone());
            }
        }

        // Force pulling dst from memory for soundness.
        let dst = match dst {
            Some(dst) => dst,
            None => self.validated_memory.borrow_mut().index(&dst_addr)?,
        };

        // Write updated values.
        if should_update_dst {
            self.validated_memory
                .index_set(dst_addr.clone(), dst.clone());
        }
        if should_update_op0 {
            self.validated_memory
                .index_set(op0_addr.clone(), op0.clone());
        }
        if should_update_op1 {
            self.validated_memory
                .index_set(op1_addr.clone(), op1.clone());
        }

        Ok((
            Operands { dst, op0, op1, res },
            vec![dst_addr, op0_addr, op1_addr],
        ))
    }

    #[allow(clippy::let_and_return)] // Doing this on purpose to mimic Python code
    pub fn decode_current_instruction(&self) -> Instruction {
        let (instruction_encoding, imm) = self
            .run_context
            .as_ref()
            .borrow_mut()
            .get_instruction_encoding();

        let instruction = decode_instruction(instruction_encoding, imm);

        instruction
    }

    pub fn opcode_assertions(
        &self,
        instruction: &Instruction,
        operands: &Operands,
    ) -> Result<(), VirtualMachineError> {
        match instruction.opcode {
            Opcode::ASSERT_EQ => match &operands.res {
                Some(res) => {
                    if &operands.dst != res && !check_eq(&operands.dst, res) {
                        Err(VirtualMachineError::AssertEqFailed {
                            dst: operands.dst.clone(),
                            res: res.to_owned(),
                        })
                    } else {
                        Ok(())
                    }
                }
                None => Err(VirtualMachineError::AssertEqWithUnconstrained),
            },
            Opcode::CALL => {
                let return_pc =
                    self.run_context.borrow().pc.clone() + &BigInt::from(instruction.size());
                if operands.op0 != return_pc && !check_eq(&operands.op0, &return_pc) {
                    return Err(VirtualMachineError::FailedToWriteReturnPc {
                        op0: operands.op0.clone(),
                        return_pc,
                    });
                }
                let return_fp = self.run_context.borrow().fp.clone();
                if operands.dst != return_fp && !check_eq(&operands.dst, &return_fp) {
                    return Err(VirtualMachineError::FailedToWriteReturnFp {
                        dst: operands.dst.clone(),
                        return_fp,
                    });
                }
                Ok(())
            }
            Opcode::RET => Ok(()),
            Opcode::NOP => Ok(()),
        }
    }

    pub fn run_instruction(
        &mut self,
        instruction: &Instruction,
    ) -> Result<(), VirtualMachineError> {
        // TODO: use `as_vm_exception` as `cairo-lang` does

        // Compute operands.
        let (operands, operands_mem_addresses) = self.compute_operands(instruction)?;

        // Opcode assertions.
        self.opcode_assertions(instruction, &operands)?;

        // Write to trace.
        self.trace.push(TraceEntry {
            pc: self.run_context.borrow().pc.clone(),
            ap: self.run_context.borrow().ap.clone(),
            fp: self.run_context.borrow().fp.clone(),
        });

        for addr in operands_mem_addresses.into_iter() {
            self.accessed_addresses.insert(addr);
        }
        self.accessed_addresses
            .insert(self.run_context.borrow().pc.clone());

        // Update registers.
        self.update_registers(instruction, &operands)?;

        self.current_step += 1;

        Ok(())
    }

    /// Tries to deduce the value of memory\[addr\] if it was not already computed.
    ///
    /// Returns the value if deduced, otherwise returns None.
    pub fn deduce_memory_cell(&mut self, addr: &MaybeRelocatable) -> Option<MaybeRelocatable> {
        match addr {
            MaybeRelocatable::Int(_) => None,
            MaybeRelocatable::RelocatableValue(addr) => {
                match self.auto_deduction.get(&addr.segment_index) {
                    Some(rules) => {
                        for (rule, args) in rules.iter() {
                            match (rule.inner)(self, addr, args) {
                                Some(value) => self
                                    .validated_memory
                                    .index_set(addr.to_owned().into(), value.into()),
                                None => continue,
                            }
                        }
                        None
                    }
                    None => None,
                }
            }
        }
    }

    /// Makes sure that all assigned memory cells are consistent with their auto deduction rules.
    #[allow(clippy::needless_collect)] // Need some refactoring to work around the issue
    pub fn verify_auto_deductions(&mut self) -> Result<(), VirtualMachineError> {
        let addrs = self
            .validated_memory
            .memory
            .as_ref()
            .borrow()
            .data
            .iter()
            .map(|(addr, _)| addr.to_owned())
            .collect::<Vec<_>>();

        for addr in addrs.into_iter() {
            match addr {
                MaybeRelocatable::Int(_) => continue,
                MaybeRelocatable::RelocatableValue(addr) => {
                    if let Some(rules) = self.auto_deduction.get(&addr.segment_index) {
                        for (rule, args) in rules.iter() {
                            match (rule.inner)(self, &addr, args) {
                                Some(value) => {
                                    let current =
                                        self.validated_memory.index(&addr.clone().into())?;

                                    // If the values are not the same, try using check_eq to
                                    // allow a subclass to override this result.
                                    if current != value
                                        && !check_eq(&current, &value.clone().into())
                                    {
                                        return Err(
                                            VirtualMachineError::InconsistentAutoDeduction {
                                                addr: addr.to_owned(),
                                                current_value: current,
                                                new_value: value.into(),
                                            },
                                        );
                                    }
                                }
                                None => continue,
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn end_run(&mut self) -> Result<(), VirtualMachineError> {
        self.verify_auto_deductions()?;
        if self.exec_scopes.len() != 1 {
            return Err(VirtualMachineError::EnterExitScopeMismatch);
        }

        Ok(())
    }
}

impl Debug for VirtualMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualMachine")
            .field("prime", &self.prime)
            .field("builtin_runners", &self.builtin_runners)
            .field("exec_scopes", &self.exec_scopes)
            .field("hints", &self.hints)
            .field("hint_pc_and_index", &self.hint_pc_and_index)
            .field("instruction_debug_info", &self.instruction_debug_info)
            .field("debug_file_contents", &self.debug_file_contents)
            .field("error_message_attributes", &self.error_message_attributes)
            .field("program", &self.program)
            .field("validated_memory", &self.validated_memory)
            .field("auto_deduction", &self.auto_deduction)
            .field(
                "skip_instruction_execution",
                &self.skip_instruction_execution,
            )
            .field("run_context", &self.run_context)
            .field("accessed_addresses", &self.accessed_addresses)
            .field("trace", &self.trace)
            .field("current_step", &self.current_step)
            .finish()
    }
}

impl From<RunContextError> for VirtualMachineError {
    fn from(value: RunContextError) -> Self {
        VirtualMachineError::RunContextError(value)
    }
}

impl From<MemoryDictError> for VirtualMachineError {
    fn from(value: MemoryDictError) -> Self {
        VirtualMachineError::MemoryDictError(value)
    }
}

impl From<PureValueError> for VirtualMachineError {
    fn from(value: PureValueError) -> Self {
        VirtualMachineError::PureValueError(value)
    }
}

impl From<rustpython_vm::compile::CompileError> for VirtualMachineError {
    fn from(value: rustpython_vm::compile::CompileError) -> Self {
        VirtualMachineError::HintCompileError(value)
    }
}

/// Returns True if value is zero (used for jnz instructions).
/// This function can be overridden by subclasses.
fn is_zero(value: &MaybeRelocatable) -> Result<bool, PureValueError> {
    match value {
        MaybeRelocatable::Int(value) => Ok(value == &BigInt::from(0u32)),
        MaybeRelocatable::RelocatableValue(value) => {
            if value.offset >= BigInt::from(0u32) {
                Ok(false)
            } else {
                Err(PureValueError {})
            }
        }
    }
}

/// Called when an instruction encounters an assertion that two values should be equal.
/// This function can be overridden by subclasses.
fn check_eq(val0: &MaybeRelocatable, val1: &MaybeRelocatable) -> bool {
    val0 == val1
}
