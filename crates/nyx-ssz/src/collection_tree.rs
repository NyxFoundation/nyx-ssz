//! Collection roots, packing basic elements but rooting composite ones.

use crate::{
    codec::{fixed_size, is_basic, validated_fixed_sequence_bytes, Ssz},
    error::CodecError,
    merkle::{merkleize, merkleize_progressive, MerkleizeError, Node},
    sha256::hash_pair,
    tree_hash::hash_tree_root_u128,
};

struct BasicPacker {
    chunks: Vec<Node>,
    pending: Node,
    filled: usize,
}

impl BasicPacker {
    fn push(&mut self, bytes: &[u8]) {
        let mut offset = 0;
        while offset < bytes.len() {
            let count = (32 - self.filled).min(bytes.len() - offset);
            self.pending[self.filled..self.filled + count]
                .copy_from_slice(&bytes[offset..offset + count]);
            self.filled += count;
            offset += count;
            if self.filled == 32 {
                self.chunks.push(self.pending);
                self.pending = [0; 32];
                self.filled = 0;
            }
        }
    }

    fn finish(mut self) -> Vec<Node> {
        if self.filled != 0 {
            self.chunks.push(self.pending);
        }
        self.chunks
    }
}

fn maximum_encoding<T: Ssz>(values: &[T]) -> Result<Option<usize>, CodecError> {
    let mut maximum = 0;
    let mut oversized = false;
    for value in values {
        match value.encoded_len() {
            Ok(length) => maximum = maximum.max(length),
            Err(CodecError::NativeOutputOverflow) => oversized = true,
            Err(error) => return Err(error),
        }
    }
    Ok(if oversized { None } else { Some(maximum) })
}

fn pack_native_bytes(bytes: &[u8], capacity: usize) -> Vec<Node> {
    let (complete, remainder) = bytes.as_chunks::<32>();
    let mut chunks = Vec::with_capacity(capacity);
    chunks.extend_from_slice(complete);
    if !remainder.is_empty() {
        let mut last = [0; 32];
        last[..remainder.len()].copy_from_slice(remainder);
        chunks.push(last);
    }
    chunks
}

fn pack_basic<T: Ssz>(values: &[T]) -> Result<Vec<Node>, CodecError> {
    let capacity = basic_capacity::<T>(values.len())?;
    if let Some(bytes) = validated_fixed_sequence_bytes(values)? {
        return Ok(pack_native_bytes(bytes, capacity));
    }
    let maximum = maximum_encoding(values)?.ok_or(CodecError::InvalidLength)?;
    // One shared child buffer preserves even externally supplied variable encodings;
    // it does not impose a serialization-wide byte budget on root calculation.
    let mut scratch = vec![0; maximum];
    let mut packer = BasicPacker {
        chunks: Vec::with_capacity(capacity),
        pending: [0; 32],
        filled: 0,
    };
    for value in values {
        let length = value.encoded_len()?;
        let output = scratch.get_mut(..length).ok_or(CodecError::InvalidLength)?;
        value.encode_into(output)?;
        packer.push(output);
    }
    Ok(packer.finish())
}

fn root_elements<T: Ssz>(values: &[T]) -> Result<Vec<Node>, CodecError> {
    let mut roots = Vec::new();
    let mut index = 0;
    while index < values.len() {
        roots.push(values[index].hash_tree_root()?);
        index += 1;
    }
    Ok(roots)
}

fn basic_capacity<T: Ssz>(positions: usize) -> Result<usize, CodecError> {
    match fixed_size::<T>()? {
        Some(width @ (1 | 2 | 4 | 8 | 16 | 32)) => Ok(positions.div_ceil(32 / width)),
        _ => Err(CodecError::InvalidIntegerWidth),
    }
}

pub(crate) fn root_progressive_sequence<T: Ssz>(values: &[T]) -> Result<Node, CodecError> {
    let chunks = if is_basic::<T>() {
        pack_basic(values)?
    } else {
        root_elements(values)?
    };
    let contents = merkleize_progressive(&chunks);
    Ok(hash_pair(
        &contents,
        &hash_tree_root_u128(values.len() as u128),
    ))
}

pub(crate) fn root_sequence<T: Ssz>(
    values: &[T],
    positions: usize,
    mix_length: bool,
) -> Result<Node, CodecError> {
    let (chunks, capacity) = if is_basic::<T>() {
        (pack_basic(values)?, basic_capacity::<T>(positions)?)
    } else {
        (root_elements(values)?, positions)
    };
    let contents = match merkleize(&chunks, Some(capacity)) {
        Ok(root) => root,
        // Only an unlawful child codec can exceed a checked parent's capacity.
        Err(MerkleizeError::LimitTooSmall) => return Err(CodecError::InvalidLength),
    };
    if mix_length {
        Ok(hash_pair(
            &contents,
            &hash_tree_root_u128(values.len() as u128),
        ))
    } else {
        Ok(contents)
    }
}
