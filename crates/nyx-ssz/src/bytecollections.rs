//! Owned opaque SSZ byte arrays, distinct from generic uint8 sequences.
//!
//! Values of at most 32 bytes are stored inline. Larger values retain the accepted
//! `Vec` allocation. Private storage prevents resizing; the unit-array markers
//! preserve const bounds during generic extraction only.

use crate::{
    bytes::{self, ByteError},
    codec::Ssz,
    error::CodecError,
    tree_hash,
    type_info::TypeDescriptor,
};

const INLINE_LIMIT: usize = 32;

fn inline_bytes<const N: usize>(bytes: &[u8]) -> [u8; INLINE_LIMIT] {
    let mut inline = [0; INLINE_LIMIT];
    inline[..N].copy_from_slice(bytes);
    inline
}

/// Exactly `N` bytes; zero-length vector declarations are invalid.
///
/// `N <= 32` uses `inline` and an empty `heap`. Larger `N` moves bytes into `heap`.
pub struct ByteVector<const N: usize> {
    inline: [u8; INLINE_LIMIT],
    heap: Vec<u8>,
    _bound: [(); N],
}

/// At most `MAX` bytes, including empty values and zero capacity.
pub struct ByteList<const MAX: usize> {
    bytes: Vec<u8>,
    _bound: [(); MAX],
}

fn byte_error(error: ByteError) -> CodecError {
    match error {
        ByteError::EmptyVector => CodecError::EmptyVector,
        ByteError::InvalidLength => CodecError::InvalidLength,
        ByteError::OverLimit => CodecError::OverLimit,
        ByteError::OffsetOverflow => CodecError::SizeOverflow,
    }
}

fn copy_bytes(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}

impl<const N: usize> ByteVector<N> {
    /// Validate the declaration and exact length.
    /// Lengths above 32 bytes move the accepted allocation; smaller lengths are copied inline.
    #[cfg_attr(feature = "verification", verify::start_from)]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, CodecError> {
        match bytes::encode_byte_vector(&bytes, N) {
            Ok(_) => Ok(Self::from_validated(bytes)),
            Err(error) => Err(byte_error(error)),
        }
    }

    /// Borrow every byte in its original order without changing storage.
    #[cfg_attr(feature = "verification", verify::start_from)]
    pub fn as_bytes(&self) -> &[u8] {
        if N <= INLINE_LIMIT {
            &self.inline[..N]
        } else {
            &self.heap
        }
    }

    fn from_slice(bytes: &[u8]) -> Self {
        if N <= INLINE_LIMIT {
            Self {
                inline: inline_bytes::<N>(bytes),
                heap: Vec::new(),
                _bound: [(); N],
            }
        } else {
            Self {
                inline: [0; INLINE_LIMIT],
                heap: bytes.to_vec(),
                _bound: [(); N],
            }
        }
    }

    fn from_validated(bytes: Vec<u8>) -> Self {
        if N <= INLINE_LIMIT {
            Self::from_slice(&bytes)
        } else {
            Self {
                inline: [0; INLINE_LIMIT],
                heap: bytes,
                _bound: [(); N],
            }
        }
    }
}

impl<const MAX: usize> ByteList<MAX> {
    /// Validate the upper bound, moving accepted storage unchanged.
    #[cfg_attr(feature = "verification", verify::start_from)]
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, CodecError> {
        match bytes::encode_byte_list(&bytes, MAX) {
            Ok(_) => Ok(Self {
                bytes,
                _bound: [(); MAX],
            }),
            Err(error) => Err(byte_error(error)),
        }
    }

    /// Borrow every byte in its original order without changing the allocation.
    #[cfg_attr(feature = "verification", verify::start_from)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

// Do not format the zero-sized marker: its logical length can be usize::MAX.
impl<const N: usize> std::fmt::Debug for ByteVector<N> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ByteVector")
            .field("bytes", &self.as_bytes())
            .finish()
    }
}

impl<const MAX: usize> std::fmt::Debug for ByteList<MAX> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ByteList")
            .field("bytes", &self.bytes)
            .finish()
    }
}

mod ssz {
    use super::*;

