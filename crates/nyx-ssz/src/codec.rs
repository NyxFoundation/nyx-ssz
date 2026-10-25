//! Typed SSZ codecs. External implementations require their own correctness proof.

use crate::{
    error::CodecError,
    scalar, tree_hash,
    type_info::{self, TypeDescriptor},
    U256,
};

/// An owned SSZ type with canonical encoding, decoding and tree hashing.
///
/// Native conformance tests and formal proof coverage are tracked separately.
/// Implementing or deriving this trait does not automatically certify its laws.
pub trait Ssz: Sized {
    /// Immutable declaration used for type validation and layout metadata.
    const TYPE: &'static TypeDescriptor;
    /// Exact reference bytes, its protocol error, or `NativeOutputOverflow` iff
    /// successful natural reference bytes have length greater than `usize::MAX`.
    /// Representable successes cannot use the native refusal; protocol faults
    /// retain their distinct errors and precedence, including through parents.
    fn encode(&self) -> Result<Vec<u8>, CodecError> {
        let mut output = vec![0; self.encoded_len()?];
        self.encode_into(&mut output)?;
        Ok(output)
    }
    /// Exact encoded size, including complete value validation and error precedence.
    /// Must agree with encode_into; native unrepresentability is not a protocol fault.
    fn encoded_len(&self) -> Result<usize, CodecError>;
    /// Write canonical bytes into an exactly sized destination without scratch allocation.
    /// Wrong destination length is rejected before writing. Other failures may leave
    /// partial writes; callers must discard the destination on error.
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError>;
    fn decode(bytes: &[u8]) -> Result<Self, CodecError>;
    /// Borrow the canonical concatenated bytes of a fixed-width sequence when the
    /// representation permits a safe native view. Implementations returning `Some`
    /// guarantee exact element order, width and canonical byte order for every value.
    fn fixed_sequence_bytes(values: &[Self]) -> Option<&[u8]> {
        let _ = values;
        None
    }
    /// Decode fixed-width values under the four-byte sequence envelope, without
    /// an additional element-count bound. Success must return one value per
    /// declared width. Variable declarations are rejected. Integer implementations
    /// use safe bulk conversion; other types retain their individual callbacks.
    fn decode_fixed_sequence(bytes: &[u8]) -> Result<Vec<Self>, CodecError> {
        decode_fixed_sequence_default(bytes)
    }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError>;
}

/// Return a fixed width, variable layout (`Ok(None)`), or size overflow.
pub const fn fixed_size<T: Ssz>() -> Result<Option<usize>, CodecError> {
    type_info::fixed_size(T::TYPE)
}

pub(crate) fn validated_fixed_sequence_bytes<T: Ssz>(
    values: &[T],
) -> Result<Option<&[u8]>, CodecError> {
    let Some(bytes) = T::fixed_sequence_bytes(values) else {
        return Ok(None);
    };
    let width = fixed_size::<T>()?.ok_or(CodecError::BadDeclaration)?;
    let expected = values
        .len()
        .checked_mul(width)
        .ok_or(CodecError::SizeOverflow)?;
    if bytes.len() != expected {
        return Err(CodecError::InvalidLength);
    }
    Ok(Some(bytes))
}

/// Whether declared values pack together before collection Merkleization.
pub fn is_basic<T: Ssz>() -> bool {
    type_info::is_basic(T::TYPE)
}

/// Compare declared Merkle layouts; compatibility does not imply type validity.
pub fn is_compatible<Left: Ssz, Right: Ssz>() -> bool {
    type_info::is_compatible(Left::TYPE, Right::TYPE)
}

/// Check a type declaration without invoking any value codec.
pub const fn validate_type<T: Ssz>() -> Result<(), CodecError> {
    type_info::validate(T::TYPE)
}

/// Sum compile-time fixed widths for generated container decoders.
#[doc(hidden)]
pub const fn fixed_layout<const N: usize>(
    widths: [Result<Option<usize>, CodecError>; N],
) -> Result<Option<(usize, [usize; N])>, CodecError> {
    let mut total = 0usize;
    let mut exact = [0usize; N];
    let mut index = 0;
    while index < N {
        match widths[index] {
            Ok(Some(width)) => match total.checked_add(width) {
                Some(next) => {
                    exact[index] = width;
                    total = next;
                }
                None => return Err(CodecError::InvalidLength),
            },
            _ => return Ok(None),
        }
        index += 1;
    }
    Ok(Some((total, exact)))
}

/// Encode a typed value using its codec.
pub fn encode<T: Ssz>(value: &T) -> Result<Vec<u8>, CodecError> {
    value.encode()
}

/// Decode a typed value.
pub fn decode<T: Ssz>(bytes: &[u8]) -> Result<T, CodecError> {
    T::decode(bytes)
}

/// Decode validated body ranges.
///
/// The view validates layout, not the element type declaration. Each element
/// codec validates its own value; an empty view invokes no element codecs.
pub fn decode_elements<T: Ssz>(
    view: &crate::sequence::SequenceView<'_>,
) -> Result<Vec<T>, CodecError> {
    let mut values = Vec::with_capacity(view.ranges.len());
    let mut index = 0;
    while index < view.ranges.len() {
        let range = &view.ranges[index];
        values.push(T::decode(&view.input[range.start..range.end])?);
        index += 1;
    }
    Ok(values)
}

