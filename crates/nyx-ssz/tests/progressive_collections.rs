use nyx_ssz::{
    codec::{decode, encode, fixed_size, hash_tree_root, is_basic, validate_type, Ssz},
    collections::{list, progressive_list, vector, List, ProgressiveList, Vector},
    error::CodecError,
    merkle::merkleize,
    sha256::hash_pair,
    tree_hash::{hash_tree_root_progressive_byte_list, hash_tree_root_u128},
    type_info::TypeDescriptor,
};

#[test]
fn should_preserve_the_owned_allocation_when_constructing_a_progressive_list() {
    let values = vec![1_u16, 2, 3];
    let pointer = values.as_ptr();
    let value = progressive_list(values);
    assert_eq!(value.as_slice().as_ptr(), pointer);
    assert_eq!(value.as_slice(), &[1, 2, 3]);
    assert_eq!(fixed_size::<ProgressiveList<u16>>(), Ok(None));
    assert!(!is_basic::<ProgressiveList<u16>>());
    assert_eq!(validate_type::<ProgressiveList<u16>>(), Ok(()));
    assert_eq!(
        ProgressiveList::<u16>::TYPE,
        &TypeDescriptor::ProgressiveList(&TypeDescriptor::Uint(2), None)
    );
}

#[test]
fn should_preserve_noncodec_owners_and_machine_maximum_storage_when_no_bound_is_declared() {
    use std::{cell::Cell, rc::Rc};
    struct Owner(Rc<Cell<usize>>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0));
    {
        let value = progressive_list(vec![Owner(drops.clone()), Owner(drops.clone())]);
        assert_eq!(value.as_slice().len(), 2);
        assert_eq!(drops.get(), 0);
    }
    assert_eq!(drops.get(), 2);
    assert_eq!(
        progressive_list(vec![(); usize::MAX]).as_slice().len(),
        usize::MAX
    );
    assert_eq!(
        format!("{:?}", progressive_list(vec![1_u8, 2])),
        "ProgressiveList { values: [1, 2] }"
    );
}

#[test]
fn should_encode_decode_and_root_empty_values_when_the_child_type_is_valid() {
    let value = progressive_list::<u8>(vec![]);
    assert_eq!(encode(&value), Ok(vec![]));
    assert!(decode::<ProgressiveList<u8>>(&[])
        .unwrap()
        .as_slice()
        .is_empty());
    assert_eq!(hash_tree_root(&value), Ok(hash_pair(&[0; 32], &[0; 32])));
}

#[test]
fn should_mix_the_element_count_when_basic_elements_span_several_bytes() {
    let value = progressive_list(vec![0x0102_u16, 0x0304]);
    let bytes = [2, 1, 4, 3];
    assert_eq!(encode(&value), Ok(bytes.to_vec()));
    assert_eq!(
        decode::<ProgressiveList<u16>>(&bytes).unwrap().as_slice(),
        value.as_slice()
    );
    let mut chunk = [0; 32];
    chunk[..4].copy_from_slice(&bytes);
    let contents = hash_pair(&chunk, &[0; 32]);
    assert_eq!(
        hash_tree_root(&value),
        Ok(hash_pair(&contents, &hash_tree_root_u128(2)))
    );
}

#[test]
fn should_match_borrowed_roots_when_packed_bytes_cross_progressive_levels() {
    for count in [0, 1, 31, 32, 33, 159, 160, 161, 671, 672, 673] {
        let bytes: Vec<u8> = (0..count).map(|index| (index % 251) as u8).collect();
        let expected = hash_tree_root_progressive_byte_list(&bytes, None);
        let value = progressive_list(bytes);
        assert_eq!(hash_tree_root(&value), expected, "count={count}");
        let encoded = encode(&value).unwrap();
        assert_eq!(
            decode::<ProgressiveList<u8>>(&encoded).unwrap().as_slice(),
            value.as_slice()
        );
    }
}

