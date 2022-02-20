use num_bigint::BigInt;
use std::{collections::HashMap, str::FromStr};

use crate::cairo::lang::builtins::{
    hash::instance_def::PedersenInstanceDef, range_check::instance_def::RangeCheckInstanceDef,
    signature::instance_def::EcdsaInstanceDef, BuiltinDefinition,
};

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
    pub builtins: HashMap<String, BuiltinDefinition>,
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

    pub fn small_instance() -> Self {
        Self {
            layout_name: "small",
            cpu_component_step: 1u32.into(),
            rc_units: 16u32.into(),
            builtins: vec![
                (String::from("output"), BuiltinDefinition::Bool(true)),
                (
                    String::from("pedersen"),
                    BuiltinDefinition::PedersenInstanceDef(PedersenInstanceDef {
                        ratio: 8,
                        repetitions: 4,
                        element_height: 256,
                        element_bits: 252,
                        n_inputs: 2,
                        hash_limit: Some(prime()),
                    }),
                ),
                (
                    String::from("range_check"),
                    BuiltinDefinition::RangeCheckInstanceDef(RangeCheckInstanceDef {
                        ratio: 8,
                        n_parts: 8,
                    }),
                ),
                (
                    String::from("ecdsa"),
                    BuiltinDefinition::EcdsaInstanceDef(EcdsaInstanceDef {
                        ratio: 512,
                        repetitions: 1,
                        height: 256,
                        n_hash_bits: 251,
                    }),
                ),
            ]
            .into_iter()
            .collect(),
            public_memory_fraction: 4u32.into(),
            memory_units_per_step: 8u32.into(),
            diluted_pool_instance_def: None,
            n_trace_columns: Some(25u32.into()),
            cpu_instance_def: CpuInstanceDef { safe_call: true },
        }
    }
}

fn prime() -> BigInt {
    BigInt::from_str("3618502788666131213697322783095070105623107215331596699973092056135872020481")
        .unwrap()
}
