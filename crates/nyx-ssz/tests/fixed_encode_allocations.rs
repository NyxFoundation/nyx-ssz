use nyx_ssz::{bytecollections::ByteVector, codec::Ssz};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(nyx_ssz::Ssz)]
struct Header {
    slot: u64,
    proposer_index: u64,
    parent_root: ByteVector<32>,
    state_root: ByteVector<32>,
    body_root: ByteVector<32>,
}

#[test]
fn should_allocate_only_the_output_when_encoding_a_fixed_header() {
    let value = Header {
        slot: 1,
        proposer_index: 2,
        parent_root: ByteVector::from_bytes(vec![1; 32]).unwrap(),
        state_root: ByteVector::from_bytes(vec![2; 32]).unwrap(),
        body_root: ByteVector::from_bytes(vec![3; 32]).unwrap(),
    };
    let region = Region::new(ALLOCATOR);
    let bytes = value.encode().unwrap();
    let stats = region.change();
    std::hint::black_box(bytes);
    assert_eq!(stats.allocations, 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}