/// Decode a prevalidated fixed layout without allocating an element-range table.
pub(crate) fn decode_fixed<T: Ssz>(
    input: &[u8],
    width: usize,
    count: usize,
) -> Result<Vec<T>, CodecError> {
    let mut values = Vec::new();
    for index in 0..count {
        let start = index * width;
        let value = T::decode(&input[start..start + width])?;
        if index == 0 {
            let capacity = count.min(input.len() / core::mem::size_of::<T>().max(1));
            values.reserve_exact(capacity);
        }
        values.push(value);
    }
    Ok(values)
}

fn decode_fixed_sequence_default<T: Ssz>(input: &[u8]) -> Result<Vec<T>, CodecError> {
    validate_type::<T>()?;
    match fixed_size::<T>() {
        Ok(Some(width)) => {
            let count = crate::sequence::fixed_list_count(input.len(), width, None)?;
            decode_fixed(input, width, count)
        }
        Ok(None) => Err(CodecError::BadDeclaration),
        Err(CodecError::SizeOverflow) => {
            if input.len() as u128 >= 4_294_967_296 {
                return Err(CodecError::SizeOverflow);
            }
            if input.is_empty() {
                Ok(Vec::new())
            } else {
                Err(CodecError::ScopeUndivided)
            }
        }
        Err(error) => Err(error),
    }
}

fn decode_integer_sequence<T: Ssz + bytemuck::Pod, const WIDTH: usize>(
    input: &[u8],
    convert: impl Fn([u8; WIDTH]) -> T,
) -> Result<Vec<T>, CodecError> {
    crate::sequence::fixed_list_count(input.len(), WIDTH, None)?;
    #[cfg(target_endian = "little")]
    if let Ok(values) = bytemuck::try_cast_slice::<u8, T>(input) {
        return Ok(values.to_vec());
    }
    Ok(collect_integer_sequence(input, convert))
}

fn decode_integer_sequence_scalar<T, const WIDTH: usize>(
    input: &[u8],
    convert: impl Fn([u8; WIDTH]) -> T,
) -> Result<Vec<T>, CodecError> {
    crate::sequence::fixed_list_count(input.len(), WIDTH, None)?;
    Ok(collect_integer_sequence(input, convert))
}

fn collect_integer_sequence<T, const WIDTH: usize>(
    input: &[u8],
    convert: impl Fn([u8; WIDTH]) -> T,
) -> Vec<T> {
    input
        .as_chunks::<WIDTH>()
        .0
        .iter()
        .copied()
        .map(convert)
        .collect()
}

/// Compute a typed value's SSZ root using its codec.
pub fn hash_tree_root<T: Ssz>(value: &T) -> Result<[u8; 32], CodecError> {
    value.hash_tree_root()
}

fn scalar_error(error: scalar::ScalarError) -> CodecError {
    match error {
        scalar::ScalarError::InvalidLength => CodecError::InvalidLength,
        scalar::ScalarError::InvalidBoolean => CodecError::InvalidBoolean,
    }
}

macro_rules! integer_bulk_method {
    (bool) => {};
    ($type:ident) => {
        #[cfg(target_endian = "little")]
        fn fixed_sequence_bytes(values: &[Self]) -> Option<&[u8]> {
            Some(bytemuck::cast_slice(values))
        }

        fn decode_fixed_sequence(input: &[u8]) -> Result<Vec<Self>, CodecError> {
            decode_integer_sequence::<Self, { core::mem::size_of::<$type>() }>(
                input,
                <$type>::from_le_bytes,
            )
        }
    };
}

macro_rules! basic_codec {
    ($type:ident, $declaration:expr, $encode:ident, $decode:ident, $root:ident) => {
        impl Ssz for $type {
            const TYPE: &'static TypeDescriptor = &$declaration;

            integer_bulk_method!($type);

            fn encoded_len(&self) -> Result<usize, CodecError> {
                Ok(core::mem::size_of::<$type>())
            }

            fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
                crate::encoding::copy(output, &scalar::$encode(*self))
            }

            fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
                scalar::$decode(bytes).map_err(scalar_error)
            }

            fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
                Ok(tree_hash::$root(*self))
            }
        }
    };
}

basic_codec!(
    bool,
    TypeDescriptor::Boolean,
    encode_bool,
    decode_bool,
    hash_tree_root_bool
);
basic_codec!(
    u8,
    TypeDescriptor::Uint(1),
    encode_u8,
    decode_u8,
    hash_tree_root_u8
);
basic_codec!(
    u16,
    TypeDescriptor::Uint(2),
    encode_u16,
    decode_u16,
    hash_tree_root_u16
);
basic_codec!(
    u32,
    TypeDescriptor::Uint(4),
    encode_u32,
    decode_u32,
    hash_tree_root_u32
);
basic_codec!(
    u64,
    TypeDescriptor::Uint(8),
    encode_u64,
    decode_u64,
    hash_tree_root_u64
);
basic_codec!(
    u128,
    TypeDescriptor::Uint(16),
    encode_u128,
    decode_u128,
    hash_tree_root_u128
);

impl Ssz for U256 {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Uint(32);

    fn decode_fixed_sequence(input: &[u8]) -> Result<Vec<Self>, CodecError> {
        decode_integer_sequence_scalar::<Self, 32>(input, Self::from_le_bytes::<32>)
    }

    fn encoded_len(&self) -> Result<usize, CodecError> {
        Ok(32)
    }

    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        crate::encoding::copy(output, &scalar::encode_u256(self))
    }

    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        scalar::decode_u256(bytes).map_err(scalar_error)
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        Ok(tree_hash::hash_tree_root_u256(self))
    }
}
