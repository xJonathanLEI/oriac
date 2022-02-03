use num_bigint::BigInt;

pub struct AttributeBase {
    pub name: String,
    pub value: String,
}

pub struct AttributeScope {
    pub name: String,
    pub value: String,
    pub start_pc: BigInt,
    pub end_pc: BigInt,
}
