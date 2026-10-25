use nyx_ssz::{
    bytecollections::{ByteList, ByteVector},
    codec::{
        decode, encode, fixed_size, hash_tree_root, is_basic, is_compatible, validate_type, Ssz,
    },
    collections::{list, vector, List, ProgressiveList, Vector},
    error::CodecError,
    tree_hash::{hash_tree_root_byte_list, hash_tree_root_byte_vector},
    type_info::TypeDescriptor,
};

#[test]
fn constructors_move_allocations_and_expose_exact_bytes() {
    let inline = ByteVector::<3>::from_bytes(vec![0, 128, 255]).unwrap();
    assert_eq!(inline.as_bytes(), &[0, 128, 255]);
    let mut bytes = Vec::with_capacity(41);
    bytes.extend([7; 33]);
    let pointer = bytes.as_ptr();
    let value = ByteVector::<33>::from_bytes(bytes).unwrap();
    assert_eq!(value.as_bytes().as_ptr(), pointer);
    assert_eq!(value.as_bytes(), &[7; 33]);
    let mut bytes = Vec::with_capacity(43);
    bytes.extend([255, 0, 128]);
    let pointer = bytes.as_ptr();
    let value = ByteList::<7>::from_bytes(bytes).unwrap();
    assert_eq!(value.as_bytes().as_ptr(), pointer);
    assert_eq!(value.as_bytes(), &[255, 0, 128]);
    let bytes = Vec::with_capacity(17);
    let pointer = bytes.as_ptr();
    let empty = ByteList::<0>::from_bytes(bytes).unwrap();
    assert_eq!(empty.as_bytes().as_ptr(), pointer);
    assert!(empty.as_bytes().is_empty());
}

