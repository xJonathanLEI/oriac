use num_bigint::BigInt;

pub enum Register {
    AP = 0,
    FP = 1,
}

pub enum Op1Addr {
    /// op1 = [pc + 1].
    IMM = 0,
    /// op1 = [ap + off2].
    AP = 1,
    /// op1 = [fp + off2].
    FP = 2,
    /// op1 = [op0].
    OP0 = 3,
}

pub enum Res {
    /// res = operand_1.
    OP1 = 0,
    /// res = operand_0 + operand_1.
    ADD = 1,
    /// res = operand_0 * operand_1.
    MUL = 2,
    /// res is not constrained.
    UNCONSTRAINED = 3,
}

/// Flags for register update.
#[allow(non_camel_case_types)]
pub enum PcUpdate {
    /// Next pc: pc + op_size.
    REGULAR = 0,
    /// Next pc: res (jmp abs).
    JUMP = 1,
    /// Next pc: pc + res (jmp rel).
    JUMP_REL = 2,
    /// Next pc: jnz_addr (jnz), where jnz_addr is a complex expression, representing the jnz logic.
    JNZ = 3,
}

pub enum ApUpdate {
    /// Next ap: ap.
    REGULAR = 0,
    /// Next ap: ap + [pc + 1].
    ADD = 1,
    /// Next ap: ap + 1.
    ADD1 = 2,
    /// Next ap: ap + 2.
    ADD2 = 3,
}

#[allow(non_camel_case_types)]
pub enum FpUpdate {
    /// Next fp: fp.
    REGULAR = 0,
    /// Next fp: ap + 2.
    AP_PLUS2 = 1,
    /// Next fp: operand_dst.
    DST = 2,
}

#[allow(non_camel_case_types)]
pub enum Opcode {
    NOP = 0,
    ASSERT_EQ = 1,
    CALL = 2,
    RET = 3,
}

pub struct Instruction {
    /// Offset. In the range [-2**15, 2*15) = [-2**(OFFSET_BITS-1), 2**(OFFSET_BITS-1)).
    pub off0: i16,
    /// Offset. In the range [-2**15, 2*15) = [-2**(OFFSET_BITS-1), 2**(OFFSET_BITS-1)).
    pub off1: i16,
    /// Offset. In the range [-2**15, 2*15) = [-2**(OFFSET_BITS-1), 2**(OFFSET_BITS-1)).
    pub off2: i16,
    /// Immediate.
    pub imm: Option<BigInt>,
    /// Flag for operands.
    pub dst_register: Register,
    /// Flag for operands.
    pub op0_register: Register,
    pub op1_addr: Op1Addr,
    pub res: Res,
    pub pc_update: PcUpdate,
    pub ap_update: ApUpdate,
    pub fp_update: FpUpdate,
    pub opcode: Opcode,
}

impl Instruction {
    pub fn size(&self) -> u32 {
        if self.imm.is_some() {
            2
        } else {
            1
        }
    }
}
