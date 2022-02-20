use crate::cairo::lang::builtins::{
    hash::instance_def::PedersenInstanceDef, range_check::instance_def::RangeCheckInstanceDef,
    signature::instance_def::EcdsaInstanceDef,
};

pub mod hash;

pub mod range_check;

pub mod signature;

#[derive(Debug)]
pub enum BuiltinDefinition {
    Bool(bool),
    PedersenInstanceDef(PedersenInstanceDef),
    RangeCheckInstanceDef(RangeCheckInstanceDef),
    EcdsaInstanceDef(EcdsaInstanceDef),
}
