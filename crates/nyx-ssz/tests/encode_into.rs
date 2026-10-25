use nyx_ssz::{
    bitcollections::{BitList, BitVector, BoundedProgressiveBitList, ProgressiveBitList},
    bytecollections::{ByteList, ByteVector},
    codec::Ssz,
    collections::{bounded_progressive_list, list, progressive_list, vector},
    error::CodecError,
    type_info::TypeDescriptor,
    U256,
};

fn check<T: Ssz>(value: T, expected: &[u8]) {
    assert_eq!(value.encoded_len(), Ok(expected.len()));
    assert_eq!(value.encode().unwrap(), expected);
    let mut guarded = vec![0xa5; expected.len() + 2];
    value
        .encode_into(&mut guarded[1..expected.len() + 1])
        .unwrap();
    assert_eq!(&guarded[1..expected.len() + 1], expected);
    assert_eq!((guarded[0], guarded[expected.len() + 1]), (0xa5, 0xa5));
    for length in [expected.len().checked_sub(1), Some(expected.len() + 1)]
        .into_iter()
        .flatten()
    {
        let mut wrong = vec![0xa5; length];
        assert_eq!(
            value.encode_into(&mut wrong),
            Err(CodecError::InvalidLength)
        );
        assert!(wrong.iter().all(|&byte| byte == 0xa5));
    }
}

#[test]
fn should_write_exact_scalar_bytes_and_reject_wrong_sized_destinations() {
    check(true, &[1]);
    check(3_u8, &[3]);
    check(0x1234_u16, &[0x34, 0x12]);
    check(0x12345678_u32, &0x12345678_u32.to_le_bytes());
    check(u64::MAX, &[255; 8]);
    check(u128::MAX, &[255; 16]);
    check(U256::MAX, &[255; 32]);
}

#[cfg(target_endian = "little")]
#[test]
fn should_borrow_canonical_native_integer_sequence_bytes() {
    let values = [0x0102_u16, 0x0304];
    assert_eq!(
        u16::fixed_sequence_bytes(&values),
        Some([2, 1, 4, 3].as_slice())
    );
}

#[test]
fn should_write_owned_bytes_and_bits_without_temporary_encodings() {
    check(
        ByteVector::<3>::from_bytes(vec![1, 2, 3]).unwrap(),
        &[1, 2, 3],
    );
    check(ByteList::<0>::from_bytes(vec![]).unwrap(), &[]);
    check(ByteList::<3>::from_bytes(vec![1, 2]).unwrap(), &[1, 2]);
    check(BitVector::<9>::from_bits(&[true; 9]).unwrap(), &[255, 1]);
    check(BitList::<9>::from_bits(&[true; 9]).unwrap(), &[255, 3]);
    check(ProgressiveBitList::from_bits(&[]).unwrap(), &[1]);
    check(
        BoundedProgressiveBitList::<9>::from_ssz_bytes(vec![255, 3]).unwrap(),
        &[255, 3],
    );
}

#[test]
fn should_preserve_empty_bodies_and_relative_offsets_in_nested_sequences() {
    check(vector::<_, 2>(vec![1_u16, 2]).unwrap(), &[1, 0, 2, 0]);
    check(list::<u16, 0>(vec![]).unwrap(), &[]);
    check(progressive_list(vec![1_u16, 2]), &[1, 0, 2, 0]);
    check(
        bounded_progressive_list::<_, 2>(vec![1_u16, 2]).unwrap(),
        &[1, 0, 2, 0],
    );
    let values = vec![
        ByteList::<2>::from_bytes(vec![]).unwrap(),
        ByteList::from_bytes(vec![7]).unwrap(),
    ];
    check(list::<_, 2>(values).unwrap(), &[8, 0, 0, 0, 8, 0, 0, 0, 7]);
}

#[derive(nyx_ssz::Ssz)]
struct Record {
    first: u16,
    body: ByteList<4>,
    last: u8,
}
#[derive(nyx_ssz::Ssz)]
#[ssz(progressive)]
struct Sparse {
    #[ssz(position = 4)]
    last: u16,
    #[ssz(position = 0)]
    first: bool,
}
#[derive(nyx_ssz::Ssz)]
enum Choice {
    #[ssz(selector = 127)]
    Body(Record),
}

