use num_bigint::BigInt;
use serde::{de::Error as DeError, Deserialize, Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

pub struct BigIntHex;

pub struct BigIntNumber;

impl SerializeAs<BigInt> for BigIntHex {
    fn serialize_as<S>(value: &BigInt, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:#x}", value))
    }
}

impl<'de> DeserializeAs<'de, BigInt> for BigIntHex {
    fn deserialize_as<D>(deserializer: D) -> Result<BigInt, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        utils::big_int_from_hex(&value)
            .map_err(|err| DeError::custom(format!("invalid hex string: {}", err)))
    }
}

impl SerializeAs<BigInt> for BigIntNumber {
    fn serialize_as<S>(value: &BigInt, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", value))
    }
}

impl<'de> DeserializeAs<'de, BigInt> for BigIntNumber {
    fn deserialize_as<D>(deserializer: D) -> Result<BigInt, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Ok(BigInt::from(value))
    }
}

mod utils {
    use hex::FromHexError;
    use num_bigint::{BigInt, Sign};

    pub fn big_int_from_hex(value: &str) -> Result<BigInt, FromHexError> {
        let stripped_value = value.trim_start_matches("0x");

        let decoded_bytes = if stripped_value.len() % 2 == 0 {
            hex::decode(&stripped_value)
        } else {
            let mut padded_string = String::from('0');
            padded_string.push_str(stripped_value);

            hex::decode(&padded_string)
        };

        decoded_bytes.map(|bytes| BigInt::from_bytes_be(Sign::Plus, &bytes))
    }
}
