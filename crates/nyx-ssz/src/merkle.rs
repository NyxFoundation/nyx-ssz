//! Fixed-width SSZ Merkleization and branch verification.
//! Generalized indices use the full u128 range.

use crate::sha256::hash_pair;

/// One complete SSZ Merkle node.
pub type Node = [u8; 32];

include!(concat!(env!("OUT_DIR"), "/zero_hashes.rs"));

mod error;
pub use error::ProofError;

/// A Merkle tree capacity cannot contain all supplied nodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MerkleizeError {
    LimitTooSmall,
}

fn depth_for(count: usize) -> usize {
    if count <= 1 {
        return 0;
    }
    usize::BITS as usize - (count - 1).leading_zeros() as usize
}

fn zero_root(depth: usize) -> Node {
    ZERO_HASHES[depth]
}

fn subtree_root(chunks: &[Node], depth: usize) -> Node {
    if chunks.is_empty() {
        return zero_root(depth);
    }
    if depth == 0 {
        return chunks[0];
    }
    let below = depth - 1;
    let half = 1usize << below;
    let left = subtree_root(&chunks[..chunks.len().min(half)], below);
    let right = if chunks.len() <= half {
        zero_root(below)
    } else {
        subtree_root(&chunks[half..], below)
    };
    hash_pair(&left, &right)
}

fn frontier(chunks: &[Node]) -> [Node; 65] {
    let mut branches = [[0; 32]; 65];
    for (index, chunk) in chunks.iter().enumerate() {
        let mut node = *chunk;
        let mut occupied = index;
        let mut level = 0;
        while occupied & 1 == 1 {
            node = hash_pair(&branches[level], &node);
            occupied >>= 1;
            level += 1;
        }
        branches[level] = node;
    }
    branches
}

fn finish_frontier(branches: &[Node; 65], count: usize, depth: usize) -> Node {
    if depth < usize::BITS as usize && count == 1usize << depth {
        return branches[depth];
    }
    let mut root = ZERO_HASHES[0];
    let mut occupied = count;
    for level in 0..depth {
        root = if occupied & 1 == 1 {
            hash_pair(&branches[level], &root)
        } else {
            hash_pair(&root, &ZERO_HASHES[level])
        };
        occupied >>= 1;
    }
    root
}

/// Merkleize complete nodes, padding to the next power of two of the capacity.
///
/// `None` uses the number of supplied nodes as the capacity. Empty trees and
/// zero capacity are valid. Even `Some(usize::MAX)` does not overflow when
/// rounded up, and no padded tree is allocated.
pub fn merkleize(chunks: &[Node], limit: Option<usize>) -> Result<Node, MerkleizeError> {
    let capacity = match limit {
        Some(capacity) if chunks.len() > capacity => return Err(MerkleizeError::LimitTooSmall),
        Some(capacity) => capacity,
        None => chunks.len(),
    };
    let depth = depth_for(capacity);
    if chunks.is_empty() {
        return Ok(ZERO_HASHES[depth]);
    }
    if chunks.len() <= 16 {
        return Ok(subtree_root(chunks, depth));
    }
    Ok(finish_frontier(&frontier(chunks), chunks.len(), depth))
}

fn progressive_subtree_root(chunks: &[Node], depth: usize, width: u128) -> Node {
    if chunks.is_empty() {
        return [0; 32];
    }
    let count = if width >= chunks.len() as u128 {
        chunks.len()
    } else {
        width as usize
    };
    let left = subtree_root(&chunks[..count], depth);
    let right = if count == chunks.len() {
        [0; 32]
    } else {
        progressive_subtree_root(&chunks[count..], depth + 2, width * 4)
    };
    hash_pair(&left, &right)
}

pub(crate) fn merkleize_progressive(chunks: &[Node]) -> Node {
    progressive_subtree_root(chunks, 0, 1)
}

fn climb(current: Node, proof: &[Node], index: u128) -> Node {
    if proof.is_empty() {
        return current;
    }
    let sibling = proof[0];
    let parent = if index % 2 == 1 {
        hash_pair(&sibling, &current)
    } else {
        hash_pair(&current, &sibling)
    };
    climb(parent, &proof[1..], index >> 1u32)
}

/// Reconstruct a root from a leaf and its bottom-up branch.
///
/// Index zero is invalid; the root itself (index one) has no branch in the
/// reference specification. The branch must contain exactly floor(log2(index))
/// siblings. All node widths are enforced by the array types.
pub fn calculate_merkle_root(leaf: &Node, proof: &[Node], index: u128) -> Result<Node, ProofError> {
    if index == 0 {
        return Err(ProofError::InvalidIndex);
    }
    if index == 1 {
        return Err(ProofError::RootHasNoBranch);
    }
    let depth = proof.len();
    if depth >= 128 {
        return Err(ProofError::InvalidBranchLength);
    }
    if index >> depth != 1 {
        return Err(ProofError::InvalidBranchLength);
    }
    Ok(climb(*leaf, proof, index))
}

/// Verify a branch; malformed requests are errors and mismatched roots are false.
pub fn verify_merkle_proof(
    leaf: &Node,
    proof: &[Node],
    index: u128,
    root: &Node,
) -> Result<bool, ProofError> {
    match calculate_merkle_root(leaf, proof, index) {
        Ok(actual) => Ok(actual == *root),
        Err(error) => Err(error),
    }
}