#[test]
fn should_write_derived_mixed_sparse_and_union_values_in_one_destination() {
    let wire = [0x34, 0x12, 7, 0, 0, 0, 9, 1, 2, 3];
    check(
        Record {
            first: 0x1234,
            body: ByteList::from_bytes(vec![1, 2, 3]).unwrap(),
            last: 9,
        },
        &wire,
    );
    check(
        Sparse {
            last: 5,
            first: true,
        },
        &[1, 5, 0],
    );
    let value = Choice::Body(Record {
        first: 0x1234,
        body: ByteList::from_bytes(vec![1, 2, 3]).unwrap(),
        last: 9,
    });
    let mut expected = vec![127];
    expected.extend_from_slice(&wire);
    check(value, &expected);
}

#[derive(Debug)]
struct BadSequenceBytes(u8);
impl Ssz for BadSequenceBytes {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Uint(1);
    fn encoded_len(&self) -> Result<usize, CodecError> {
        Ok(1)
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        self.0.encode_into(output)
    }
    fn decode(input: &[u8]) -> Result<Self, CodecError> {
        u8::decode(input).map(Self)
    }
    fn fixed_sequence_bytes(_values: &[Self]) -> Option<&[u8]> {
        Some(&[])
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        self.0.hash_tree_root()
    }
}

#[test]
fn should_reject_a_fixed_sequence_hook_with_the_wrong_byte_length() {
    let value = list::<_, 1>(vec![BadSequenceBytes(7)]).unwrap();
    assert_eq!(value.encoded_len(), Err(CodecError::InvalidLength));
    assert_eq!(value.encode(), Err(CodecError::InvalidLength));
    assert_eq!(value.hash_tree_root(), Err(CodecError::InvalidLength));
}

// A concrete all-zero byte vector represented by length, not allocated storage.
#[derive(Debug)]
struct ZeroVector<const N: usize>;
impl<const N: usize> Ssz for ZeroVector<N> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::ByteVector(N);
    fn encoded_len(&self) -> Result<usize, CodecError> {
        if N == 0 {
            return Err(CodecError::EmptyVector);
        }
        Ok(N)
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        nyx_ssz::encoding::check_length(output, self.encoded_len()?)?;
        output.fill(0);
        Ok(())
    }
    fn decode(input: &[u8]) -> Result<Self, CodecError> {
        if N == 0 || input.len() != N || input.iter().any(|&byte| byte != 0) {
            return Err(CodecError::InvalidLength);
        }
        Ok(Self)
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        self.encoded_len()?;
        nyx_ssz::merkle::merkleize(&[], Some(N.div_ceil(32))).map_err(|_| CodecError::InvalidLength)
    }
}
#[derive(nyx_ssz::Ssz)]
enum Maximum {
    #[ssz(selector = 1)]
    Bytes(ZeroVector<{ usize::MAX }>),
}
#[derive(nyx_ssz::Ssz)]
enum Representable {
    #[ssz(selector = 1)]
    Bytes(ZeroVector<{ usize::MAX - 1 }>),
}

#[test]
fn should_compute_native_and_protocol_boundaries_without_allocating_huge_bodies() {
    assert_eq!(
        Maximum::Bytes(ZeroVector).encoded_len(),
        Err(CodecError::NativeOutputOverflow)
    );
    assert_eq!(
        Maximum::Bytes(ZeroVector).encode(),
        Err(CodecError::NativeOutputOverflow)
    );
    assert_eq!(
        Representable::Bytes(ZeroVector).encoded_len(),
        Ok(usize::MAX)
    );
    let huge = vector::<_, 2>(vec![ZeroVector::<{ usize::MAX }>, ZeroVector]).unwrap();
    assert_eq!(huge.encoded_len(), Err(CodecError::SizeOverflow));
    let mut wrong = [0xa5; 8];
    assert_eq!(
        Representable::Bytes(ZeroVector).encode_into(&mut wrong),
        Err(CodecError::InvalidLength)
    );
    assert_eq!(wrong, [0xa5; 8]);
}