#[test]
fn invalid_lengths_and_zero_declarations_have_exact_faults() {
    assert_eq!(
        ByteVector::<0>::from_bytes(vec![]).unwrap_err(),
        CodecError::EmptyVector
    );
    assert_eq!(
        ByteVector::<0>::from_bytes(vec![1]).unwrap_err(),
        CodecError::EmptyVector
    );
    assert_eq!(
        ByteVector::<2>::from_bytes(vec![1]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(
        ByteVector::<2>::from_bytes(vec![1, 2, 3]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(
        ByteList::<0>::from_bytes(vec![1]).unwrap_err(),
        CodecError::OverLimit
    );
    assert_eq!(
        ByteList::<2>::from_bytes(vec![1, 2, 3]).unwrap_err(),
        CodecError::OverLimit
    );
    assert_eq!(
        decode::<ByteVector<0>>(&[1]).unwrap_err(),
        CodecError::EmptyVector
    );
    assert_eq!(
        decode::<ByteVector<2>>(&[1]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(
        decode::<ByteList<0>>(&[1]).unwrap_err(),
        CodecError::OverLimit
    );
}

#[test]
fn encoding_decoding_and_complete_roots_match_borrowed_bytes() {
    for bytes in [
        vec![],
        vec![0],
        vec![255; 31],
        vec![128; 32],
        vec![17; 33],
        (0..=255).collect(),
    ] {
        let value = ByteList::<256>::from_bytes(bytes.clone()).unwrap();
        assert_eq!(encode(&value).unwrap(), bytes);
        assert_eq!(decode::<ByteList<256>>(&bytes).unwrap().as_bytes(), bytes);
        assert_eq!(
            hash_tree_root(&value),
            hash_tree_root_byte_list(&bytes, 256)
        );
    }
    let bytes = [0, 255, 128, 42];
    let value = ByteVector::<4>::from_bytes(bytes.to_vec()).unwrap();
    assert_eq!(encode(&value).unwrap(), bytes);
    assert_eq!(decode::<ByteVector<4>>(&bytes).unwrap().as_bytes(), bytes);
    assert_eq!(
        hash_tree_root(&value),
        hash_tree_root_byte_vector(&bytes, 4)
    );
    let mut root = [0; 32];
    root[..4].copy_from_slice(&bytes);
    assert_eq!(hash_tree_root(&value).unwrap(), root);
    let empty = decode::<ByteList<0>>(&[]).unwrap();
    assert_eq!(encode(&empty).unwrap(), []);
    assert_eq!(hash_tree_root(&empty), hash_tree_root_byte_list(&[], 0));
}

#[test]
fn metadata_and_huge_bounds_do_not_require_huge_allocations_or_formatting() {
    assert_eq!(ByteVector::<3>::TYPE, &TypeDescriptor::ByteVector(3));
    assert_eq!(ByteList::<3>::TYPE, &TypeDescriptor::ByteList(3));
    assert_eq!(fixed_size::<ByteVector<0>>(), Ok(Some(0)));
    assert_eq!(
        fixed_size::<ByteVector<{ usize::MAX }>>(),
        Ok(Some(usize::MAX))
    );
    assert_eq!(fixed_size::<ByteList<{ usize::MAX }>>(), Ok(None));
    assert_eq!(
        validate_type::<ByteVector<0>>(),
        Err(CodecError::EmptyVector)
    );
    assert_eq!(validate_type::<ByteVector<{ usize::MAX }>>(), Ok(()));
    assert_eq!(validate_type::<ByteList<0>>(), Ok(()));
    assert!(!is_basic::<ByteVector<1>>());
    assert!(!is_basic::<ByteList<1>>());
    assert!(is_compatible::<ByteVector<3>, Vector<u8, 3>>());
    assert!(is_compatible::<Vector<u8, 3>, ByteVector<3>>());
    assert!(is_compatible::<ByteList<3>, List<u8, 3>>());
    assert!(is_compatible::<List<u8, 3>, ByteList<3>>());
    assert!(is_compatible::<ByteVector<0>, Vector<u8, 0>>());
    assert!(is_compatible::<ByteList<0>, ByteList<0>>());
    assert!(is_compatible::<ByteVector<3>, ByteVector<3>>());
    assert!(!is_compatible::<ByteVector<3>, ByteVector<4>>());
    assert!(!is_compatible::<ByteList<3>, ByteList<4>>());
    assert!(!is_compatible::<ByteVector<3>, ByteList<3>>());
    assert!(!is_compatible::<ByteList<3>, ByteVector<3>>());
    assert!(!is_compatible::<ByteVector<3>, Vector<u16, 3>>());
    assert!(!is_compatible::<Vector<u16, 3>, ByteVector<3>>());
    assert!(!is_compatible::<ByteList<3>, List<u16, 3>>());
    assert!(!is_compatible::<List<u16, 3>, ByteList<3>>());
    assert!(!is_compatible::<ByteVector<3>, u8>());
    assert!(!is_compatible::<ByteList<3>, ProgressiveList<u8>>());
    let huge = ByteList::<{ usize::MAX }>::from_bytes(vec![255]).unwrap();
    assert!(format!("{huge:?}").len() < 80);
    assert_eq!(encode(&huge), Ok(vec![255]));
    assert_eq!(
        hash_tree_root(&huge),
        hash_tree_root_byte_list(&[255], usize::MAX)
    );
    assert_eq!(
        ByteVector::<{ usize::MAX }>::from_bytes(vec![]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(
        decode::<ByteVector<{ usize::MAX }>>(&[]).unwrap_err(),
        CodecError::InvalidLength
    );
}

#[test]
fn nested_collections_use_byte_descriptors_as_composite_children() {
    let fixed = vector::<_, 2>(vec![
        ByteVector::<2>::from_bytes(vec![1, 2]).unwrap(),
        ByteVector::<2>::from_bytes(vec![3, 4]).unwrap(),
    ])
    .unwrap();
    assert_eq!(encode(&fixed).unwrap(), [1, 2, 3, 4]);
    assert_eq!(fixed_size::<Vector<ByteVector<2>, 2>>(), Ok(Some(4)));
    let decoded = decode::<Vector<ByteVector<2>, 2>>(&[1, 2, 3, 4])
        .unwrap_or_else(|_| panic!("fixed decode"));
    assert_eq!(decoded.as_slice()[1].as_bytes(), [3, 4]);
    assert_eq!(hash_tree_root(&fixed), hash_tree_root(&decoded));
    let variable = list::<_, 2>(vec![
        ByteList::<3>::from_bytes(vec![1, 2]).unwrap(),
        ByteList::<3>::from_bytes(vec![]).unwrap(),
    ])
    .unwrap();
    let wire = [8, 0, 0, 0, 10, 0, 0, 0, 1, 2];
    assert_eq!(encode(&variable).unwrap(), wire);
    let decoded =
        decode::<List<ByteList<3>, 2>>(&wire).unwrap_or_else(|_| panic!("variable decode"));
    assert_eq!(decoded.as_slice()[0].as_bytes(), [1, 2]);
    assert_eq!(decoded.as_slice()[1].as_bytes(), []);
    assert_eq!(hash_tree_root(&variable), hash_tree_root(&decoded));
    assert_eq!(
        validate_type::<List<ByteVector<0>, 0>>(),
        Err(CodecError::EmptyVector)
    );
    assert!(decode::<List<ByteVector<0>, 0>>(&[]).is_err());
    assert!(decode::<List<ByteList<1>, 2>>(&[4, 0, 0, 0, 1, 2]).is_err());
    assert!(decode::<List<ByteList<3>, 2>>(&[3, 0, 0, 0]).is_err());
}
