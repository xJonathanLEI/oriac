use rustpython::vm::bytecode::CodeObject;

#[derive(Debug)]
pub struct CompiledHint {
    pub compiled: CodeObject,
    pub consts: (),
}

// There's no `VirtualMachineBase`. All base class functionalities have been merged into
// `VirtualMachine`.
