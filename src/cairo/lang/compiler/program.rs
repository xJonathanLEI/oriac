use crate::{
    cairo::lang::compiler::{
        debug_info::DebugInfo,
        identifier_manager::IdentifierManager,
        preprocessor::{flow::ReferenceManager, preprocessor::AttributeScope},
        scoped_name::ScopedName,
    },
    serde::big_int::BigIntHex,
};

use num_bigint::BigInt;
use serde::Deserialize;
use serde_with::serde_as;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct CairoHint {
    pub code: String,
    // accessible_scopes: List[ScopedName] = field(
    //     metadata=dict(marshmallow_field=mfields.List(ScopedNameAsStr))
    // )
    // flow_tracking_data: FlowTrackingDataActual
}

pub struct ProgramBase {
    pub prime: BigInt,
    pub data: Vec<BigInt>,
    pub builtins: Vec<String>,
    pub main: Option<BigInt>,
}

/// Cairo program minimal information (stripped from hints, identifiers, etc.). The absence of hints
/// is crucial for security reasons. Can be used for verifying execution.
pub struct StrippedProgram {
    pub prime: BigInt,
    pub data: Vec<BigInt>,
    pub builtins: Vec<String>,
    pub main: BigInt,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Program {
    #[serde_as(as = "BigIntHex")]
    pub prime: BigInt,
    #[serde_as(as = "Vec<BigIntHex>")]
    pub data: Vec<BigInt>,
    #[serde_as(as = "HashMap<BigIntHex, Vec<_>>")]
    pub hints: HashMap<BigInt, Vec<CairoHint>>,
    pub builtins: Vec<String>,
    pub main_scope: ScopedName,
    pub identifiers: IdentifierManager,
    pub reference_manager: ReferenceManager,
    pub attributes: Vec<AttributeScope>,
    pub debug_info: Option<DebugInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_deser() {
        serde_json::from_str::<Program>(include_str!(
            "../../../../test-data/artifacts/run_past_end.json"
        ))
        .unwrap();
    }
}
