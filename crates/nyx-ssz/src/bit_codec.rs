//! Owned packed bitfield dictionaries, with complete logical observations.
//!
//! Private stored bytes are canonically re-encoded rather than assumed valid in
//! the value law. Safe constructors and decoders only produce canonical storage.

use crate::{
    bit_storage,
    bitcollections::{BitList, BitVector, BoundedProgressiveBitList, ProgressiveBitList},
    bitfield,
    codec::Ssz,
    error::CodecError,
    tree_hash,
    type_info::TypeDescriptor,
};

impl<const N: usize> Ssz for BitVector<N> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::BitVector(N);

    fn encoded_len(&self) -> Result<usize, CodecError> {
        crate::bit_encoding::vector_len(self.as_ssz_bytes(), N)
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        crate::bit_encoding::write_vector(self.as_ssz_bytes(), N, output)
    }

    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        Self::from_ssz_bytes(bytes.to_vec())
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        let encoded = self.encode()?;
        tree_hash::hash_tree_root_byte_vector(&encoded, N.div_ceil(8))
    }
}

impl<const MAX: usize> Ssz for BitList<MAX> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::BitList(MAX);

    fn encoded_len(&self) -> Result<usize, CodecError> {
        crate::bit_encoding::list_len(self.as_ssz_bytes(), Some(MAX))
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        crate::bit_encoding::write_list(self.as_ssz_bytes(), Some(MAX), output)
    }

    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        Self::from_ssz_bytes(bytes.to_vec())
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        let encoded = self.encode()?;
        bit_storage::root_bounded_list(&encoded, MAX)
    }
}

impl<const MAX: usize> Ssz for BoundedProgressiveBitList<MAX> {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::ProgressiveBitList(Some(MAX));

    fn encoded_len(&self) -> Result<usize, CodecError> {
        crate::bit_encoding::list_len(self.as_ssz_bytes(), Some(MAX))
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        crate::bit_encoding::write_list(self.as_ssz_bytes(), Some(MAX), output)
    }

    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        Self::from_ssz_bytes(bytes.to_vec())
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        // Rooting ignores context caps in the pinned reference. Normalize every
        // private logical state without imposing the serialization capacity.
        let encoded = bit_storage::normalize_list(self.as_ssz_bytes(), None)?;
        let view = match bitfield::decode_progressive_bit_list(&encoded, None) {
            Ok(view) => view,
            Err(error) => return Err(bit_storage::fault(error)),
        };
        tree_hash::hash_tree_root_progressive_bit_list_view(&view, None)
    }
}

impl Ssz for ProgressiveBitList {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::ProgressiveBitList(None);

    fn encoded_len(&self) -> Result<usize, CodecError> {
        crate::bit_encoding::list_len(self.as_ssz_bytes(), None)
    }
    fn encode_into(&self, output: &mut [u8]) -> Result<(), CodecError> {
        crate::bit_encoding::write_list(self.as_ssz_bytes(), None, output)
    }

    fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        Self::from_ssz_bytes(bytes.to_vec())
    }

    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> {
        let encoded = self.encode()?;
        let view = match bitfield::decode_progressive_bit_list(&encoded, None) {
            Ok(view) => view,
            Err(error) => return Err(bit_storage::fault(error)),
        };
        tree_hash::hash_tree_root_progressive_bit_list_view(&view, None)
    }
}
