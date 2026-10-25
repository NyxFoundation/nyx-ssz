use nyx_ssz::{
    bitcollections::BitVector, bytecollections::ByteList, codec::Ssz, collections::List,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(nyx_ssz::Ssz)]
struct Mixed {
    slot: u64,
    payload: ByteList<4096>,
    votes: List<u64, 1024>,
}

#[test]
fn should_allocate_only_owned_outputs_when_decoding() {
    let mut bytes = vec![0; 8192];
    for chunk in bytes.as_chunks_mut::<8>().0 {
        chunk.copy_from_slice(&7_u64.to_le_bytes());
    }
    let region = Region::new(ALLOCATOR);
    let decoded = List::<u64, 1024>::decode(&bytes).unwrap();
    let stats = region.change();
    assert_eq!(decoded.as_slice(), &[7; 1024]);
    assert_eq!(stats.allocations, 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");

    let payload = vec![7; 4096];
    let region = Region::new(ALLOCATOR);
    let decoded = ByteList::<4096>::decode(&payload).unwrap();
    let stats = region.change();
    assert_eq!(decoded.as_bytes(), payload);
    assert_eq!(stats.allocations, 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");

    let bits = vec![0; 1024];
    let region = Region::new(ALLOCATOR);
    let decoded = BitVector::<8192>::decode(&bits).unwrap();
    let stats = region.change();
    assert_eq!(decoded.as_ssz_bytes(), bits);
    assert_eq!(stats.allocations, 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");

    let region = Region::new(ALLOCATOR);
    let error = List::<bool, 1024>::decode(&[2, 1]).unwrap_err();
    let stats = region.change();
    assert_eq!(error, nyx_ssz::error::CodecError::InvalidBoolean);
    assert_eq!(stats.allocations, 0, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");

    let mut mixed = vec![0; 12_304];
    mixed[..8].copy_from_slice(&42_u64.to_le_bytes());
    mixed[8..12].copy_from_slice(&16_u32.to_le_bytes());
    mixed[12..16].copy_from_slice(&4_112_u32.to_le_bytes());
    mixed[16..4_112].fill(7);
    for chunk in mixed[4_112..].as_chunks_mut::<8>().0 {
        chunk.copy_from_slice(&1_u64.to_le_bytes());
    }
    let region = Region::new(ALLOCATOR);
    let decoded = Mixed::decode(&mixed).unwrap();
    let stats = region.change();
    assert_eq!(decoded.slot, 42);
    assert_eq!(decoded.payload.as_bytes(), &[7; 4096]);
    assert_eq!(decoded.votes.as_slice(), &[1; 1024]);
    assert_eq!(stats.allocations, 2, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}
