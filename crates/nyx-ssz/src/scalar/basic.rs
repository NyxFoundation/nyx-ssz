//! Canonical encodings of SSZ Boolean and native unsigned scalar values.

use std::convert::TryInto;

/// A scalar encoding violates the SSZ byte representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarError {
    InvalidLength,
    InvalidBoolean,
}

/// Serialize an SSZ boolean to its unique one-byte encoding.
pub fn encode_bool(value: bool) -> [u8; 1] {
    if value {
        [1]
    } else {
        [0]
    }
}

/// Deserialize a boolean, rejecting noncanonical bytes and lengths.
pub fn decode_bool(bytes: &[u8]) -> Result<bool, ScalarError> {
    if bytes.len() != 1 {
        return Err(ScalarError::InvalidLength);
    }
    match bytes[0] {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ScalarError::InvalidBoolean),
    }
}

macro_rules! integer_codec {
    ($ty:ty, $size:literal, $encode:ident, $decode:ident) => {
        #[doc = concat!("Serialize an SSZ `", stringify!($ty), "` in little-endian order.")]
        pub fn $encode(value: $ty) -> [u8; $size] {
            value.to_le_bytes()
        }

        #[doc = concat!("Deserialize an SSZ `", stringify!($ty), "`, rejecting every other byte length.")]
        pub fn $decode(bytes: &[u8]) -> Result<$ty, ScalarError> {
            match bytes.try_into() {
                Ok(array) => Ok(<$ty>::from_le_bytes(array)),
                Err(_) => Err(ScalarError::InvalidLength),
            }
        }
    };
}

integer_codec!(u8, 1, encode_u8, decode_u8);
integer_codec!(u16, 2, encode_u16, decode_u16);
integer_codec!(u32, 4, encode_u32, decode_u32);
integer_codec!(u64, 8, encode_u64, decode_u64);
integer_codec!(u128, 16, encode_u128, decode_u128);
