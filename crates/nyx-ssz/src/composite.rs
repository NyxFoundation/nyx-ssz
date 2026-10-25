//! Shared implementation for generated heterogeneous codecs.
use crate::{
    codec::Ssz,
    error::CodecError,
    merkle::{self, Node},
    sha256::hash_pair,
};

/// Root fields at ordinary contiguous or progressive sparse positions.
pub fn root_fields(roots: &[Node], active: Option<&[bool]>) -> Result<Node, CodecError> {
    let Some(active) = active else {
        return merkle::merkleize(roots, Some(roots.len())).map_err(|_| CodecError::InvalidLength);
    };
    if active.len() > 256 || active.iter().filter(|&&bit| bit).count() != roots.len() {
        return Err(CodecError::LayoutFieldCount);
    }
    let mut slots = Vec::with_capacity(active.len());
    let mut word = [0; 32];
    let mut ordinal = 0;
    for (index, &occupied) in active.iter().enumerate() {
        slots.push(if occupied {
            word[index / 8] |= 1 << (index % 8);
            let node = roots[ordinal];
            ordinal += 1;
            node
        } else {
            [0; 32]
        });
    }
    Ok(hash_pair(&merkle::merkleize_progressive(&slots), &word))
}

pub fn root_union<T: Ssz>(selector: u8, value: &T) -> Result<Node, CodecError> {
    let root = value.hash_tree_root()?;
    let mut word = [0; 32];
    word[0] = selector;
    Ok(hash_pair(&root, &word))
}
