//! Progressive-list codecs with exact optional bounds.

use crate::{
    codec::{validate_type, Ssz},
    collection_codec::decode_list_values,
    collection_tree::root_progressive_sequence,
    collections::{
        bounded_progressive_list, progressive_list, BoundedProgressiveList, LengthErrorKind,
        ProgressiveList,
    },
    error::CodecError,
    type_info::TypeDescriptor,
};

fn storage_error(kind: LengthErrorKind) -> CodecError {
    match kind {
        LengthErrorKind::EmptyVector => CodecError::EmptyVector,
        LengthErrorKind::InvalidLength => CodecError::InvalidLength,
        LengthErrorKind::OverLimit => CodecError::OverLimit,
    }
}

fn decode_progressive<T: Ssz>(input: &[u8]) -> Result<ProgressiveList<T>, CodecError> {
    validate_type::<T>()?;
    Ok(progressive_list(decode_list_values::<T>(input, None)?))
}

fn decode_bounded_progressive<T: Ssz, const MAX: usize>(
    input: &[u8],
) -> Result<BoundedProgressiveList<T, MAX>, CodecError> {
    validate_type::<T>()?;
    bounded_progressive_list(decode_list_values::<T>(input, Some(MAX))?)
        .map_err(|error| storage_error(error.kind))
}

impl<T: Ssz, const MAX: usize> Ssz for BoundedProgressiveList<T, MAX> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::ProgressiveList(T::TYPE, Some(MAX));

    fn encoded_len(&self) -> Result<usize, CodecError> {
        validate_type::<T>()?;
        let values = self.as_slice();
        if values.len() > MAX {
            return Err(CodecError::OverLimit);
        }
        crate::encoding::sequence_len(values)
    }

    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        crate::encoding::check_length(output, self.encoded_len()?)?;
        crate::encoding::write_sequence(self.as_slice(), output)
    }

    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        decode_bounded_progressive(bytes)
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        validate_type::<T>()?;
        // The pinned Progressive root ignores the context cap, even on private states.
        root_progressive_sequence(self.as_slice())
    }
}

impl<T: Ssz> Ssz for ProgressiveList<T> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::ProgressiveList(T::TYPE, None);

    fn encoded_len(&self) -> Result<usize, CodecError> {
        validate_type::<T>()?;
        crate::encoding::sequence_len(self.as_slice())
    }

    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        crate::encoding::check_length(output, self.encoded_len()?)?;
        crate::encoding::write_sequence(self.as_slice(), output)
    }

    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        decode_progressive(bytes)
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        validate_type::<T>()?;
        root_progressive_sequence(self.as_slice())
    }
}