#[test]
fn should_encode_offsets_and_root_children_when_elements_have_variable_layouts() {
    let first = list::<_, 3>(vec![1_u16, 2]).unwrap();
    let second = list::<_, 3>(vec![3_u16]).unwrap();
    let third = list::<u16, 3>(vec![]).unwrap();
    let left = hash_tree_root(&first).unwrap();
    let rest = merkleize(
        &[
            hash_tree_root(&second).unwrap(),
            hash_tree_root(&third).unwrap(),
        ],
        Some(4),
    )
    .unwrap();
    let contents = hash_pair(&left, &hash_pair(&rest, &[0; 32]));
    let value = progressive_list(vec![first, second, third]);
    let bytes = [12, 0, 0, 0, 16, 0, 0, 0, 18, 0, 0, 0, 1, 0, 2, 0, 3, 0];
    assert_eq!(encode(&value), Ok(bytes.to_vec()));
    assert_eq!(
        hash_tree_root(&value),
        Ok(hash_pair(&contents, &hash_tree_root_u128(3)))
    );
    let decoded = decode::<ProgressiveList<List<u16, 3>>>(&bytes).unwrap();
    assert_eq!(decoded.as_slice()[0].as_slice(), &[1, 2]);
    assert!(decoded.as_slice()[2].as_slice().is_empty());
    assert_eq!(hash_tree_root(&decoded), hash_tree_root(&value));
}

#[test]
fn should_nest_progressive_lists_when_each_child_has_its_own_offset_table() {
    let value = progressive_list(vec![
        progressive_list(vec![false, true]),
        progressive_list::<bool>(vec![]),
    ]);
    let bytes = [8, 0, 0, 0, 10, 0, 0, 0, 0, 1];
    assert_eq!(encode(&value), Ok(bytes.to_vec()));
    let decoded = decode::<ProgressiveList<ProgressiveList<bool>>>(&bytes).unwrap();
    assert_eq!(decoded.as_slice()[0].as_slice(), &[false, true]);
    assert!(decoded.as_slice()[1].as_slice().is_empty());
    assert_eq!(hash_tree_root(&decoded), hash_tree_root(&value));
}

#[test]
fn should_return_child_errors_when_a_later_element_is_invalid() {
    assert_eq!(
        decode::<ProgressiveList<Vector<bool, 2>>>(&[0, 1, 0, 2]).unwrap_err(),
        CodecError::InvalidBoolean
    );
}

#[test]
fn should_reject_invalid_types_even_when_the_progressive_value_is_empty() {
    let value = progressive_list::<Vector<u8, 0>>(vec![]);
    assert_eq!(encode(&value), Err(CodecError::EmptyVector));
    assert_eq!(hash_tree_root(&value), Err(CodecError::EmptyVector));
    assert_eq!(
        decode::<ProgressiveList<Vector<u8, 0>>>(&[]).unwrap_err(),
        CodecError::EmptyVector
    );
}

#[test]
fn should_reject_noncanonical_layouts_before_decoding_children() {
    for bytes in [&[5, 0, 0, 0][..], &[8, 0, 0, 0][..], &[0][..]] {
        assert!(decode::<ProgressiveList<List<u8, 2>>>(bytes).is_err());
    }
    assert_eq!(
        decode::<ProgressiveList<u16>>(&[1]).unwrap_err(),
        CodecError::ScopeUndivided
    );
}

#[test]
fn should_accept_empty_lists_when_only_the_fixed_child_width_exceeds_usize() {
    type Huge = Vector<Vector<u8, { usize::MAX }>, 2>;
    assert_eq!(fixed_size::<Huge>(), Err(CodecError::SizeOverflow));
    assert!(decode::<ProgressiveList<Huge>>(&[])
        .unwrap()
        .as_slice()
        .is_empty());
    assert_eq!(
        decode::<ProgressiveList<Huge>>(&[1]).unwrap_err(),
        CodecError::ScopeUndivided
    );
    assert_eq!(encode(&progressive_list::<Huge>(vec![])), Ok(vec![]));
}

#[test]
fn should_remain_composable_when_bounded_collections_contain_progressive_values() {
    let value = vector::<_, 2>(vec![
        progressive_list(vec![1_u8, 2]),
        progressive_list(vec![3_u8]),
    ])
    .unwrap();
    let bytes = [8, 0, 0, 0, 10, 0, 0, 0, 1, 2, 3];
    assert_eq!(encode(&value), Ok(bytes.to_vec()));
    let decoded = decode::<Vector<ProgressiveList<u8>, 2>>(&bytes).unwrap();
    assert_eq!(decoded.as_slice()[1].as_slice(), &[3]);
    assert_eq!(hash_tree_root(&decoded), hash_tree_root(&value));
}
