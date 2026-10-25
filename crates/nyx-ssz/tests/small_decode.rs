use nyx_ssz::{
    codec::{fixed_layout, Ssz},
    container::fixed_container_cursor,
    error::CodecError,
};

const EXACT_LAYOUT: Result<Option<(usize, [usize; 2])>, CodecError> =
    fixed_layout([Ok(Some(1)), Ok(Some(2))]);
const VARIABLE_LAYOUT: Result<Option<(usize, [usize; 2])>, CodecError> =
    fixed_layout([Ok(Some(1)), Ok(None)]);
const OVERFLOWING_LAYOUT: Result<Option<(usize, [usize; 2])>, CodecError> =
    fixed_layout([Ok(Some(usize::MAX)), Ok(Some(1))]);
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn should_classify_generated_fixed_layouts_at_compile_time() {
    assert_eq!(EXACT_LAYOUT, Ok(Some((3, [1, 2]))));
    assert_eq!(VARIABLE_LAYOUT, Ok(None));
    assert_eq!(OVERFLOWING_LAYOUT, Err(CodecError::InvalidLength));
}

#[test]
fn should_classify_fixed_container_widths_without_allocating_ranges() {
    assert!(fixed_container_cursor(&[1, 2], &[Ok(Some(1)), Ok(Some(1))])
        .unwrap()
        .is_some());
    assert_eq!(
        fixed_container_cursor(&[1], &[Ok(Some(2))]).unwrap_err(),
        CodecError::InvalidLength
    );
    assert!(
        fixed_container_cursor(&[1, 2, 3, 4], &[Ok(Some(1)), Ok(None)])
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fixed_container_cursor(&[], &[Ok(Some(usize::MAX)), Ok(Some(1))]).unwrap_err(),
        CodecError::InvalidLength
    );
}

struct Owner(u8);
static DROPS: AtomicUsize = AtomicUsize::new(0);
impl Drop for Owner {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}
impl Ssz for Owner {
    const TYPE: &'static nyx_ssz::type_info::TypeDescriptor =
        &nyx_ssz::type_info::TypeDescriptor::Uint(1);
    fn encoded_len(&self) -> Result<usize, CodecError> {
        Ok(1)
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        nyx_ssz::encoding::copy(output, &[self.0])
    }
    fn decode(input: &[u8]) -> Result<Self, CodecError> {
        match input {
            [0] => Ok(Self(0)),
            _ => Err(CodecError::InvalidBoolean),
        }
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Ok([self.0; 32])
    }
}

#[derive(nyx_ssz::Ssz)]
struct Pair {
    first: Owner,
    second: Owner,
}

#[test]
fn should_drop_fixed_container_prefixes_when_a_later_field_fails() {
    DROPS.store(0, Ordering::SeqCst);
    assert_eq!(
        Pair::decode(&[0, 2]).err().unwrap(),
        CodecError::InvalidBoolean
    );
    assert_eq!(DROPS.load(Ordering::SeqCst), 1);
}
