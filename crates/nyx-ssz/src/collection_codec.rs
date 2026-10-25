//! Typed Vector/List codecs.

use crate::{
    codec::{decode_elements, fixed_size, validate_type, Ssz},
    collection_tree::root_sequence,
    collections::{list, vector, LengthErrorKind, List, Vector},
    error::CodecError,
    sequence::{fixed_list_count, validate_fixed_vector, variable_list_view, variable_vector_view},
    type_info::TypeDescriptor,
};

fn storage_error(kind: LengthErrorKind) -> CodecError {
    match kind {
        LengthErrorKind::EmptyVector => CodecError::EmptyVector,
        LengthErrorKind::InvalidLength => CodecError::InvalidLength,
        LengthErrorKind::OverLimit => CodecError::OverLimit,
    }
}

fn vector_type<T: Ssz, const N: usize>() -> Result<(), CodecError> {
    if N == 0 {
        return Err(CodecError::EmptyVector);
    }
    validate_type::<T>()
}

fn decode_fixed_values<T: Ssz>(input: &[u8], count: usize) -> Result<Vec<T>, CodecError> {
    let values = T::decode_fixed_sequence(input)?;
    if values.len() != count {
        return Err(CodecError::InvalidLength);
    }
    Ok(values)
}

fn decode_vector_values<T: Ssz>(input: &[u8], count: usize) -> Result<Vec<T>, CodecError> {
    match fixed_size::<T>() {
        Ok(Some(width)) => {
            validate_fixed_vector(input.len(), width, count)?;
            decode_fixed_values(input, count)
        }
        Ok(None) => decode_elements(&variable_vector_view(input, count)?),
        Err(CodecError::SizeOverflow) => {
            let kind = if input.len() as u128 >= 4_294_967_296 {
                CodecError::SizeOverflow
            } else {
                CodecError::InvalidLength
            };
            Err(kind)
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn decode_list_values<T: Ssz>(
    input: &[u8],
    limit: Option<usize>,
) -> Result<Vec<T>, CodecError> {
    match fixed_size::<T>() {
        Ok(Some(width)) => {
            let count = fixed_list_count(input.len(), width, limit)?;
            decode_fixed_values(input, count)
        }
        Ok(None) => decode_elements(&variable_list_view(input, limit)?),
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

fn decode_vector<T: Ssz, const N: usize>(input: &[u8]) -> Result<Vector<T, N>, CodecError> {
    vector_type::<T, N>()?;
    vector(decode_vector_values::<T>(input, N)?).map_err(|error| storage_error(error.kind))
}

fn decode_list<T: Ssz, const MAX: usize>(input: &[u8]) -> Result<List<T, MAX>, CodecError> {
    validate_type::<T>()?;
    list(decode_list_values::<T>(input, Some(MAX))?).map_err(|error| storage_error(error.kind))
}

impl<T: Ssz, const N: usize> Ssz for Vector<T, N> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Vector(T::TYPE, N);

    fn encoded_len(&self) -> Result<usize, CodecError> {
        vector_type::<T, N>()?;
        let values = self.as_slice();
        if values.len() != N {
            return Err(CodecError::InvalidLength);
        }
        crate::encoding::sequence_len(values)
    }

    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        crate::encoding::check_length(output, self.encoded_len()?)?;
        crate::encoding::write_sequence(self.as_slice(), output)
    }

    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        decode_vector(bytes)
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        vector_type::<T, N>()?;
        let values = self.as_slice();
        if values.len() != N {
            return Err(CodecError::InvalidLength);
        }
        root_sequence(values, N, false)
    }
}

impl<T: Ssz, const MAX: usize> Ssz for List<T, MAX> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::List(T::TYPE, MAX);

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
        decode_list(bytes)
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        validate_type::<T>()?;
        let values = self.as_slice();
        if values.len() > MAX {
            return Err(CodecError::OverLimit);
        }
        root_sequence(values, MAX, true)
    }
}
