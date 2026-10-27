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

1. **Compile-time validation and capacity-based append encode.** Derived containers move
   static type validation into const evaluation. Variable containers compute field sizes once,
   reserve the exact capacity, append their header and bodies directly, and avoid zero-filling
   the complete allocation. Recursively trusted fixed containers use compile-time widths and
   skip per-value length classification. External codecs keep the defensive fallback.
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

## Initial 1–4 measurements

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

## Capacity-append follow-up

The follow-up compares `aa07c0f214aec1ec1f1ddb1252a0c1e574dda3db` with
`4238d53ac6709aad11a78bcbe86a40470924a441` by alternating the two Criterion
executables on CPU 0. Encode uses seven rounds; decode/root use fifteen rounds to expose
shared-host drift. Control-adjusted speedup divides nyx's candidate/baseline ratio by the
same libssz ratio; values above one are faster.

| Fixture / operation | Before | After | Raw speedup | Control-adjusted speedup |
| --- | ---: | ---: | ---: | ---: |
| Header encode | 48.584 | 14.653 | **3.32x** | **3.26x** |
| Header decode | 7.533 | 8.340 | 0.90x | 1.05x |
| Header root | 767.622 | 768.655 | 1.00x | 0.97x |
| Mixed encode | 412.723 | 214.940 | **1.92x** | **2.10x** |
| Mixed decode | 195.670 | 209.916 | 0.93x | 1.15x |
| Mixed root | 42,114.569 | 41,998.948 | 1.00x | 1.02x |

The non-encode raw medians moved with their unchanged libssz controls: for example, mixed
decode's control was 23.8% slower in the candidate rounds. After control adjustment, only
header root regressed, by 2.8%, within the declared 5% guard. Both encode fixtures exceed the
10% target by a wide margin.

A separate ordinary five-round full run of the candidate measured Header encode at 14.332 ns
versus libssz's 21.728 ns, and Mixed encode at 264.829 ns versus 192.718 ns. Mixed decode
remained faster than libssz (226.867 ns versus 292.542 ns).

- [Alternating A/B report](data/libssz-append-ab.json)
- [Final full candidate report](data/libssz-append-after.json)

Reproduce each revision in a clean worktree:

```sh
python3 tools/bench-compare/run.py run --cpu 0 --rounds 5 \
  --out tools/bench-compare/.runs/libssz-optimizations
```

## Correctness and proof boundary

Workspace tests, clippy, 32-bit all-feature tests, the 4,601-case official corpus and the
current-source extraction/kernel gate pass. The fixed-container sequence and append tests check exact bytes, prefix preservation, rollback
on malformed hooks, trusted-layout classification, round trips, one-time validation, unaligned
integer inputs and little-/big-endian behavior. Merkle tests compare the iterative frontier with an independent recursive oracle
across the threshold and verify machine-maximum capacities.

These optimized codec, derive, unsafe-copy and Merkle paths remain explicitly unproved in the
current formal scope. The verification inventory records the new encode hook, and the proof
status documents generated codecs, native integer copies, precomputed zero hashes and the
iterative frontier without claiming new equivalence theorems.
