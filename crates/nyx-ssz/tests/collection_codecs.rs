use nyx_ssz::{
    codec::{decode, encode, fixed_size, hash_tree_root, is_basic, validate_type},
    collections::{list, vector, List, Vector},
    error::CodecError,
    merkle::merkleize,
    sha256::hash_pair,
    tree_hash::hash_tree_root_u128,
};

#[test]
fn should_round_trip_fixed_vectors_when_elements_use_basic_codecs() {
    let value = vector::<_, 2>(vec![0x1234_u16, 0x5678]).unwrap();
    let bytes = [0x34, 0x12, 0x78, 0x56];
    assert_eq!(encode(&value), Ok(bytes.to_vec()));
    assert_eq!(
        decode::<Vector<u16, 2>>(&bytes).unwrap().as_slice(),
        value.as_slice()
    );
    let mut root = [0; 32];
    root[..4].copy_from_slice(&bytes);
    assert_eq!(hash_tree_root(&value), Ok(root));
    assert_eq!(fixed_size::<Vector<u16, 2>>(), Ok(Some(4)));
    assert!(!is_basic::<Vector<u16, 2>>());
}

#[test]
fn should_round_trip_lists_and_mix_in_count_when_capacity_spans_multiple_chunks() {
    let value = list::<_, 64>(vec![1_u8, 2, 3]).unwrap();
    assert_eq!(encode(&value), Ok(vec![1, 2, 3]));
    assert_eq!(
        decode::<List<u8, 64>>(&[1, 2, 3]).unwrap().as_slice(),
        &[1, 2, 3]
    );
    assert_eq!(fixed_size::<List<u8, 64>>(), Ok(None));
    let mut chunk = [0; 32];
    chunk[..3].copy_from_slice(&[1, 2, 3]);
    let contents = hash_pair(&chunk, &[0; 32]);
    assert_eq!(
        hash_tree_root(&value),
        Ok(hash_pair(&contents, &hash_tree_root_u128(3)))
    );
}

#[test]
fn should_merkleize_child_roots_when_vectors_contain_composite_elements() {
    let left = vector::<_, 2>(vec![1_u16, 2]).unwrap();
    let right = vector::<_, 2>(vec![3_u16, 4]).unwrap();
    let expected = hash_pair(
        &hash_tree_root(&left).unwrap(),
        &hash_tree_root(&right).unwrap(),
    );
    let value = vector::<_, 2>(vec![left, right]).unwrap();
    let bytes = [1, 0, 2, 0, 3, 0, 4, 0];
    assert_eq!(encode(&value), Ok(bytes.to_vec()));
    assert_eq!(hash_tree_root(&value), Ok(expected));
    let decoded = decode::<Vector<Vector<u16, 2>, 2>>(&bytes).unwrap();
    assert_eq!(decoded.as_slice()[1].as_slice(), &[3, 4]);
}

#[test]
fn should_build_offsets_when_a_vector_contains_variable_elements() {
    let value = vector::<_, 2>(vec![
        list::<_, 4>(vec![1_u8, 2]).unwrap(),
        list::<_, 4>(vec![3_u8]).unwrap(),
    ])
    .unwrap();
    let bytes = [8, 0, 0, 0, 10, 0, 0, 0, 1, 2, 3];
    assert_eq!(encode(&value), Ok(bytes.to_vec()));
    assert_eq!(fixed_size::<Vector<List<u8, 4>, 2>>(), Ok(None));
    let decoded = decode::<Vector<List<u8, 4>, 2>>(&bytes).unwrap();
    assert_eq!(decoded.as_slice()[0].as_slice(), &[1, 2]);
    assert_eq!(decoded.as_slice()[1].as_slice(), &[3]);
    assert_eq!(hash_tree_root(&decoded), hash_tree_root(&value));
}

#[test]
fn should_build_offsets_when_a_list_contains_variable_elements() {
    let value = list::<_, 4>(vec![
        list::<_, 3>(vec![false, true]).unwrap(),
        list::<bool, 3>(vec![]).unwrap(),
    ])
    .unwrap();
    let bytes = [8, 0, 0, 0, 10, 0, 0, 0, 0, 1];
    assert_eq!(encode(&value), Ok(bytes.to_vec()));
    let decoded = decode::<List<List<bool, 3>, 4>>(&bytes).unwrap();
    assert_eq!(decoded.as_slice()[0].as_slice(), &[false, true]);
    assert!(decoded.as_slice()[1].as_slice().is_empty());
    assert_eq!(hash_tree_root(&decoded), hash_tree_root(&value));
}

#[test]
fn should_return_nested_child_errors_when_a_later_body_is_invalid() {
    let bytes = [8, 0, 0, 0, 9, 0, 0, 0, 0, 0, 2];
    assert_eq!(
        decode::<Vector<List<bool, 4>, 2>>(&bytes).unwrap_err(),
        CodecError::InvalidBoolean
    );
}

#[test]
fn should_reject_invalid_layout_before_decoding_elements() {
    assert_eq!(
        decode::<Vector<u16, 2>>(&[0, 0]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(
        decode::<List<u16, 4>>(&[0]).unwrap_err(),
        CodecError::ScopeUndivided
    );
    assert_eq!(
        decode::<List<bool, 1>>(&[2, 2]).unwrap_err(),
        CodecError::OverLimit
    );
}

#[test]
fn should_validate_nested_declarations_when_values_or_capacities_are_empty() {
    assert_eq!(
        validate_type::<Vector<bool, 0>>(),
        Err(CodecError::EmptyVector)
    );
    assert_eq!(
        decode::<Vector<bool, 0>>(&[]).unwrap_err(),
        CodecError::EmptyVector
    );
    let parent = list::<Vector<bool, 0>, 0>(vec![]).unwrap();
    assert_eq!(encode(&parent), Err(CodecError::EmptyVector));
    assert_eq!(hash_tree_root(&parent), Err(CodecError::EmptyVector));
    assert_eq!(
        decode::<List<Vector<bool, 0>, 0>>(&[]).unwrap_err(),
        CodecError::EmptyVector
    );
}

#[test]
fn should_accept_empty_lists_when_a_valid_fixed_child_width_is_unrepresentable() {
    type HugeChild = Vector<u16, { usize::MAX }>;
    let value = list::<HugeChild, 0>(vec![]).unwrap();
    assert_eq!(fixed_size::<HugeChild>(), Err(CodecError::SizeOverflow));
    assert_eq!(
        decode::<Vector<HugeChild, 1>>(&[]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(encode(&value), Ok(vec![]));
    assert!(decode::<List<HugeChild, 0>>(&[])
        .unwrap()
        .as_slice()
        .is_empty());
    assert_eq!(
        decode::<List<HugeChild, 1>>(&[0]).unwrap_err(),
        CodecError::ScopeUndivided
    );
    assert_eq!(
        hash_tree_root(&value),
        Ok(hash_pair(&[0; 32], &hash_tree_root_u128(0)))
    );
}

#[test]
fn should_hash_empty_lists_without_allocating_the_declared_padding() {
    let value = list::<u8, { usize::MAX }>(vec![]).unwrap();
    let capacity = usize::MAX.div_ceil(32);
    let contents = merkleize(&[], Some(capacity)).unwrap();
    assert_eq!(
        hash_tree_root(&value),
        Ok(hash_pair(&contents, &hash_tree_root_u128(0)))
    );
}
