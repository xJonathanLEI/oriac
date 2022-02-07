use crate::cairo::lang::compiler::instruction::{
    decode_instruction_values, ApUpdate, FpUpdate, Instruction, Op1Addr, Opcode, PcUpdate,
    Register, Res, OFFSET_BITS,
};

use num_bigint::BigInt;

const DST_REG_BIT: u32 = 0;
const OP0_REG_BIT: u32 = 1;
const OP1_IMM_BIT: u32 = 2;
const OP1_FP_BIT: u32 = 3;
const OP1_AP_BIT: u32 = 4;
const RES_ADD_BIT: u32 = 5;
const RES_MUL_BIT: u32 = 6;
const PC_JUMP_ABS_BIT: u32 = 7;
const PC_JUMP_REL_BIT: u32 = 8;
const PC_JNZ_BIT: u32 = 9;
const AP_ADD_BIT: u32 = 10;
const AP_ADD1_BIT: u32 = 11;
const OPCODE_CALL_BIT: u32 = 12;
const OPCODE_RET_BIT: u32 = 13;
const OPCODE_ASSERT_EQ_BIT: u32 = 14;

/// Given 1 or 2 integers representing an instruction, returns the Instruction. If imm is given for
/// an instruction with no immediate, it will be ignored.
#[allow(unused)]
pub fn decode_instruction(encoding: BigInt, imm: Option<BigInt>) -> Instruction {
    let (flags, off0_enc, off1_enc, off2_enc) = decode_instruction_values(&encoding);

    // Get dst_register.
    let dst_register = if (&flags >> DST_REG_BIT) & BigInt::from(1) > BigInt::from(0) {
        Register::FP
    } else {
        Register::AP
    };

    // Get op0_register.
    let op0_register = if (&flags >> OP0_REG_BIT) & BigInt::from(1) > BigInt::from(0) {
        Register::FP
    } else {
        Register::AP
    };

    // Get op1.
    let op1_addr = match (
        (&flags >> OP1_IMM_BIT) & BigInt::from(1) > BigInt::from(0),
        (&flags >> OP1_AP_BIT) & BigInt::from(1) > BigInt::from(0),
        (&flags >> OP1_FP_BIT) & BigInt::from(1) > BigInt::from(0),
    ) {
        (true, false, false) => Op1Addr::IMM,
        (false, true, false) => Op1Addr::AP,
        (false, false, true) => Op1Addr::FP,
        (false, false, false) => Op1Addr::OP0,
        // TODO: switch to proper error handling
        _ => panic!("invalid op1 encoding"),
    };

    let imm = match &op1_addr {
        Op1Addr::IMM => {
            if imm.is_none() {
                // TODO: switch to proper error handling
                panic!("op1_addr is Op1Addr.IMM, but no immediate given");
            }
            imm
        }
        _ => None,
    };

    // Get pc_update.
    let pc_update = match (
        (&flags >> PC_JUMP_ABS_BIT) & BigInt::from(1) > BigInt::from(0),
        (&flags >> PC_JUMP_REL_BIT) & BigInt::from(1) > BigInt::from(0),
        (&flags >> PC_JNZ_BIT) & BigInt::from(1) > BigInt::from(0),
    ) {
        (true, false, false) => PcUpdate::JUMP,
        (false, true, false) => PcUpdate::JUMP_REL,
        (false, false, true) => PcUpdate::JNZ,
        (false, false, false) => PcUpdate::REGULAR,
        // TODO: switch to proper error handling
        _ => panic!("invalid pc_update encoding"),
    };

    // Get res.
    let res = match (
        (&flags >> RES_ADD_BIT) & BigInt::from(1) > BigInt::from(0),
        (&flags >> RES_MUL_BIT) & BigInt::from(1) > BigInt::from(0),
    ) {
        (true, false) => Res::ADD,
        (false, true) => Res::MUL,
        (false, false) => match &pc_update {
            PcUpdate::JNZ => Res::UNCONSTRAINED,
            _ => Res::OP1,
        },
        // TODO: switch to proper error handling
        _ => panic!("invalid res encoding"),
    };

    // JNZ opcode means res must be UNCONSTRAINED.
    if matches!(pc_update, PcUpdate::JNZ) && !matches!(res, Res::UNCONSTRAINED) {
        // TODO: switch to proper error handling
        panic!("JNZ opcode means res must be UNCONSTRAINED");
    }

    // Get ap_update.
    let mut ap_update = match (
        (&flags >> AP_ADD_BIT) & BigInt::from(1) > BigInt::from(0),
        (&flags >> AP_ADD1_BIT) & BigInt::from(1) > BigInt::from(0),
    ) {
        (true, false) => ApUpdate::ADD,
        (false, true) => ApUpdate::ADD1,
        (false, false) => ApUpdate::REGULAR, // OR ADD2, depending if we have CALL opcode.
        // TODO: switch to proper error handling
        _ => panic!("invalid ap_update encoding"),
    };

    // Get opcode.
    let opcode = match (
        (&flags >> OPCODE_CALL_BIT) & BigInt::from(1) > BigInt::from(0),
        (&flags >> OPCODE_RET_BIT) & BigInt::from(1) > BigInt::from(0),
        (&flags >> OPCODE_ASSERT_EQ_BIT) & BigInt::from(1) > BigInt::from(0),
    ) {
        (true, false, false) => Opcode::CALL,
        (false, true, false) => Opcode::RET,
        (false, false, true) => Opcode::ASSERT_EQ,
        (false, false, false) => Opcode::NOP,
        // TODO: switch to proper error handling
        _ => panic!("invalid opcode encoding"),
    };

    // CALL opcode means ap_update must be ADD2.
    if matches!(opcode, Opcode::CALL) {
        if !matches!(ap_update, ApUpdate::REGULAR) {
            // TODO: switch to proper error handling
            panic!("CALL must have update_ap is ADD2");
        }
        ap_update = ApUpdate::ADD2;
    }

    // Get fp_update.
    let fp_update = match &opcode {
        Opcode::CALL => FpUpdate::AP_PLUS2,
        Opcode::RET => FpUpdate::DST,
        _ => FpUpdate::REGULAR,
    };

    Instruction {
        off0: (off0_enc as i32 - 2i32.pow(OFFSET_BITS - 1)) as i16,
        off1: (off1_enc as i32 - 2i32.pow(OFFSET_BITS - 1)) as i16,
        off2: (off2_enc as i32 - 2i32.pow(OFFSET_BITS - 1)) as i16,
        imm,
        dst_register,
        op0_register,
        op1_addr,
        res,
        pc_update,
        ap_update,
        fp_update,
        opcode,
    }
}
