use nyx_ssz::{
    codec::{decode_elements, Ssz},
    collections::{BoundedProgressiveList, List, ProgressiveList, Vector},
    error::CodecError,
    sequence::{fixed_list_view, fixed_vector_view, SequenceView},
    type_info::TypeDescriptor,
};

fn through_view<T: Ssz>(view: Result<SequenceView<'_>, CodecError>) -> Result<Vec<T>, CodecError> {
    decode_elements::<T>(&view?)
}

#[test]
fn should_match_view_based_results_for_all_short_boolean_encodings() {
    for count in 0..=5_u32 {
        for pattern in 0..3_usize.pow(count) {
            let mut pattern = pattern;
            let input: Vec<_> = (0..count)
                .map(|_| {
                    let byte = (pattern % 3) as u8;
                    pattern /= 3;
                    byte
                })
                .collect();
            let expected = through_view::<bool>(fixed_list_view(&input, 1, Some(3)));
            assert_eq!(
                List::<bool, 3>::decode(&input).map(|value| value.as_slice().to_vec()),
                expected
            );
            assert_eq!(
                BoundedProgressiveList::<bool, 3>::decode(&input)
                    .map(|value| value.as_slice().to_vec()),
                expected
            );
            let expected = through_view::<bool>(fixed_list_view(&input, 1, None));
            assert_eq!(
                ProgressiveList::<bool>::decode(&input).map(|value| value.as_slice().to_vec()),
                expected
            );
            let expected = through_view::<bool>(fixed_vector_view(&input, 1, 3));
            assert_eq!(
                Vector::<bool, 3>::decode(&input).map(|value| value.as_slice().to_vec()),
                expected
            );
        }
    }
}

#[test]
fn should_preserve_alignment_and_capacity_faults_for_multibyte_elements() {
    for count in 0..=12 {
        let input: Vec<_> = (0..count).map(|index| index as u8).collect();
        let expected = through_view::<u16>(fixed_list_view(&input, 2, Some(3)));
        assert_eq!(
            List::<u16, 3>::decode(&input).map(|value| value.as_slice().to_vec()),
            expected
        );
        assert_eq!(
            BoundedProgressiveList::<u16, 3>::decode(&input).map(|value| value.as_slice().to_vec()),
            expected
        );
        let expected = through_view::<u16>(fixed_vector_view(&input, 2, 3));
        assert_eq!(
            Vector::<u16, 3>::decode(&input).map(|value| value.as_slice().to_vec()),
            expected
        );
    }
}

#[derive(Debug)]
struct PaddedBool {
    value: bool,
    _padding: [u8; 511],
}
impl Ssz for PaddedBool {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Boolean;
    fn encoded_len(&self) -> Result<usize, CodecError> {
        self.value.encoded_len()
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        self.value.encode_into(output)
    }
    fn decode(input: &[u8]) -> Result<Self, CodecError> {
        bool::decode(input).map(|value| Self {
            value,
            _padding: [0; 511],
        })
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        self.value.hash_tree_root()
    }
}

#[test]
fn should_avoid_reserving_unrepresentable_full_storage_before_a_later_child_refusal() {
    const COUNT: usize = 1 << 22;
    let mut input = vec![0; COUNT];
    input[1] = 2;
    assert_eq!(core::mem::size_of::<PaddedBool>(), 512);
    assert_eq!(
        List::<PaddedBool, COUNT>::decode(&input).err().unwrap(),
        CodecError::InvalidBoolean
    );
}

// A metadata-only dictionary: no native input can contain one complete element.
#[derive(Debug)]
struct TooWide;
impl Ssz for TooWide {
    const TYPE: &'static TypeDescriptor =
        &TypeDescriptor::Vector(&TypeDescriptor::ByteVector(usize::MAX), 2);
    fn encoded_len(&self) -> Result<usize, CodecError> {
        panic!("layout must not invoke value encoding")
    }
    fn encode_into(&self, _: &mut [u8]) -> Result<(), CodecError> {
        panic!("layout must not invoke value encoding")
    }
    fn decode(_: &[u8]) -> Result<Self, CodecError> {
        panic!("unrepresentable width must not invoke a child decoder")
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        panic!("layout must not invoke hashing")
    }
}

#[test]
fn should_accept_empty_lists_but_refuse_nonempty_scopes_when_fixed_width_is_unrepresentable() {
    assert!(TooWide::decode_fixed_sequence(&[]).unwrap().is_empty());
    assert_eq!(
        TooWide::decode_fixed_sequence(&[1]).err().unwrap(),
        CodecError::ScopeUndivided
    );
    assert!(List::<TooWide, 0>::decode(&[])
        .unwrap()
        .as_slice()
        .is_empty());
    assert!(ProgressiveList::<TooWide>::decode(&[])
        .unwrap()
        .as_slice()
        .is_empty());
    assert!(BoundedProgressiveList::<TooWide, 0>::decode(&[])
        .unwrap()
        .as_slice()
        .is_empty());
    assert_eq!(
        List::<TooWide, 1>::decode(&[1]).err().unwrap(),
        CodecError::ScopeUndivided
    );
    assert_eq!(
        Vector::<TooWide, 1>::decode(&[]).err().unwrap(),
        CodecError::InvalidLength
    );
}
