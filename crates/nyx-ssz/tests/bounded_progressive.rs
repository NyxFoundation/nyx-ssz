use nyx_ssz::{
    bitcollections::{BoundedProgressiveBitList, ProgressiveBitList},
    codec::{decode, fixed_size, is_compatible, validate_type, Ssz},
    collections::{
        bounded_progressive_list, list, progressive_list, vector, BoundedProgressiveList,
        LengthErrorKind, List, Vector,
    },
    error::CodecError,
    type_info::TypeDescriptor,
};

#[test]
fn bounded_storage_preserves_allocation_and_owners_on_both_outcomes() {
    use std::{cell::Cell, rc::Rc};
    #[derive(Debug)]
    struct Owner(Rc<Cell<usize>>);
    impl Drop for Owner {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let drops = Rc::new(Cell::new(0));
    let values = vec![Owner(drops.clone()), Owner(drops.clone())];
    let pointer = values.as_ptr();
    let error = bounded_progressive_list::<_, 1>(values).unwrap_err();
    assert_eq!(error.kind, LengthErrorKind::OverLimit);
    assert_eq!(error.values.as_ptr(), pointer);
    assert_eq!(drops.get(), 0);
    let value = bounded_progressive_list::<_, 2>(error.values).unwrap();
    assert_eq!(value.as_slice().as_ptr(), pointer);
    assert_eq!(drops.get(), 0);
    drop(value);
    assert_eq!(drops.get(), 2);
    assert!(bounded_progressive_list::<u8, 0>(vec![]).is_ok());
    assert_eq!(
        bounded_progressive_list::<_, { usize::MAX }>(vec![(); usize::MAX])
            .unwrap()
            .as_slice()
            .len(),
        usize::MAX
    );
    assert_eq!(
        format!(
            "{:?}",
            bounded_progressive_list::<_, { usize::MAX }>(vec![7]).unwrap()
        ),
        "BoundedProgressiveList { values: [7] }"
    );
}

#[test]
fn declarations_retain_real_zero_and_machine_maximum_bounds() {
    assert_eq!(
        BoundedProgressiveList::<u16, { usize::MAX }>::TYPE,
        &TypeDescriptor::ProgressiveList(&TypeDescriptor::Uint(2), Some(usize::MAX))
    );
    assert_eq!(
        BoundedProgressiveBitList::<{ usize::MAX }>::TYPE,
        &TypeDescriptor::ProgressiveBitList(Some(usize::MAX))
    );
    assert_eq!(fixed_size::<BoundedProgressiveList<u16, 0>>(), Ok(None));
    assert_eq!(validate_type::<BoundedProgressiveList<u16, 0>>(), Ok(()));
    assert!(is_compatible::<
        BoundedProgressiveBitList<0>,
        ProgressiveBitList,
    >());
    assert!(is_compatible::<
        BoundedProgressiveList<u16, 0>,
        BoundedProgressiveList<u16, { usize::MAX }>,
    >());
}

#[test]
fn bounded_fixed_values_match_accepted_unbounded_progressive_values() {
    for count in [0, 1, 31, 32, 33, 159, 160, 161, 673] {
        let values: Vec<u16> = (0..count).map(|i| i as u16).collect();
        let reference = progressive_list(values.clone());
        let bounded = bounded_progressive_list::<_, 673>(values).unwrap();
        let encoded = bounded.encode().unwrap();
        assert_eq!(reference.encode(), Ok(encoded.clone()));
        assert_eq!(bounded.hash_tree_root(), reference.hash_tree_root());
        let decoded = decode::<BoundedProgressiveList<u16, 673>>(&encoded).unwrap();
        assert_eq!(decoded.as_slice(), reference.as_slice());
    }
    assert!(decode::<BoundedProgressiveList<u8, 0>>(&[]).is_ok());
    assert_eq!(
        decode::<BoundedProgressiveList<bool, 0>>(&[2]).unwrap_err(),
        CodecError::OverLimit
    );
    assert_eq!(
        decode::<BoundedProgressiveList<bool, 1>>(&[2]).unwrap_err(),
        CodecError::InvalidBoolean
    );
}

#[test]
fn bounded_nested_fixed_and_variable_children_preserve_wire_and_roots() {
    let fixed =
        bounded_progressive_list::<_, 1>(vec![vector::<_, 2>(vec![1_u16, 2]).unwrap()]).unwrap();
    let fixed_bytes = fixed.encode().unwrap();
    let decoded = decode::<BoundedProgressiveList<Vector<u16, 2>, 1>>(&fixed_bytes).unwrap();
    assert_eq!(decoded.as_slice()[0].as_slice(), &[1, 2]);
    assert_eq!(decoded.hash_tree_root(), fixed.hash_tree_root());
    let children = vec![
        list::<_, 3>(vec![1_u16, 2]).unwrap(),
        list::<_, 3>(vec![]).unwrap(),
    ];
    let reference = progressive_list(children);
    let bytes = reference.encode().unwrap();
    let bounded = decode::<BoundedProgressiveList<List<u16, 3>, 2>>(&bytes).unwrap();
    assert_eq!(bounded.encode(), Ok(bytes));
    assert_eq!(bounded.hash_tree_root(), reference.hash_tree_root());
    let nested = bounded_progressive_list::<_, 1>(vec![bounded]).unwrap();
    let bytes = nested.encode().unwrap();
    let decoded =
        decode::<BoundedProgressiveList<BoundedProgressiveList<List<u16, 3>, 2>, 1>>(&bytes)
            .unwrap();
    assert_eq!(decoded.hash_tree_root(), nested.hash_tree_root());
}

#[test]
fn bounded_packed_dictionaries_compose_as_variable_children_at_machine_maximum() {
    type Bits = BoundedProgressiveBitList<{ usize::MAX }>;
    type Outer = BoundedProgressiveList<Bits, { usize::MAX }>;
    let value = bounded_progressive_list::<_, { usize::MAX }>(vec![
        Bits::from_ssz_bytes(vec![13]).unwrap(),
        Bits::from_ssz_bytes(vec![1]).unwrap(),
    ])
    .unwrap();
    let bytes = [8, 0, 0, 0, 9, 0, 0, 0, 13, 1];
    assert_eq!(value.encode(), Ok(bytes.to_vec()));
    let decoded = decode::<Outer>(&bytes).unwrap();
    assert_eq!(decoded.as_slice()[0].len_bits(), 3);
    assert_eq!(decoded.as_slice()[1].len_bits(), 0);
    assert_eq!(decoded.hash_tree_root(), value.hash_tree_root());
    assert_eq!(
        decode::<BoundedProgressiveList<u8, { usize::MAX }>>(&[7])
            .unwrap()
            .as_slice(),
        &[7]
    );
}

#[test]
fn layout_and_capacity_faults_precede_callbacks() {
    // Two variable children; the second offset is unordered and the first body is invalid.
    let bytes = [8, 0, 0, 0, 4, 0, 0, 0, 2];
    assert_eq!(
        decode::<BoundedProgressiveList<List<bool, 2>, 1>>(&bytes).unwrap_err(),
        CodecError::OverLimit
    );
    assert_eq!(
        decode::<BoundedProgressiveList<List<bool, 2>, 2>>(&bytes).unwrap_err(),
        CodecError::OffsetUnordered
    );
    assert_eq!(
        decode::<BoundedProgressiveList<u16, 0>>(&[1]).unwrap_err(),
        CodecError::ScopeUndivided
    );
    assert_eq!(
        decode::<BoundedProgressiveList<Vector<bool, 2>, 2>>(&[0, 1, 0, 2]).unwrap_err(),
        CodecError::InvalidBoolean
    );
}

#[test]
fn invalid_child_declarations_precede_bounds_and_overflowing_widths_allow_empty_values() {
    let empty = bounded_progressive_list::<Vector<u8, 0>, 0>(vec![]).unwrap();
    assert_eq!(empty.encode(), Err(CodecError::EmptyVector));
    assert_eq!(empty.hash_tree_root(), Err(CodecError::EmptyVector));
    assert_eq!(
        decode::<BoundedProgressiveList<Vector<u8, 0>, 0>>(&[1]).unwrap_err(),
        CodecError::EmptyVector
    );
    type Huge = Vector<Vector<u8, { usize::MAX }>, 2>;
    assert!(decode::<BoundedProgressiveList<Huge, 0>>(&[]).is_ok());
    assert_eq!(
        decode::<BoundedProgressiveList<Huge, 0>>(&[1]).unwrap_err(),
        CodecError::ScopeUndivided
    );
}

#[test]
fn bounded_packed_bits_preserve_allocation_delimiters_and_wide_index_boundaries() {
    let bytes = vec![0xad, 0b1011]; // Eleven logical bits.
    let pointer = bytes.as_ptr();
    let bits = BoundedProgressiveBitList::<11>::from_ssz_bytes(bytes).unwrap();
    assert_eq!(bits.as_ssz_bytes().as_ptr(), pointer);
    assert_eq!(bits.len_bits(), 11_u128);
    for index in 0..11 {
        let expected = (bits.as_ssz_bytes()[(index / 8) as usize] >> (index % 8)) & 1 == 1;
        assert_eq!(bits.get(index), Some(expected));
    }
    for index in [
        11,
        12,
        usize::MAX as u128,
        usize::MAX as u128 + 1,
        u128::MAX,
    ] {
        assert_eq!(bits.get(index), None);
    }
    let reference = ProgressiveBitList::from_ssz_bytes(vec![0xad, 0b1011]).unwrap();
    assert_eq!(bits.encode(), reference.encode());
    assert_eq!(bits.hash_tree_root(), reference.hash_tree_root());
    let decoded = decode::<BoundedProgressiveBitList<11>>(bits.as_ssz_bytes()).unwrap();
    assert_eq!(decoded.as_ssz_bytes(), bits.as_ssz_bytes());
    assert_eq!(decoded.hash_tree_root(), bits.hash_tree_root());
    assert_eq!(
        format!(
            "{:?}",
            BoundedProgressiveBitList::<{ usize::MAX }>::from_ssz_bytes(vec![1]).unwrap()
        ),
        "BoundedProgressiveBitList { encoded: [1] }"
    );
}

#[test]
fn packed_capacity_checks_preserve_canonical_error_order_at_zero_one_and_maximum() {
    let empty = BoundedProgressiveBitList::<0>::from_ssz_bytes(vec![1]).unwrap();
    assert_eq!(empty.len_bits(), 0);
    assert_eq!(empty.get(0), None);
    assert_eq!(empty.encode(), Ok(vec![1]));
    assert!(BoundedProgressiveBitList::<1>::from_ssz_bytes(vec![3]).is_ok());
    assert!(BoundedProgressiveBitList::<{ usize::MAX }>::from_ssz_bytes(vec![255, 1]).is_ok());
    for (bytes, expected) in [
        (vec![], CodecError::EmptyEncoding),
        (vec![0], CodecError::NoDelimiter),
        (vec![1, 0], CodecError::TrailingZeros),
        (vec![2], CodecError::OverLimit),
    ] {
        assert_eq!(
            BoundedProgressiveBitList::<0>::from_ssz_bytes(bytes.clone()).unwrap_err(),
            expected
        );
        assert_eq!(
            decode::<BoundedProgressiveBitList<0>>(&bytes).unwrap_err(),
            expected
        );
    }
}
