use nyx_ssz::{bytecollections::ByteVector, codec::Ssz, error::CodecError};

struct Overflowing;
impl Ssz for Overflowing {
    const TYPE: &'static nyx_ssz::type_info::TypeDescriptor =
        &nyx_ssz::type_info::TypeDescriptor::Uint(1);
    fn encoded_len(&self) -> Result<usize, CodecError> {
        Err(CodecError::NativeOutputOverflow)
    }
    fn encode_into(&self, _: &mut [u8]) -> Result<(), CodecError> {
        Err(CodecError::NativeOutputOverflow)
    }
    fn decode(_: &[u8]) -> Result<Self, CodecError> {
        Ok(Self)
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Ok([0; 32])
    }
}

struct BadBool;
impl Ssz for BadBool {
    const TYPE: &'static nyx_ssz::type_info::TypeDescriptor =
        &nyx_ssz::type_info::TypeDescriptor::Boolean;
    fn encoded_len(&self) -> Result<usize, CodecError> {
        Err(CodecError::InvalidBoolean)
    }
    fn encode_into(&self, _: &mut [u8]) -> Result<(), CodecError> {
        Err(CodecError::InvalidBoolean)
    }
    fn decode(_: &[u8]) -> Result<Self, CodecError> {
        Ok(Self)
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Ok([0; 32])
    }
}

#[derive(nyx_ssz::Ssz)]
struct FixedPair {
    first: Overflowing,
    second: BadBool,
}

#[derive(nyx_ssz::Ssz)]
struct FixedBudget {
    first: Overflowing,
    second: u8,
}

#[derive(nyx_ssz::Ssz)]
struct Header {
    slot: u64,
    root: ByteVector<32>,
}

#[test]
fn should_keep_fixed_encode_error_precedence_and_write_exact_bytes() {
    assert_eq!(
        FixedPair {
            first: Overflowing,
            second: BadBool
        }
        .encode(),
        Err(CodecError::InvalidBoolean)
    );
    assert_eq!(
        FixedBudget {
            first: Overflowing,
            second: 1
        }
        .encode(),
        Err(CodecError::SizeOverflow)
    );
    let value = Header {
        slot: 0x0102,
        root: ByteVector::from_bytes(vec![7; 32]).unwrap(),
    };
    let bytes = value.encode().unwrap();
    assert_eq!(bytes.len(), 40);
    assert_eq!(&bytes[..8], &0x0102_u64.to_le_bytes());
    assert_eq!(&bytes[8..], &[7; 32]);
    assert_eq!(value.encoded_len(), Ok(40));
    let mut output = [0; 40];
    value.encode_into(&mut output).unwrap();
    assert_eq!(output.as_slice(), bytes.as_slice());
    assert_eq!(
        value.encode_into(&mut [0; 39]),
        Err(CodecError::InvalidLength)
    );
}
