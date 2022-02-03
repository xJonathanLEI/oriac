use crate::{
    debug_info::DebugInfo, flow::ReferenceManager, identifier_manager::IdentifierManager,
    preprocessor::AttributeScope, scoped_name::ScopedName,
};

use num_bigint::BigInt;
use std::collections::HashMap;

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

pub struct Program {
    pub prime: BigInt,
    pub data: Vec<BigInt>,
    pub hints: HashMap<BigInt, Vec<CairoHint>>,
    pub builtins: Vec<String>,
    pub main_scope: ScopedName,
    pub identifiers: IdentifierManager,
    pub reference_manager: ReferenceManager,
    pub attributes: Vec<AttributeScope>,
    pub debug_info: Option<DebugInfo>,
}
