use nyx_ssz::{
    codec::{decode, decode_elements, Ssz},
    collections::{BoundedProgressiveList, List, ProgressiveList, Vector},
    error::CodecError,
    sequence::variable_vector_view,
    type_info::TypeDescriptor,
};
use std::cell::RefCell;

thread_local! {
    static DROPS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static CALLS: RefCell<usize> = const { RefCell::new(0) };
}

#[derive(Debug)]
struct TracedByte(u8);

impl Drop for TracedByte {
    fn drop(&mut self) {
        DROPS.with(|log| log.borrow_mut().push(self.0));
    }
}

impl Ssz for TracedByte {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Uint(1);

    fn encoded_len(&self) -> Result<usize, CodecError> {
        Ok(1)
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        nyx_ssz::encoding::copy(output, &[self.0])
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Ok(nyx_ssz::tree_hash::hash_tree_root_u8(self.0))
    }
    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        CALLS.with(|count| *count.borrow_mut() += 1);
        if bytes.len() != 1 || bytes[0] == 255 {
            return Err(CodecError::InvalidLength);
        }
        Ok(Self(bytes[0]))
    }
}

fn reset() {
    DROPS.with(|log| log.borrow_mut().clear());
    CALLS.with(|count| *count.borrow_mut() = 0);
}

fn drops() -> Vec<u8> {
    DROPS.with(|log| log.borrow().clone())
}
fn calls() -> usize {
    CALLS.with(|count| *count.borrow())
}

#[test]
fn should_decode_elements_in_order_when_each_body_is_valid() {
    reset();
    let bytes = [12, 0, 0, 0, 13, 0, 0, 0, 14, 0, 0, 0, 7, 8, 9];
    let view = variable_vector_view(&bytes, 3).unwrap();
    let values = decode_elements::<TracedByte>(&view).unwrap();
    assert_eq!(
        values.iter().map(|value| value.0).collect::<Vec<_>>(),
        [7, 8, 9]
    );
    assert_eq!(calls(), 3);
    assert!(drops().is_empty());
    drop(values);
    assert_eq!(drops(), [7, 8, 9]);
}

#[test]
fn should_drop_completed_elements_when_a_later_body_is_invalid() {
    reset();
    let bytes = [8, 0, 0, 0, 9, 0, 0, 0, 7];
    let view = variable_vector_view(&bytes, 2).unwrap();
    assert_eq!(
        decode_elements::<TracedByte>(&view).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(calls(), 2);
    assert_eq!(drops(), [7]);
}

#[test]
fn should_stop_at_the_first_failure() {
    reset();
    let bytes = [8, 0, 0, 0, 8, 0, 0, 0, 7];
    let view = variable_vector_view(&bytes, 2).unwrap();
    assert_eq!(
        decode_elements::<TracedByte>(&view).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(calls(), 1);
    assert!(drops().is_empty());
}

#[test]
fn should_not_invoke_element_decoders_when_the_view_is_empty() {
    reset();
    let view = variable_vector_view(&[], 0).unwrap();
    assert!(decode_elements::<TracedByte>(&view).unwrap().is_empty());
    assert_eq!(calls(), 0);
    assert!(drops().is_empty());
}

#[test]
fn should_drop_nested_prefixes_when_a_typed_child_fails() {
    reset();
    let bytes = [8, 0, 0, 0, 9, 0, 0, 0, 7, 8, 255];
    assert_eq!(
        decode::<List<List<TracedByte, 2>, 2>>(&bytes).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(calls(), 3);
    let mut actual = drops();
    actual.sort();
    assert_eq!(actual, [7, 8]);
}

#[test]
fn bounded_progressive_capacity_refusal_invokes_no_callbacks() {
    reset();
    assert_eq!(
        decode::<BoundedProgressiveList<TracedByte, 1>>(&[7, 255]).unwrap_err(),
        CodecError::OverLimit
    );
    assert_eq!(calls(), 0);
    assert!(drops().is_empty());
}

#[test]
fn should_drop_prefixes_at_each_failure_position_in_all_fixed_sequence_families() {
    for family in 0..4 {
        for input in [[255, 7, 9], [7, 255, 9], [7, 9, 255]] {
            reset();
            let error = match family {
                0 => Vector::<TracedByte, 3>::decode(&input).unwrap_err(),
                1 => List::<TracedByte, 3>::decode(&input).unwrap_err(),
                2 => ProgressiveList::<TracedByte>::decode(&input).unwrap_err(),
                _ => BoundedProgressiveList::<TracedByte, 3>::decode(&input).unwrap_err(),
            };
            let prefix: Vec<_> = input.into_iter().take_while(|&byte| byte != 255).collect();
            assert_eq!(error, CodecError::InvalidLength);
            assert_eq!(calls(), prefix.len() + 1);
            assert_eq!(drops(), prefix);
        }
    }
}

#[test]
fn should_return_before_callbacks_when_typed_layout_is_invalid() {
    reset();
    assert_eq!(
        decode::<Vector<TracedByte, 2>>(&[7]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert_eq!(calls(), 0);
    assert!(drops().is_empty());
}
