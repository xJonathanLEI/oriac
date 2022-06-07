use crate::{cairo::lang::compiler::scoped_name::ScopedName, serde::big_int::BigIntNumber};

use num_bigint::BigInt;
use serde::Deserialize;
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IdentifierDefinition {
    /// Represents an identifier that will be defined later in the code.
    Future,
    Alias {
        destination: ScopedName,
    },
    Const,
    Member,
    /// Represents a struct definition.
    ///
    ///```cairo
    /// struct MyStruct:
    ///     ...
    /// end
    ///```
    Struct,
    TypeDefinition,
    Label {
        #[serde_as(as = "BigIntNumber")]
        pc: BigInt,
    },
    Function {
        #[serde_as(as = "BigIntNumber")]
        pc: BigInt,
    },
    Namespace,
    Reference,
    Scope,
}

impl IdentifierDefinition {
    pub fn is_label(&self) -> bool {
        // `Function` inherits from `Label` in Python
        matches!(self, IdentifierDefinition::Label { .. })
            || matches!(self, IdentifierDefinition::Function { .. })
    }
}
