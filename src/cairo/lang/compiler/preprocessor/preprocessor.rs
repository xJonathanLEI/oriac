use crate::serde::big_int::BigIntHex;

use num_bigint::BigInt;
use serde::Deserialize;
use serde_with::serde_as;

pub struct AttributeBase {
    pub name: String,
    pub value: String,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct AttributeScope {
    pub name: String,
    pub value: String,
    #[serde_as(as = "BigIntHex")]
    pub start_pc: BigInt,
    #[serde_as(as = "BigIntHex")]
    pub end_pc: BigInt,
}
