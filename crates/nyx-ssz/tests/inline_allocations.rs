use nyx_ssz::{bytecollections::ByteVector, codec::Ssz, error::CodecError};
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
fn should_decode_inline_values_without_allocation() {
    let bytes = [7; 32];
    assert_eq!(ByteVector::<32>::decode(&bytes).unwrap().as_bytes(), &bytes);
    let header = Header {
        slot: 1,
        proposer_index: 2,
        parent_root: ByteVector::from_bytes(vec![1; 32]).unwrap(),
        state_root: ByteVector::from_bytes(vec![2; 32]).unwrap(),
        body_root: ByteVector::from_bytes(vec![3; 32]).unwrap(),
    };
    let encoded = header.encode().unwrap();
    let region = Region::new(ALLOCATOR);
    drop(ByteVector::<32>::decode(&bytes).unwrap());
    drop(Header::decode(&encoded).unwrap());
    let inline = region.change();
    assert_eq!(inline.allocations, 0, "{inline:?}");
    assert_eq!(inline.reallocations, 0, "{inline:?}");
    let region = Region::new(ALLOCATOR);
    drop(ByteVector::<33>::decode(&[9; 33]).unwrap());
    let heap = region.change();
    assert_eq!(heap.allocations, 1, "{heap:?}");
    assert_eq!(
        Header::decode(&encoded[..111]).err().unwrap(),
        CodecError::InvalidLength
    );
}
