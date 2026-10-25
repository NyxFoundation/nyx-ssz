use nyx_ssz::{
    bitcollections::{BitList, BitVector, ProgressiveBitList},
    codec::{decode, encode, fixed_size, hash_tree_root, is_basic, is_compatible, Ssz},
    collections::{list, progressive_list, vector, List, ProgressiveList, Vector},
    error::CodecError,
    tree_hash::{
        hash_tree_root_bit_list, hash_tree_root_bit_vector, hash_tree_root_progressive_bit_list,
    },
    type_info::TypeDescriptor,
};

#[test]
fn should_keep_packed_allocations_when_owning_canonical_bytes() {
    let encoded = vec![5];
    let pointer = encoded.as_ptr();
    let value = BitVector::<3>::from_ssz_bytes(encoded).unwrap();
    assert_eq!(value.as_ssz_bytes().as_ptr(), pointer);
    assert_eq!(value.len_bits(), 3);
    assert_eq!(value.get(0), Some(true));
    assert_eq!(value.get(1), Some(false));
    assert_eq!(value.get(2), Some(true));
    assert_eq!(value.get(3), None);
    assert_eq!(value.get(u128::MAX), None);
    let encoded = vec![13];
    let pointer = encoded.as_ptr();
    let value = BitList::<8>::from_ssz_bytes(encoded).unwrap();
    assert_eq!(value.as_ssz_bytes().as_ptr(), pointer);
    assert_eq!(value.len_bits(), 3);
    assert_eq!(value.get(2), Some(true));
    assert_eq!(value.get(3), None);
}

fn check_vector<const N: usize>() {
    let bits: Vec<bool> = (0..N).map(|index| index % 3 == 1).collect();
    let value = BitVector::<N>::from_bits(&bits).unwrap();
    let bytes = encode(&value).unwrap();
    assert_eq!(bytes.len(), N.div_ceil(8));
    assert_eq!(hash_tree_root(&value), hash_tree_root_bit_vector(&bits, N));
    let decoded = decode::<BitVector<N>>(&bytes).unwrap();
    assert_eq!(decoded.as_ssz_bytes(), value.as_ssz_bytes());
    assert_eq!(decoded.len_bits(), N as u128);
    for (index, expected) in bits.iter().enumerate() {
        assert_eq!(decoded.get(index as u128), Some(*expected));
    }
}

#[test]
fn should_round_trip_vector_bits_and_roots_when_padding_or_chunks_are_partial() {
    check_vector::<1>();
    check_vector::<7>();
    check_vector::<8>();
    check_vector::<9>();
    check_vector::<255>();
    check_vector::<256>();
    check_vector::<257>();
}

#[test]
fn should_round_trip_bounded_and_progressive_bits_without_counting_the_delimiter() {
    for count in [0, 1, 7, 8, 9, 255, 256, 257, 1365, 1366] {
        let bits: Vec<bool> = (0..count).map(|index| index % 5 == 2).collect();
        let bounded = BitList::<1366>::from_bits(&bits).unwrap();
        let progressive = ProgressiveBitList::from_bits(&bits).unwrap();
        let bytes = encode(&bounded).unwrap();
        assert_eq!(bytes.len(), count / 8 + 1);
        assert_eq!(encode(&progressive), Ok(bytes.clone()));
        assert_eq!(bounded.len_bits(), count as u128);
        assert_eq!(progressive.len_bits(), count as u128);
        assert_eq!(bounded.get(count as u128), None);
        assert_eq!(progressive.get(u128::MAX), None);
        assert_eq!(
            hash_tree_root(&bounded),
            hash_tree_root_bit_list(&bits, 1366)
        );
        assert_eq!(
            hash_tree_root(&progressive),
            hash_tree_root_progressive_bit_list(&bits, None)
        );
        assert_eq!(
            decode::<BitList<1366>>(&bytes).unwrap().as_ssz_bytes(),
            bytes
        );
        assert_eq!(
            decode::<ProgressiveBitList>(&bytes).unwrap().as_ssz_bytes(),
            bytes
        );
        for (index, expected) in bits.iter().enumerate() {
            assert_eq!(progressive.get(index as u128), Some(*expected));
        }
    }
}

