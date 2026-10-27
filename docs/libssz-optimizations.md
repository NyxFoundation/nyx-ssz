---
title: libssz-inspired codec and Merkle optimizations
last_updated: 2026-10-27
tags:
  - benchmarks
  - codecs
  - merkle-trees
  - performance
---
# libssz-inspired codec and Merkle optimizations

This change applies four techniques observed in libssz-style high-throughput paths while
preserving nyx-ssz's checked public contracts and fail-fast error precedence.

## Implemented stages

1. **Compile-time generated validation and one-allocation encode.** Derived containers move
   static type validation into const evaluation. Their allocating fallback now computes field
   sizes once, allocates the exact destination, and writes directly instead of calling
   `encoded_len` and then repeating the same traversal through `encode_into`.
2. **Generated fixed-container sequence codecs.** The derive macro emits bulk encode/decode
   hooks for ordinary fixed-width containers. A parent sequence validates its declaration and
   destination once, then slices each field directly without rediscovering the container layout
   for every element. Variable and progressive containers retain the generic path.
3. **Audited native integer decode copy.** On little-endian targets, fixed `u8` through `u128`
   sequences reserve their exact output and copy the validated byte range into initialized
   integer storage. One sealed `NativeInteger` trait limits the unsafe block to integer types
   whose layouts are statically asserted. Big-endian targets retain `from_le_bytes` conversion.
4. **Precomputed zero hashes and iterative Merkle frontier.** The build script generates all 65
   canonical zero roots once. Merkleization uses an iterative frontier for more than 16 chunks,
   while the small recursive path remains for low-overhead tiny trees. No padded tree is built.

The new `Ssz::encode_fixed_sequence` hook is optional. Existing manual implementations continue
using the generic element loop. Its safety and canonical-byte obligations are documented in the
[native API contract](native-api.md#fixed-sequence-encode-hook).

## Same-harness measurements

Before: `ae9d9b4d1dec8fe3e1d50cc54316c4eb5271c253` (`origin/main`).
After: `9b04686170280b5814e37da723feccb585748401` (runtime implementation commit).
Both reports use harness hash
`148a17a585d1fd89fb8e763a6d3d03cea6adeacef882e6f976921baabc826431`, CPU 0,
five rounds, and median per-run means. Values are nanoseconds per operation.

| Fixture / operation | Before | After | Before / after | libssz after |
| --- | ---: | ---: | ---: | ---: |
| Header encode | 76.162 | 47.542 | **1.60x** | 24.367 |
| Header decode | 10.743 | 10.635 | 1.01x | 10.171 |
| Header root | 1,091.197 | 1,007.660 | 1.08x | 751.328 |
| Mixed encode | 464.661 | 394.347 | **1.18x** | 224.403 |
| Mixed decode | 241.544 | 288.852 | 0.84x | 330.456 |
| Mixed root | 55,348.166 | 56,054.261 | 0.99x | 51,229.901 |

The unchanged controls indicate normal shared-host noise: libssz's six common cases were 2.8%
slower by geometric mean in the after batch, while fastssz was 1.0% faster. The
current harness directly demonstrates the derived allocating-encode improvement. It does not
isolate lists of derived fixed containers, the integer-only copy, or sparse Merkle frontiers;
those paths are protected by focused behavior, allocation, cross-endian, and recursive-oracle
tests rather than a claimed microbenchmark speedup.

- [Before report and full provenance](data/libssz-optimizations-before.json)
- [After report and full provenance](data/libssz-optimizations-after.json)

Reproduce each revision in a clean worktree:

```sh
python3 tools/bench-compare/run.py run --cpu 0 --rounds 5 \
  --out tools/bench-compare/.runs/libssz-optimizations
```

## Correctness and proof boundary

Workspace tests, clippy, 32-bit all-feature tests, the 4,601-case official corpus and the
current-source extraction/kernel gate pass. The fixed-container sequence tests check exact
bytes, round trips, one-time validation, unaligned integer inputs and little-/big-endian
behavior. Merkle tests compare the iterative frontier with an independent recursive oracle
across the threshold and verify machine-maximum capacities.

These optimized codec, derive, unsafe-copy and Merkle paths remain explicitly unproved in the
current formal scope. The verification inventory records the new encode hook, and the proof
status documents generated codecs, native integer copies, precomputed zero hashes and the
iterative frontier without claiming new equivalence theorems.
