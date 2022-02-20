// Each signature consists of 2 cells (a public key and a message).
pub const CELLS_PER_SIGNATURE: u32 = 2;
pub const INPUT_CELLS_PER_SIGNATURE: u32 = 2;

#[derive(Debug)]
pub struct EcdsaInstanceDef {
    /// Defines the ratio between the number of steps to the number of ECDSA instances.
    /// For every ratio steps, we have one instance.
    pub ratio: u32,
    /// Split to this many different components - for optimization.
    pub repetitions: u32,
    /// Size of hash.
    pub height: u32,
    pub n_hash_bits: u32,
}
