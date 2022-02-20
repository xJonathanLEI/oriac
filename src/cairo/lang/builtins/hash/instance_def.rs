use num_bigint::BigInt;

// Each hash consists of 3 cells (two inputs and one output).
pub const CELLS_PER_HASH: u32 = 3;
pub const INPUT_CELLS_PER_HASH: u32 = 2;

#[derive(Debug)]
pub struct PedersenInstanceDef {
    /// Defines the ratio between the number of steps to the number of pedersen instances.
    /// For every ratio steps, we have one instance.
    pub ratio: u32,

    /// Split to this many different components - for optimization.
    pub repetitions: u32,

    /// Size of hash.
    pub element_height: u32,
    pub element_bits: u32,
    /// Number of inputs for hash.
    pub n_inputs: u32,
    /// The upper bound on the hash inputs. If None, the upper bound is 2^element_bits.
    pub hash_limit: Option<BigInt>,
}
