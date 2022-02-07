use std::collections::HashMap;

use num_bigint::BigInt;

#[derive(Debug)]
pub struct CpuInstanceDef {
    /// Verifies that each 'call' instruction returns, even if the called function is malicious.
    pub safe_call: bool,
}

#[derive(Debug)]
pub struct DilutedPoolInstanceDef {
    /// The ratio between the number of diluted cells in the pool and the number of cpu steps.
    pub units_per_step: BigInt,
    /// In diluted form the binary sequence **** of length n_bits is represented as 00*00*00*00*,
    /// with (spacing - 1) zero bits between consecutive information carying bits.
    pub spacing: BigInt,
    /// The number of (information) bits (before diluting).
    pub n_bits: BigInt,
}

#[derive(Debug)]
pub struct CairoLayout {
    pub layout_name: &'static str,
    pub cpu_component_step: BigInt,
    /// Range check units.
    pub rc_units: BigInt,
    pub builtins: HashMap<String, ()>,
    /// The ratio between the number of public memory cells and the total number of memory cells.
    pub public_memory_fraction: BigInt,
    pub memory_units_per_step: BigInt,
    pub diluted_pool_instance_def: Option<DilutedPoolInstanceDef>,
    pub n_trace_columns: Option<BigInt>,
    pub cpu_instance_def: CpuInstanceDef,
}

impl CairoLayout {
    pub fn plain_instance() -> Self {
        Self {
            layout_name: "plain",
            cpu_component_step: 1u32.into(),
            rc_units: 16u32.into(),
            builtins: HashMap::new(),
            public_memory_fraction: 4u32.into(),
            memory_units_per_step: 8u32.into(),
            diluted_pool_instance_def: None,
            n_trace_columns: Some(8u32.into()),
            cpu_instance_def: CpuInstanceDef { safe_call: true },
        }
    }
}
