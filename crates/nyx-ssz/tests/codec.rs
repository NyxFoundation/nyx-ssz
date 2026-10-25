use nyx_ssz::{
    codec::{decode, encode, fixed_size, hash_tree_root, is_basic, Ssz},
    error::CodecError,
    scalar, tree_hash,
    type_info::TypeDescriptor,
    U256,
};

fn assert_codec<T: Ssz + std::fmt::Debug + PartialEq>(value: T, bytes: &[u8], root: [u8; 32]) {
    assert_eq!(encode(&value), Ok(bytes.to_vec()));
    assert_eq!(fixed_size::<T>(), Ok(Some(bytes.len())));
    assert!(is_basic::<T>());
    assert_eq!(decode::<T>(bytes), Ok(value));
    let decoded = decode::<T>(bytes).unwrap();
    assert_eq!(hash_tree_root(&decoded), Ok(root));
    assert_eq!(T::decode(&[]), Err(CodecError::InvalidLength));
    assert_eq!(
        T::decode(&vec![0; bytes.len() + 1]),
        Err(CodecError::InvalidLength)
    );
}

#[test]
fn should_encode_decode_and_hash_every_basic_type_through_the_same_trait() {
    assert_codec(true, &[1], tree_hash::hash_tree_root_bool(true));
    assert_codec(false, &[0], tree_hash::hash_tree_root_bool(false));
    assert_codec(
        197u8,
        &scalar::encode_u8(197),
        tree_hash::hash_tree_root_u8(197),
    );
    assert_codec(
        0x8152u16,
        &scalar::encode_u16(0x8152),
        tree_hash::hash_tree_root_u16(0x8152),
    );
    assert_codec(
        u32::MAX,
        &scalar::encode_u32(u32::MAX),
        tree_hash::hash_tree_root_u32(u32::MAX),
    );
    assert_codec(
        u64::MAX,
        &scalar::encode_u64(u64::MAX),
        tree_hash::hash_tree_root_u64(u64::MAX),
    );
    assert_codec(
        u128::MAX,
        &scalar::encode_u128(u128::MAX),
        tree_hash::hash_tree_root_u128(u128::MAX),
    );
    assert_codec(
        U256::MAX,
        &scalar::encode_u256(&U256::MAX),
        tree_hash::hash_tree_root_u256(&U256::MAX),
    );
}

#[derive(Debug, PartialEq)]
struct Oversized;

// An external dictionary is callable, but is not covered by library instance proofs.
impl Ssz for Oversized {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Uint(1);

    fn encoded_len(&self) -> Result<usize, CodecError> {
        Err(CodecError::SizeOverflow)
    }
    fn encode_into(&self, _: &mut [u8]) -> Result<(), CodecError> {
        Err(CodecError::SizeOverflow)
    }
    fn decode(_bytes: &[u8]) -> Result<Self, CodecError> {
        Err(CodecError::SizeOverflow)
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Err(CodecError::SizeOverflow)
    }
}

#[test]
fn should_forward_external_codec_errors_without_certifying_the_implementation() {
    assert_eq!(fixed_size::<Oversized>(), Ok(Some(1)));
    assert_eq!(encode(&Oversized), Err(CodecError::SizeOverflow));
    let error = decode::<Oversized>(&[]).unwrap_err();
    assert_eq!(error, CodecError::SizeOverflow);
    let boxed: Box<dyn std::error::Error> = Box::new(error);
    assert_eq!(
        boxed.to_string(),
        "SSZ size exceeds its supported representation"
    );
    assert_eq!(format!("{boxed:?}"), "SizeOverflow");
    assert!(boxed.source().is_none());
    assert_eq!(hash_tree_root(&Oversized), Err(CodecError::SizeOverflow));
}

#[test]
fn should_expose_protocol_errors_through_standard_error_interfaces() {
    let cases = [
        (
            CodecError::InvalidLength,
            "SSZ input has an invalid byte length",
        ),
        (
            CodecError::InvalidBoolean,
            "SSZ boolean must be encoded as zero or one",
        ),
        (
            CodecError::SizeOverflow,
            "SSZ size exceeds its supported representation",
        ),
    ];
    for (error, expected) in cases {
        let boxed: Box<dyn std::error::Error> = Box::new(error);
        assert_eq!(boxed.to_string(), expected);
        assert!(boxed.source().is_none());
    }
}

#[test]
fn should_reject_noncanonical_booleans_through_the_trait() {
    for byte in 2..=255 {
        assert_eq!(decode::<bool>(&[byte]), Err(CodecError::InvalidBoolean));
    }
}