    impl<const N: usize> Ssz for ByteVector<N> {
        const TYPE: &'static TypeDescriptor = &TypeDescriptor::ByteVector(N);
        const TRUSTED_FIXED_LAYOUT: bool = true;

        fn encode_append_prevalidated(
            &self,
            output: &mut Vec<u8>,
        ) -> Option<Result<(), CodecError>> {
            output.extend_from_slice(self.as_bytes());
            Some(Ok(()))
        }

        fn encoded_len(&self) -> Result<usize, CodecError> {
            if N == 0 {
                return Err(CodecError::EmptyVector);
            }
            if N <= INLINE_LIMIT {
                return Ok(N);
            }
            bytes::encode_byte_vector(self.as_bytes(), N)
                .map(|bytes| bytes.len())
                .map_err(byte_error)
        }
        fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
            if N == 0 {
                return Err(CodecError::EmptyVector);
            }
            if N <= INLINE_LIMIT {
                return crate::encoding::copy(output, &self.inline[..N]);
            }
            let bytes = bytes::encode_byte_vector(self.as_bytes(), N).map_err(byte_error)?;
            crate::encoding::copy(output, bytes)
        }

        fn decode(input: &[u8]) -> Result<Self, CodecError> {
            bytes::decode_byte_vector(input, N)
                .map(Self::from_slice)
                .map_err(byte_error)
        }

        fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
            tree_hash::hash_tree_root_byte_vector(self.as_bytes(), N)
        }
    }

    impl<const MAX: usize> Ssz for ByteList<MAX> {
        const TYPE: &'static TypeDescriptor = &TypeDescriptor::ByteList(MAX);

        fn encode_append_prevalidated(
            &self,
            output: &mut Vec<u8>,
        ) -> Option<Result<(), CodecError>> {
            output.extend_from_slice(self.as_bytes());
            Some(Ok(()))
        }

        fn encoded_len(&self) -> Result<usize, CodecError> {
            bytes::encode_byte_list(self.as_bytes(), MAX)
                .map(|bytes| bytes.len())
                .map_err(byte_error)
        }
        fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
            let bytes = bytes::encode_byte_list(self.as_bytes(), MAX).map_err(byte_error)?;
            crate::encoding::copy(output, bytes)
        }

        fn decode(input: &[u8]) -> Result<Self, CodecError> {
            bytes::decode_byte_list(input, MAX)
                .map(|bytes| Self {
                    bytes: copy_bytes(bytes),
                    _bound: [(); MAX],
                })
                .map_err(byte_error)
        }

        fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
            tree_hash::hash_tree_root_byte_list(self.as_bytes(), MAX)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codecs_defensively_check_private_storage_and_declarations() {
        let zero = ByteVector::<0> {
            inline: [0; INLINE_LIMIT],
            heap: Vec::new(),
            _bound: [],
        };
        assert_eq!(zero.encode(), Err(CodecError::EmptyVector));
        assert_eq!(zero.hash_tree_root(), Err(CodecError::EmptyVector));
        let short = ByteVector::<33> {
            inline: [0; INLINE_LIMIT],
            heap: vec![1],
            _bound: [(); 33],
        };
        assert_eq!(short.encode(), Err(CodecError::InvalidLength));
        assert_eq!(short.hash_tree_root(), Err(CodecError::InvalidLength));
        let over = ByteList::<0> {
            bytes: vec![1],
            _bound: [],
        };
        assert_eq!(over.encode(), Err(CodecError::OverLimit));
        assert_eq!(over.hash_tree_root(), Err(CodecError::OverLimit));
        assert_eq!(
            byte_error(ByteError::OffsetOverflow),
            CodecError::SizeOverflow
        );
    }

    #[test]
    fn formatting_never_expands_machine_maximum_markers() {
        let value = ByteVector::<{ usize::MAX }> {
            inline: [0; INLINE_LIMIT],
            heap: Vec::new(),
            _bound: [(); usize::MAX],
        };
        assert_eq!(format!("{value:?}"), "ByteVector { bytes: [] }");
        assert_eq!(
            format!(
                "{:?}",
                ByteList::<{ usize::MAX }>::from_bytes(vec![]).unwrap()
            ),
            "ByteList { bytes: [] }"
        );
    }
}
