use serde::Deserialize;

use crate::cairo::lang::compiler::scoped_name::ScopedName;

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
    Label {
        pc: u64,
    },
    Function {
        pc: u64,
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
