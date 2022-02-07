use crate::{
    cairo::lang::compiler::{
        debug_info::DebugInfo,
        identifier_definition::IdentifierDefinition,
        identifier_manager::{IdentifierError, IdentifierManager},
        preprocessor::{
            flow::{FlowTrackingDataActual, ReferenceManager},
            preprocessor::AttributeScope,
        },
        scoped_name::ScopedName,
    },
    serde::big_int::BigIntHex,
};

use num_bigint::BigInt;
use serde::Deserialize;
use serde_with::serde_as;
use std::collections::HashMap;

#[derive(Debug)]
// Simulate inheritance
pub enum Program {
    Stripped(StrippedProgram),
    Full(Box<FullProgram>),
}

#[derive(Debug, Deserialize)]
pub struct CairoHint {
    pub code: String,
    pub accessible_scopes: Vec<ScopedName>,
    pub flow_tracking_data: FlowTrackingDataActual,
}

/// Cairo program minimal information (stripped from hints, identifiers, etc.). The absence of hints
/// is crucial for security reasons. Can be used for verifying execution.
#[derive(Debug)]
pub struct StrippedProgram {
    pub prime: BigInt,
    pub data: Vec<BigInt>,
    pub builtins: Vec<String>,
    pub main: BigInt,
}

#[serde_as]
#[derive(Debug, Deserialize)]
/// Correspond to `Program` in `cairo-lang`.
pub struct FullProgram {
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

impl Program {
    pub fn prime(&self) -> &BigInt {
        match self {
            Self::Stripped(program) => &program.prime,
            Self::Full(program) => &program.prime,
        }
    }

    pub fn data(&self) -> &[BigInt] {
        match self {
            Self::Stripped(program) => &program.data,
            Self::Full(program) => &program.data,
        }
    }

    pub fn builtins(&self) -> &[String] {
        match self {
            Self::Stripped(program) => &program.builtins,
            Self::Full(program) => &program.builtins,
        }
    }

    pub fn main(&self) -> Option<BigInt> {
        match self {
            Self::Stripped(program) => Some(program.main.clone()),
            Self::Full(program) => program.main(),
        }
    }
}

impl From<StrippedProgram> for Program {
    fn from(value: StrippedProgram) -> Self {
        Program::Stripped(value)
    }
}

impl From<FullProgram> for Program {
    fn from(value: FullProgram) -> Self {
        Program::Full(Box::new(value))
    }
}

impl FullProgram {
    pub fn get_identifier(
        &self,
        name: ScopedName,
        _expected_type: &'static str,
        full_name_lookup: bool,
    ) -> Result<IdentifierDefinition, IdentifierError> {
        let result = if full_name_lookup {
            self.identifiers.root.get(name)
        } else {
            self.identifiers.search(&[self.main_scope.clone()], name)
        };

        // TODO: implement these Python lines
        // result.assert_fully_parsed()
        // identifier_definition = result.identifier_definition
        // assert isinstance(identifier_definition, expected_type), (
        //     f"'{scoped_name}' is expected to be {expected_type.TYPE}, "
        //     + f"found {identifier_definition.TYPE}."  # type: ignore
        // )  # type: ignore

        result.map(|result| result.identifier_definition)
    }

    pub fn get_label(&self, name: ScopedName, full_name_lookup: bool) -> Option<BigInt> {
        match self.get_identifier(name, "label", full_name_lookup) {
            Ok(value) => match value {
                IdentifierDefinition::Label { pc, .. } => Some(pc),
                IdentifierDefinition::Function { pc, .. } => Some(pc),
                _ => None,
            },
            Err(_) => None,
        }
    }

    pub fn main(&self) -> Option<BigInt> {
        self.get_label(ScopedName::new(vec![String::from("main")]).unwrap(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_deser() {
        serde_json::from_str::<FullProgram>(include_str!(
            "../../../../test-data/artifacts/run_past_end.json"
        ))
        .unwrap();
    }

    #[test]
    fn test_program_main() {
        let program = serde_json::from_str::<FullProgram>(include_str!(
            "../../../../test-data/artifacts/run_past_end.json"
        ))
        .unwrap();

        assert_eq!(program.main(), Some(BigInt::from(0)));
    }
}