#[test]
fn should_validate_type_and_padding_before_accepting_a_vector_encoding() {
    assert_eq!(
        BitVector::<0>::from_ssz_bytes(vec![]).unwrap_err(),
        CodecError::EmptyVector
    );
    assert_eq!(
        BitVector::<3>::from_bits(&[true]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(
        BitVector::<3>::from_ssz_bytes(vec![]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(
        BitVector::<3>::from_ssz_bytes(vec![0x85]).unwrap_err(),
        CodecError::PaddingBits
    );
    assert_eq!(
        decode::<BitVector<3>>(&[0x85]).unwrap_err(),
        CodecError::PaddingBits
    );
}

#[test]
fn should_preserve_delimiter_fault_categories_when_list_encodings_are_invalid() {
    for (bytes, expected) in [
        (vec![], CodecError::EmptyEncoding),
        (vec![0], CodecError::NoDelimiter),
        (vec![1, 0], CodecError::TrailingZeros),
    ] {
        assert_eq!(
            BitList::<8>::from_ssz_bytes(bytes.clone()).unwrap_err(),
            expected
        );
        assert_eq!(
            ProgressiveBitList::from_ssz_bytes(bytes.clone()).unwrap_err(),
            expected
        );
        assert_eq!(decode::<ProgressiveBitList>(&bytes).unwrap_err(), expected);
    }
    assert_eq!(
        BitList::<2>::from_ssz_bytes(vec![13]).unwrap_err(),
        CodecError::OverLimit
    );
    assert_eq!(
        BitList::<0>::from_bits(&[false]).unwrap_err(),
        CodecError::OverLimit
    );
    assert_eq!(BitList::<0>::from_bits(&[]).unwrap().as_ssz_bytes(), &[1]);
}

#[test]
fn should_derive_full_type_metadata_when_bit_types_are_nested() {
    assert_eq!(fixed_size::<BitVector<9>>(), Ok(Some(2)));
    assert_eq!(fixed_size::<Vector<BitVector<9>, 2>>(), Ok(Some(4)));
    assert_eq!(fixed_size::<BitList<8>>(), Ok(None));
    assert_eq!(fixed_size::<ProgressiveBitList>(), Ok(None));
    assert!(!is_basic::<BitVector<8>>());
    assert!(!is_compatible::<BitVector<8>, u8>());
    assert!(!is_compatible::<BitList<8>, ProgressiveBitList>());
    assert_eq!(BitVector::<9>::TYPE, &TypeDescriptor::BitVector(9));
    assert_eq!(BitList::<8>::TYPE, &TypeDescriptor::BitList(8));
    assert_eq!(
        ProgressiveBitList::TYPE,
        &TypeDescriptor::ProgressiveBitList(None)
    );
    assert_eq!(
        fixed_size::<BitVector<{ usize::MAX }>>(),
        Ok(Some(usize::MAX.div_ceil(8)))
    );
}

#[test]
fn should_compose_with_generic_collections_when_bits_have_fixed_or_variable_layouts() {
    let value = vector::<_, 2>(vec![
        BitVector::<3>::from_bits(&[true, false, true]).unwrap(),
        BitVector::<3>::from_bits(&[false, true, false]).unwrap(),
    ])
    .unwrap();
    assert_eq!(encode(&value), Ok(vec![5, 2]));
    let decoded = decode::<Vector<BitVector<3>, 2>>(&[5, 2]).unwrap();
    assert_eq!(hash_tree_root(&decoded), hash_tree_root(&value));
    let bits = BitList::<8>::from_bits(&[true, false, true]).unwrap();
    let bounded = list::<_, 2>(vec![bits]).unwrap();
    let bytes = encode(&bounded).unwrap();
    let decoded = decode::<List<BitList<8>, 2>>(&bytes).unwrap();
    assert_eq!(hash_tree_root(&decoded), hash_tree_root(&bounded));
    let value = progressive_list(vec![ProgressiveBitList::from_bits(&[true]).unwrap()]);
    let bytes = encode(&value).unwrap();
    let decoded = decode::<ProgressiveList<ProgressiveBitList>>(&bytes).unwrap();
    assert_eq!(hash_tree_root(&decoded), hash_tree_root(&value));
}

#[test]
fn should_read_large_packed_values_without_expanding_them_or_narrowing_the_count_type() {
    let encoded = vec![255; 65537];
    let pointer = encoded.as_ptr();
    let value = ProgressiveBitList::from_ssz_bytes(encoded).unwrap();
    assert_eq!(value.as_ssz_bytes().as_ptr(), pointer);
    assert_eq!(value.as_ssz_bytes().len(), 65537);
    assert_eq!(value.len_bits(), 65536_u128 * 8 + 7);
    assert_eq!(value.get(65536_u128 * 8 + 6), Some(true));
    assert_eq!(value.get(65536_u128 * 8 + 7), None);
    assert!(hash_tree_root(&value).is_ok());
}
