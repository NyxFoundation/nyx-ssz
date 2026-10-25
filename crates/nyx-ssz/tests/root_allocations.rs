use nyx_ssz::{codec::Ssz, collections::list};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn should_allocate_only_the_chunk_buffer_when_rooting_native_basic_values() {
    let value = list::<_, 1024>(vec![7_u64; 1024]).unwrap();
    let region = Region::new(ALLOCATOR);
    let root = value.hash_tree_root().unwrap();
    let stats = region.change();
    std::hint::black_box(root);
    assert_eq!(stats.allocations, 1, "{stats:?}");
    assert_eq!(stats.reallocations, 0, "{stats:?}");
}
