use nyx_ssz::{
    bytecollections::ByteVector,
    codec::Ssz,
    collections::{list, List},
    error::CodecError,
};
use std::sync::atomic::{AtomicUsize, Ordering};

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

static LENGTH_CALLS: AtomicUsize = AtomicUsize::new(0);

struct VariableByte(u8);
impl Ssz for VariableByte {
    const TYPE: &'static nyx_ssz::type_info::TypeDescriptor =
        &nyx_ssz::type_info::TypeDescriptor::ByteList(1);
    fn encoded_len(&self) -> Result<usize, CodecError> {
        LENGTH_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(1)
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        nyx_ssz::encoding::copy(output, &[self.0])
    }
    fn decode(input: &[u8]) -> Result<Self, CodecError> {
        if input.len() != 1 {
            return Err(CodecError::InvalidLength);
        }
        Ok(Self(input[0]))
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Ok([self.0; 32])
    }
}

#[derive(nyx_ssz::Ssz)]
struct VariableOnly {
    value: VariableByte,
}

#[test]
fn should_skip_fixed_classification_for_variable_containers() {
    LENGTH_CALLS.store(0, Ordering::SeqCst);
    assert_eq!(
        VariableOnly {
            value: VariableByte(7)
        }
        .encode()
        .unwrap(),
        [4, 0, 0, 0, 7]
    );
    assert_eq!(LENGTH_CALLS.load(Ordering::SeqCst), 1);
}

static FIXED_LENGTH_CALLS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct FixedByte(u8);
impl Ssz for FixedByte {
    const TYPE: &'static nyx_ssz::type_info::TypeDescriptor =
        &nyx_ssz::type_info::TypeDescriptor::Uint(1);

    fn encoded_len(&self) -> Result<usize, CodecError> {
        FIXED_LENGTH_CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(1)
    }

    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        nyx_ssz::encoding::copy(output, &[self.0])
    }

    fn decode(input: &[u8]) -> Result<Self, CodecError> {
        input
            .first()
            .copied()
            .filter(|_| input.len() == 1)
            .map(Self)
            .ok_or(CodecError::InvalidLength)
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Ok([self.0; 32])
    }
}

#[derive(Debug, nyx_ssz::Ssz)]
struct FixedRecord {
    first: FixedByte,
    second: FixedByte,
}

#[test]
fn should_validate_fixed_container_sequences_once_before_bulk_encoding() {
    FIXED_LENGTH_CALLS.store(0, Ordering::SeqCst);
    let values: List<FixedRecord, 2> = list(vec![
        FixedRecord {
            first: FixedByte(1),
            second: FixedByte(2),
        },
        FixedRecord {
            first: FixedByte(3),
            second: FixedByte(4),
        },
    ])
    .unwrap();
    assert_eq!(values.encode().unwrap(), [1, 2, 3, 4]);
    assert_eq!(FIXED_LENGTH_CALLS.load(Ordering::SeqCst), 4);
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
