use crate::cairo::lang::compiler::references::Reference;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ReferenceManager {
    pub references: Vec<Reference>,
}

#[derive(Debug, Deserialize)]
pub struct FlowTrackingDataActual {}
