use crate::cairo::lang::compiler::instruction::Instruction;

use num_bigint::BigInt;

/// Given 1 or 2 integers representing an instruction, returns the Instruction. If imm is given for
/// an instruction with no immediate, it will be ignored.
#[allow(unused)]
pub fn decode_instruction(encoding: BigInt, imm: Option<BigInt>) -> Instruction {
    todo!()
}
