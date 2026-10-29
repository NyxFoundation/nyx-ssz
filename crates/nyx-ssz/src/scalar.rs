//! Canonical encodings of SSZ scalar values.

mod basic;

pub use basic::{
    decode_bool, decode_u128, decode_u16, decode_u32, decode_u64, decode_u8, encode_bool,
    encode_u128, encode_u16, encode_u32, encode_u64, encode_u8, ScalarError,
};

/// Serialize an SSZ uint256 without relying on native-endian memory casts.
pub fn encode_u256(value: &crate::U256) -> [u8; 32] {
    let limbs = value.as_limbs();
    let a = encode_u64(limbs[0]);
    let b = encode_u64(limbs[1]);
    let c = encode_u64(limbs[2]);
    let d = encode_u64(limbs[3]);
    [
        a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], b[0], b[1], b[2], b[3], b[4], b[5], b[6],
        b[7], c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], d[0], d[1], d[2], d[3], d[4], d[5],
        d[6], d[7],
    ]
}

/// Deserialize an SSZ uint256, rejecting every input length other than 32.
pub fn decode_u256(bytes: &[u8]) -> Result<crate::U256, ScalarError> {
    let array: [u8; 32] = match bytes.try_into() {
        Ok(array) => array,
        Err(_) => return Err(ScalarError::InvalidLength),
    };
    let a = u64::from_le_bytes([
        array[0], array[1], array[2], array[3], array[4], array[5], array[6], array[7],
    ]);
    let b = u64::from_le_bytes([
        array[8], array[9], array[10], array[11], array[12], array[13], array[14], array[15],
    ]);
    let c = u64::from_le_bytes([
        array[16], array[17], array[18], array[19], array[20], array[21], array[22], array[23],
    ]);
    let d = u64::from_le_bytes([
        array[24], array[25], array[26], array[27], array[28], array[29], array[30], array[31],
    ]);
    Ok(crate::U256::from_limbs([a, b, c, d]))
}
