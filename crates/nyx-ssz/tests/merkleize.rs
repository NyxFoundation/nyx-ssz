use nyx_ssz::merkle::{merkleize, MerkleizeError, Node};
use nyx_ssz::sha256::hash_pair;

fn zero_root(depth: usize) -> Node {
    (0..depth).fold([0; 32], |node, _| hash_pair(&node, &node))
}

fn reference_root(chunks: &[Node], depth: usize) -> Node {
    if chunks.is_empty() {
        return zero_root(depth);
    }
    if depth == 0 {
        return chunks[0];
    }
    let below = depth - 1;
    let half = 1usize << below;
    let left = reference_root(&chunks[..chunks.len().min(half)], below);
    let right = if chunks.len() <= half {
        zero_root(below)
    } else {
        reference_root(&chunks[half..], below)
    };
    hash_pair(&left, &right)
}

#[test]
fn should_return_the_zero_node_when_no_leaves_or_capacity_are_present() {
    assert_eq!(merkleize(&[], None), Ok([0; 32]));
    assert_eq!(merkleize(&[], Some(0)), Ok([0; 32]));
    assert_eq!(merkleize(&[], Some(1)), Ok([0; 32]));
}

#[test]
fn should_return_the_leaf_unchanged_when_one_node_fills_the_tree() {
    let leaf = [42; 32];
    assert_eq!(merkleize(&[leaf], None), Ok(leaf));
    assert_eq!(merkleize(&[leaf], Some(1)), Ok(leaf));
}

#[test]
fn should_reject_a_capacity_that_would_drop_any_supplied_node() {
    assert_eq!(
        merkleize(&[[1; 32]], Some(0)),
        Err(MerkleizeError::LimitTooSmall)
    );
    assert_eq!(
        merkleize(&[[1; 32], [2; 32]], Some(1)),
        Err(MerkleizeError::LimitTooSmall)
    );
}

#[test]
fn should_hash_siblings_in_order_when_two_nodes_fill_the_tree() {
    let left = [1; 32];
    let right = [2; 32];
    assert_eq!(
        merkleize(&[left, right], None),
        Ok(hash_pair(&left, &right))
    );
}

#[test]
fn should_zero_pad_an_odd_node_count_to_the_next_power_of_two() {
    let nodes = [[1; 32], [2; 32], [3; 32]];
    let expected = hash_pair(
        &hash_pair(&nodes[0], &nodes[1]),
        &hash_pair(&nodes[2], &[0; 32]),
    );
    for limit in [None, Some(3), Some(4)] {
        assert_eq!(merkleize(&nodes, limit), Ok(expected));
    }
    assert_eq!(
        merkleize(&[nodes[0], nodes[1], nodes[2], [0; 32]], None),
        Ok(expected)
    );
}

#[test]
fn should_respect_declared_capacity_when_a_small_value_occupies_a_deeper_tree() {
    let leaf = [7; 32];
    let expected = hash_pair(&hash_pair(&leaf, &[0; 32]), &zero_root(1));
    assert_eq!(merkleize(&[leaf], Some(3)), Ok(expected));
    assert_eq!(merkleize(&[], Some(3)), Ok(zero_root(2)));
}

#[test]
fn should_match_recursive_merkleization_across_the_iterative_frontier_boundary() {
    for count in [17_usize, 31, 32, 33, 100] {
        let chunks: Vec<Node> = (0..count).map(|value| [value as u8; 32]).collect();
        for capacity in [count, count + 1, count.next_power_of_two()] {
            let depth = capacity.next_power_of_two().trailing_zeros() as usize;
            assert_eq!(
                merkleize(&chunks, Some(capacity)),
                Ok(reference_root(&chunks, depth))
            );
        }
    }
}

#[test]
fn should_handle_the_machine_maximum_capacity_without_rounding_overflow() {
    assert_eq!(
        merkleize(&[], Some(usize::MAX)),
        Ok(zero_root(usize::BITS as usize))
    );
    let leaf = [9; 32];
    let expected =
        (0..usize::BITS as usize).fold(leaf, |root, depth| hash_pair(&root, &zero_root(depth)));
    assert_eq!(merkleize(&[leaf], Some(usize::MAX)), Ok(expected));
}
