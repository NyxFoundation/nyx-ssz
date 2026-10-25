use nyx_ssz::{
    codec::{fixed_size, is_basic, is_compatible, validate_type, Ssz},
    error::CodecError,
    type_info::TypeDescriptor,
    U256,
};

fn assert_declaration<T: Ssz>(expected: TypeDescriptor, width: usize) {
    let declaration: &'static TypeDescriptor = T::TYPE;
    assert_eq!(*declaration, expected);
    assert_eq!(fixed_size::<T>(), Ok(Some(width)));
    assert!(is_basic::<T>());
    assert_eq!(validate_type::<T>(), Ok(()));
}

#[test]
fn should_derive_every_basic_types_metadata_from_its_static_declaration() {
    assert_declaration::<bool>(TypeDescriptor::Boolean, 1);
    assert_declaration::<u8>(TypeDescriptor::Uint(1), 1);
    assert_declaration::<u16>(TypeDescriptor::Uint(2), 2);
    assert_declaration::<u32>(TypeDescriptor::Uint(4), 4);
    assert_declaration::<u64>(TypeDescriptor::Uint(8), 8);
    assert_declaration::<u128>(TypeDescriptor::Uint(16), 16);
    assert_declaration::<U256>(TypeDescriptor::Uint(32), 32);
}

struct DeclaredUint<const WIDTH: usize>;

// Metadata inspection must not invoke any value codec, even on an external type.
impl<const WIDTH: usize> Ssz for DeclaredUint<WIDTH> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Uint(WIDTH);

    fn encoded_len(&self) -> Result<usize, CodecError> {
        panic!("metadata must not invoke encoded_len")
    }
    fn encode_into(&self, _: &mut [u8]) -> Result<(), CodecError> {
        panic!("metadata must not invoke encode_into")
    }
    fn decode(_: &[u8]) -> Result<Self, CodecError> {
        panic!("metadata inspection must not call decode")
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        panic!("metadata inspection must not call hash_tree_root")
    }
}

fn assert_invalid_width<const WIDTH: usize>() {
    assert_eq!(
        validate_type::<DeclaredUint<WIDTH>>(),
        Err(CodecError::InvalidIntegerWidth)
    );
    // Layout metadata and declaration validity are distinct in the reference.
    assert_eq!(fixed_size::<DeclaredUint<WIDTH>>(), Ok(Some(WIDTH)));
    assert!(is_basic::<DeclaredUint<WIDTH>>());
}

#[test]
fn should_validate_fixed_sequence_declarations_before_invoking_external_callbacks() {
    assert_eq!(
        DeclaredUint::<3>::decode_fixed_sequence(&[]).err().unwrap(),
        CodecError::InvalidIntegerWidth
    );
    assert!(DeclaredUint::<1>::decode_fixed_sequence(&[])
        .unwrap()
        .is_empty());
}

#[test]
fn should_reject_illegal_integer_declarations_without_invoking_value_codecs() {
    assert_invalid_width::<0>();
    assert_invalid_width::<3>();
    assert_invalid_width::<{ usize::MAX }>();
}

#[test]
fn should_compare_declarations_without_confusing_equal_widths_with_equal_types() {
    assert!(is_compatible::<bool, bool>());
    assert!(is_compatible::<u16, u16>());
    assert!(is_compatible::<U256, U256>());
    assert!(!is_compatible::<bool, u8>());
    assert!(!is_compatible::<u8, bool>());
    assert!(!is_compatible::<u8, u16>());
    assert!(!is_compatible::<U256, u128>());
    assert!(is_compatible::<DeclaredUint<3>, DeclaredUint<3>>());
    assert!(!is_compatible::<DeclaredUint<3>, u16>());
}

#[test]
fn should_explain_invalid_integer_declarations_without_exposing_internals() {
    assert_eq!(
        CodecError::InvalidIntegerWidth.to_string(),
        "SSZ unsigned integer width must be 1, 2, 4, 8, 16 or 32 bytes"
    );
}

const NEW_SHAPES: [TypeDescriptor; 8] = [
    TypeDescriptor::ByteVector(2),
    TypeDescriptor::ByteList(3),
    TypeDescriptor::BitVector(9),
    TypeDescriptor::BitList(3),
    TypeDescriptor::ProgressiveBitList(Some(usize::MAX)),
    TypeDescriptor::Container(&["字段🦀"], &[&TypeDescriptor::Uint(2)]),
    TypeDescriptor::ProgressiveContainer(&[false, true], &["é\0"], &[&TypeDescriptor::Uint(2)]),
    TypeDescriptor::CompatibleUnion(
        &[1, 127],
        &[
            &TypeDescriptor::ByteVector(2),
            &TypeDescriptor::Vector(&TypeDescriptor::Uint(1), 2),
        ],
    ),
];

// Deliberately metadata-only external dictionaries: no value operation is invoked.
struct DeclaredShape<const INDEX: usize>;

impl<const INDEX: usize> Ssz for DeclaredShape<INDEX> {
    const TYPE: &'static TypeDescriptor = &NEW_SHAPES[INDEX];

    fn encoded_len(&self) -> Result<usize, CodecError> {
        panic!("metadata must not invoke encoded_len")
    }
    fn encode_into(&self, _: &mut [u8]) -> Result<(), CodecError> {
        panic!("metadata must not invoke encode_into")
    }
    fn decode(_: &[u8]) -> Result<Self, CodecError> {
        panic!("metadata inspection must not call decode")
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        panic!("metadata inspection must not call hash_tree_root")
    }
}

fn assert_composite<const INDEX: usize>(width: Option<usize>) {
    assert_eq!(fixed_size::<DeclaredShape<INDEX>>(), Ok(width));
    assert!(!is_basic::<DeclaredShape<INDEX>>());
    assert_eq!(validate_type::<DeclaredShape<INDEX>>(), Ok(()));
    assert!(is_compatible::<DeclaredShape<INDEX>, DeclaredShape<INDEX>>());
    assert_eq!(*DeclaredShape::<INDEX>::TYPE, NEW_SHAPES[INDEX]);
}

#[test]
fn all_new_families_use_the_same_existing_public_metadata_apis() {
    assert_composite::<0>(Some(2));
    assert_composite::<1>(None);
    assert_composite::<2>(Some(2));
    assert_composite::<3>(None);
    assert_composite::<4>(None);
    assert_composite::<5>(Some(2));
    assert_composite::<6>(Some(2));
    assert_composite::<7>(None);
    assert!(is_compatible::<
        DeclaredShape<0>,
        nyx_ssz::collections::Vector<u8, 2>,
    >());
    assert!(is_compatible::<
        DeclaredShape<1>,
        nyx_ssz::collections::List<u8, 3>,
    >());
    assert!(!is_compatible::<DeclaredShape<5>, DeclaredShape<6>>());
}
