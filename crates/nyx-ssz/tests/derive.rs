use nyx_ssz::{
    bytecollections::ByteList,
    codec::{self, Ssz},
    error::CodecError,
    sha256::hash_pair,
};

#[derive(Debug, nyx_ssz::Ssz)]
struct Record {
    a: u16,
    bytes: ByteList<8>,
    flag: bool,
}

#[derive(Debug, PartialEq, nyx_ssz::Ssz)]
#[ssz(progressive)]
struct Sparse {
    #[ssz(position = 4)]
    last: u16,
    #[ssz(position = 0, rename = "最初")]
    first: bool,
}

#[derive(Debug, PartialEq, nyx_ssz::Ssz)]
enum Choice {
    #[ssz(selector = 7)]
    Small(u16),
    #[ssz(selector = 1)]
    Other(u16),
}

#[derive(nyx_ssz::Ssz)]
struct Generic<T, const N: usize> {
    items: nyx_ssz::collections::Vector<T, N>,
}

#[test]
fn should_round_trip_mixed_fields_when_deriving_a_container() {
    let value = Record {
        a: 0x1234,
        bytes: ByteList::from_bytes(vec![9, 8]).unwrap(),
        flag: true,
    };
    let wire = vec![0x34, 0x12, 7, 0, 0, 0, 1, 9, 8];
    assert_eq!(value.encode().unwrap(), wire);
    assert_eq!(Record::decode(&wire).unwrap().encode().unwrap(), wire);
    let a = value.a.hash_tree_root().unwrap();
    let b = value.bytes.hash_tree_root().unwrap();
    let c = value.flag.hash_tree_root().unwrap();
    assert_eq!(
        value.hash_tree_root().unwrap(),
        hash_pair(&hash_pair(&a, &b), &hash_pair(&c, &[0; 32]))
    );
    assert_eq!(codec::fixed_size::<Record>(), Ok(None));
}

#[test]
fn should_sort_positions_and_mix_the_active_mask_when_deriving_progressive_containers() {
    let value = Sparse {
        last: 5,
        first: true,
    };
    assert_eq!(value.encode().unwrap(), vec![1, 5, 0]);
    assert_eq!(Sparse::decode(&[1, 5, 0]).unwrap(), value);
    let first = true.hash_tree_root().unwrap();
    let last = 5u16.hash_tree_root().unwrap();
    let zero = [0; 32];
    let second = hash_pair(&hash_pair(&zero, &zero), &hash_pair(&zero, &last));
    let contents = hash_pair(&first, &hash_pair(&second, &zero));
    let mut mask = zero;
    mask[0] = 0b10001;
    assert_eq!(value.hash_tree_root().unwrap(), hash_pair(&contents, &mask));
}

#[test]
fn should_use_explicit_selectors_when_deriving_compatible_unions() {
    let value = Choice::Small(0x1234);
    assert_eq!(value.encode().unwrap(), vec![7, 0x34, 0x12]);
    assert_eq!(Choice::decode(&[7, 0x34, 0x12]).unwrap(), value);
    let mut selector = [0; 32];
    selector[0] = 7;
    assert_eq!(
        value.hash_tree_root().unwrap(),
        hash_pair(&0x1234u16.hash_tree_root().unwrap(), &selector)
    );
    assert_eq!(Choice::decode(&[]).unwrap_err(), CodecError::NoSelector);
    assert_eq!(
        Choice::decode(&[0]).unwrap_err(),
        CodecError::UnknownSelector
    );
}

#[test]
fn should_return_the_first_child_error_when_a_later_field_refuses_its_body() {
    assert_eq!(
        Record::decode(&[1, 0, 7, 0, 0, 0, 2]).unwrap_err(),
        CodecError::InvalidBoolean
    );
    assert_eq!(Record::decode(&[0]).unwrap_err(), CodecError::ScopeTooSmall);
}

#[test]
fn should_derive_generic_declarations_when_children_have_codec_bounds() {
    let value = Generic::<u16, 2> {
        items: nyx_ssz::collections::vector(vec![1, 2]).unwrap(),
    };
    assert_eq!(value.encode().unwrap(), vec![1, 0, 2, 0]);
    assert_eq!(
        Generic::<u16, 2>::decode(&[1, 0, 2, 0])
            .unwrap()
            .items
            .as_slice(),
        &[1, 2]
    );
}
