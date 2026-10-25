use std::marker::PhantomData;

use nyx_ssz::{
    codec::{fixed_size, is_basic, is_compatible, validate_type, Ssz},
    error::CodecError,
    type_info::TypeDescriptor,
};

trait Declaration {
    const TYPE: &'static TypeDescriptor;
}

struct Declared<D>(PhantomData<D>);

// Inspecting declarations must never call a value codec.
impl<D: Declaration> Ssz for Declared<D> {
    const TYPE: &'static TypeDescriptor = D::TYPE;

    fn encoded_len(&self) -> Result<usize, CodecError> {
        panic!("metadata must not invoke encoded_len")
    }
    fn encode_into(&self, _: &mut [u8]) -> Result<(), CodecError> {
        panic!("metadata must not invoke encode_into")
    }
    fn decode(_: &[u8]) -> Result<Self, CodecError> {
        panic!("metadata must not call decode")
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        panic!("metadata must not call hash_tree_root")
    }
}

struct VectorShape<T, const N: usize>(PhantomData<T>);
struct ListShape<T, const N: usize>(PhantomData<T>);
struct ProgressiveShape<T, const N: usize, const BOUNDED: bool>(PhantomData<T>);
struct UintShape<const WIDTH: usize>;

impl<T: Ssz, const N: usize> Declaration for VectorShape<T, N> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Vector(T::TYPE, N);
}
impl<T: Ssz, const N: usize> Declaration for ListShape<T, N> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::List(T::TYPE, N);
}
impl<T: Ssz, const N: usize, const B: bool> Declaration for ProgressiveShape<T, N, B> {
    const TYPE: &'static TypeDescriptor =
        &TypeDescriptor::ProgressiveList(T::TYPE, if B { Some(N) } else { None });
}
impl<const WIDTH: usize> Declaration for UintShape<WIDTH> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Uint(WIDTH);
}

type Vector<T, const N: usize> = Declared<VectorShape<T, N>>;
type List<T, const N: usize> = Declared<ListShape<T, N>>;
type Progressive<T, const N: usize, const B: bool> = Declared<ProgressiveShape<T, N, B>>;
type Uint<const WIDTH: usize> = Declared<UintShape<WIDTH>>;

#[test]
fn should_derive_recursive_fixed_widths_when_sequences_are_nested() {
    assert_eq!(fixed_size::<Vector<Vector<u16, 3>, 4>>(), Ok(Some(24)));
    assert_eq!(fixed_size::<Vector<List<u16, 4>, 3>>(), Ok(None));
    assert_eq!(fixed_size::<List<Vector<u16, 3>, 4>>(), Ok(None));
    assert_eq!(fixed_size::<Progressive<u16, 0, false>>(), Ok(None));
    assert_eq!(fixed_size::<Progressive<u16, 0, true>>(), Ok(None));
    assert!(!is_basic::<Vector<u8, 1>>());
    assert!(!is_basic::<List<bool, 0>>());
    assert!(!is_basic::<Progressive<bool, 0, false>>());
    assert_eq!(validate_type::<Vector<Vector<u16, 3>, 4>>(), Ok(()));
}

#[test]
fn should_preserve_reference_widths_when_nested_products_overflow_or_cancel() {
    type TooWide = Vector<Uint<{ usize::MAX }>, 2>;
    assert_eq!(fixed_size::<TooWide>(), Err(CodecError::SizeOverflow));
    assert_eq!(fixed_size::<Vector<TooWide, 0>>(), Ok(Some(0)));
    assert_eq!(
        fixed_size::<Vector<TooWide, 1>>(),
        Err(CodecError::SizeOverflow)
    );
    assert_eq!(fixed_size::<Vector<List<TooWide, 1>, 0>>(), Ok(None));
    assert_eq!(
        fixed_size::<Vector<u16, { usize::MAX }>>(),
        Err(CodecError::SizeOverflow)
    );
    assert_eq!(validate_type::<Vector<u16, { usize::MAX }>>(), Ok(()));
    assert_eq!(
        fixed_size::<Vector<Vector<TooWide, { usize::MAX }>, 0>>(),
        Ok(Some(0))
    );
}

#[test]
fn should_validate_children_when_a_sequence_declares_zero_capacity() {
    assert_eq!(
        CodecError::EmptyVector.to_string(),
        "SSZ vector declarations must contain at least one element"
    );
    assert_eq!(validate_type::<List<bool, 0>>(), Ok(()));
    assert_eq!(validate_type::<Progressive<bool, 0, true>>(), Ok(()));
    assert_eq!(validate_type::<Progressive<bool, 0, false>>(), Ok(()));
    assert_eq!(
        validate_type::<List<Uint<3>, 0>>(),
        Err(CodecError::InvalidIntegerWidth)
    );
    assert_eq!(
        validate_type::<Progressive<Uint<3>, 0, false>>(),
        Err(CodecError::InvalidIntegerWidth)
    );
    assert_eq!(
        validate_type::<Vector<Uint<3>, 0>>(),
        Err(CodecError::EmptyVector)
    );
    assert_eq!(
        validate_type::<Vector<Uint<3>, 1>>(),
        Err(CodecError::InvalidIntegerWidth)
    );
    assert_eq!(
        validate_type::<List<Vector<bool, 0>, 1>>(),
        Err(CodecError::EmptyVector)
    );
}

#[test]
fn should_compare_recursive_layouts_when_progressive_limits_differ() {
    assert!(is_compatible::<Vector<u8, 3>, Vector<u8, 3>>());
    assert!(!is_compatible::<Vector<u8, 3>, Vector<u8, 4>>());
    assert!(!is_compatible::<Vector<bool, 3>, Vector<u8, 3>>());
    assert!(!is_compatible::<Vector<u8, 3>, List<u8, 3>>());
    assert!(is_compatible::<
        List<Vector<u16, 2>, 3>,
        List<Vector<u16, 2>, 3>,
    >());
    assert!(!is_compatible::<List<bool, 3>, List<bool, 4>>());
    assert!(is_compatible::<
        Progressive<bool, 0, false>,
        Progressive<bool, 4, true>,
    >());
    assert!(is_compatible::<
        Vector<Progressive<bool, 2, true>, 3>,
        Vector<Progressive<bool, 4, true>, 3>,
    >());
    assert!(!is_compatible::<
        Progressive<bool, 0, false>,
        Progressive<u8, 4, true>,
    >());
    assert!(!is_compatible::<List<bool, 4>, Progressive<bool, 4, true>>());
}
