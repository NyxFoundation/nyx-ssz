//! Scripted dictionaries test runtime ordering, not certified conformance.
use std::{cell::RefCell, rc::Rc};

use nyx_ssz::{
    codec::{encode, Ssz},
    collections::progressive_list,
    collections::{list, vector},
    error::CodecError,
    type_info::TypeDescriptor,
};

type Trace = Rc<RefCell<Vec<usize>>>;

#[derive(Debug)]
struct Script<const VARIABLE: bool> {
    result: Result<Vec<u8>, CodecError>,
    index: usize,
    trace: Trace,
}

impl<const VARIABLE: bool> Ssz for Script<VARIABLE> {
    const TYPE: &'static TypeDescriptor = if VARIABLE {
        &TypeDescriptor::List(&TypeDescriptor::Uint(1), 8)
    } else {
        &TypeDescriptor::Uint(1)
    };

    fn encoded_len(&self) -> Result<usize, CodecError> {
        self.trace.borrow_mut().push(self.index);
        self.result.as_ref().map(Vec::len).map_err(|error| *error)
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        match &self.result {
            Ok(bytes) => nyx_ssz::encoding::copy(output, bytes),
            Err(error) => Err(*error),
        }
    }

    fn decode(_: &[u8]) -> Result<Self, CodecError> {
        Err(CodecError::InvalidLength)
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Err(CodecError::InvalidLength)
    }
}

fn check<const VARIABLE: bool>(
    results: &[Result<Vec<u8>, CodecError>],
    expected: Result<Vec<u8>, CodecError>,
    visited: &[usize],
) {
    for family in 0..3 {
        let trace = Trace::default();
        let values = results
            .iter()
            .enumerate()
            .map(|(index, result)| Script::<VARIABLE> {
                result: result.clone(),
                index,
                trace: trace.clone(),
            })
            .collect();
        let actual = match family {
            0 => encode(&vector::<_, 3>(values).unwrap()),
            1 => encode(&list::<_, 3>(values).unwrap()),
            _ => encode(&progressive_list(values)),
        };
        assert_eq!(actual, expected);
        let trace = trace.borrow();
        if actual.is_err() {
            assert_eq!(*trace, visited);
        } else {
            assert!(!trace.is_empty());
            assert!(trace.chunks(visited.len()).all(|pass| pass == visited));
        }
    }
}

#[test]
fn deferred_native_success_does_not_hide_later_protocol_faults() {
    use CodecError::*;
    for fault in [InvalidBoolean, SizeOverflow, OverLimit, InvalidLength] {
        let results = [Err(NativeOutputOverflow), Err(fault), Err(EmptyEncoding)];
        check::<false>(&results, Err(fault), &[0, 1]);
        check::<true>(&results, Err(fault), &[0, 1]);
    }
}

#[test]
fn genuine_protocol_overflow_stops_before_later_faults() {
    use CodecError::*;
    let results = [
        Err(SizeOverflow),
        Err(InvalidBoolean),
        Err(NativeOutputOverflow),
    ];
    check::<false>(&results, Err(SizeOverflow), &[0]);
    check::<true>(&results, Err(SizeOverflow), &[0]);
}

#[test]
fn all_reference_success_with_native_marker_gets_parent_protocol_budget_error() {
    use CodecError::*;
    let results = [Ok(vec![1]), Err(NativeOutputOverflow), Ok(vec![2])];
    check::<false>(&results, Err(SizeOverflow), &[0, 1, 2]);
    check::<true>(&results, Err(SizeOverflow), &[0, 1, 2]);
    let repeated = [
        Err(NativeOutputOverflow),
        Err(NativeOutputOverflow),
        Ok(vec![]),
    ];
    check::<true>(&repeated, Err(SizeOverflow), &[0, 1, 2]);
}

fn basic_root(results: &[Result<Vec<u8>, CodecError>]) -> Result<[u8; 32], CodecError> {
    let values = results
        .iter()
        .enumerate()
        .map(|(index, result)| Script::<false> {
            result: result.clone(),
            index,
            trace: Trace::default(),
        })
        .collect();
    list::<_, 128>(values).unwrap().hash_tree_root()
}

#[test]
fn should_validate_every_basic_child_before_refusing_an_unrepresentable_root_encoding() {
    use CodecError::*;
    assert_eq!(
        basic_root(&[Err(NativeOutputOverflow), Err(InvalidBoolean)]),
        Err(InvalidBoolean)
    );
    assert_eq!(
        basic_root(&[Err(SizeOverflow), Err(InvalidBoolean)]),
        Err(SizeOverflow)
    );
    assert_eq!(
        basic_root(&[Ok(vec![1]), Err(NativeOutputOverflow)]),
        Err(InvalidLength)
    );
}

#[test]
fn should_pack_cross_chunk_and_empty_encodings_without_assuming_external_codec_widths() {
    use nyx_ssz::{merkle::merkleize, sha256::hash_pair, tree_hash::hash_tree_root_u128};
    let results = [Ok(vec![7; 33]), Ok(vec![]), Ok(vec![9; 17])];
    let mut second = [9; 32];
    second[0] = 7;
    second[18..].fill(0);
    let content = merkleize(&[[7; 32], second], Some(4)).unwrap();
    assert_eq!(
        basic_root(&results),
        Ok(hash_pair(&content, &hash_tree_root_u128(3)))
    );
}

#[test]
fn fully_representable_parts_retain_exact_order_and_offsets() {
    let results = [Ok(vec![1, 2]), Ok(vec![]), Ok(vec![3])];
    check::<false>(&results, Ok(vec![1, 2, 3]), &[0, 1, 2]);
    check::<true>(
        &results,
        Ok(vec![12, 0, 0, 0, 14, 0, 0, 0, 14, 0, 0, 0, 1, 2, 3]),
        &[0, 1, 2],
    );
    assert_ne!(CodecError::NativeOutputOverflow, CodecError::SizeOverflow);
    assert!(CodecError::NativeOutputOverflow
        .to_string()
        .contains("native"));
}
