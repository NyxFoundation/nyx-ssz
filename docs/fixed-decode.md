---
title: Fixed-width decode optimization
last_updated: 2026-10-24
tags:
  - ssz
  - rust
  - performance
---
# Fixed-width decode optimization

This is historical measurement for the partial-state API. The current fail-fast API drops
completed prefixes on error while retaining the same wire validation.

This records the initial table-free stage. Newer integer bulk conversion results are in
[bulk codec optimization](bulk-codecs.md).

After merging #30 at `04bb70b`, this change optimizes fixed-width sequence decoding
and owned byte/bit copies. Public APIs and error/ownership types are unchanged.
No encode/root algorithm, unsafe code or dependency is added.

## Implementation and boundaries

Typed Vector/List and bounded/unbounded ProgressiveList share the existing layout
validation rules with public fixed-width views, then walk element slices directly.
They do not allocate a temporary `Vec<BodyRange>`. The public `SequenceView` API
still exposes its existing validated ranges; variable-width offset processing is unchanged.
Decoded byte/bit storage uses standard slice-to-Vec bulk copying.

Complete layout checks precede child callbacks: exact vector length, list alignment,
capacity, zero-width/empty-input rules, unrepresentable fixed widths and the strict
four-byte envelope keep their existing fault precedence. A child failure returns
every completed owner and the refusing child's state without running their destructors.

The output vector is reserved only after first-child success. Its initial capacity hint
is bounded by encoded input size, not by the potentially much larger full Rust representation.
This avoids premature capacity-overflow panic before a later protocol error. For example,
a Boolean wrapper can occupy 512 Rust bytes per one wire byte: reserving a full 4 MiB
input's potential output would exceed the 32-bit Vec allocation range, even if its second
byte should immediately return `InvalidBoolean`. That case is regression tested.
The hint never limits the decode count or successful output: normal growth remains allowed.
Allocator exhaustion and finite execution resources remain outside the correctness claim.

## Observed timing

2026-10-23, AMD Ryzen 9 PRO 8945HS, Linux CPU 0, unchanged tools and three-library
harness. Times are medians of five per-run means, in microseconds.

| Decode fixture | Before | After | Observed speedup |
| --- | ---: | ---: | ---: |
| Header, 112 bytes | 0.5648 | 0.2177 | 2.59× |
| Mixed, 12,304 bytes | 7.0394 | 1.8003 | 3.91× |

Same-batch decode comparison after the change:

| Fixture | nyx-ssz | libssz 0.3.0 | fastssz | nyx/libssz time |
| --- | ---: | ---: | ---: | ---: |
| Header | 0.2177 | 0.0105 | 0.0656 | 20.78× |
| Mixed | 1.8003 | 0.3866 | 11.5910 | 4.66× |

nyx-ssz remains slower than libssz, especially on small values. These are shared-host
observations, not guaranteed speedups or isolated causal estimates: unchanged libssz
decode controls slowed about 13%, and fastssz controls about 18%. Baseline and final
versions ran in separate five-run batches, not randomized/interleaved trials. Encode/root
algorithms were not changed; do not attribute all of their timing shifts to this work.

- [Baseline JSON](../tools/bench-compare/results/2026-10-23-before-fixed-decode.json),
  source `d0b054834e7046f2caf6635618082e8d282f09a9` (the code merged in #30).
- [Final JSON](../tools/bench-compare/results/2026-10-23-fixed-decode.json),
  source `99f5a5162c8a83f93bd52e51321b8e742cd059e9`.
- Both reports contain all 22 cases, every run, identical harness digest, exact fixture
  bytes/roots, tool versions, affinity and load. No pre-capacity-hardening timings are used.
- Reproduce with [the harness](../tools/bench-compare/README.md), `--cpu 0 --rounds 5`.

## Allocation and semantic regression tests

| Case | Before allocations/reallocations | After allocations/reallocations |
| --- | ---: | ---: |
| 1,024 u64 list | 2 / 16 | 1 / 0 |
| Owned byte/bit storage | Geometrically growing copy | 1 / 0 |
| Boolean list refusing its first child | Range table allocated | 0 / 0 |

The u64 test was first observed failing on the old implementation with two allocations
and sixteen reallocations, then passing with only the output allocation. Input creation
is outside the measured region. Child codecs can allocate their own values/states; this
is not a claim of one allocation for arbitrary nested or external dictionaries.

Short Boolean encodings and multibyte layouts are compared with the public view-based
path, including the complete partial result and child-presence status. First/middle/last
callback failures across all four fixed sequence families retain every owner until the
caller drops the failure. Huge metadata widths and virtual layout-budget boundaries are
checked without allocating huge inputs.

## Validation and formal status

All 4,601 pinned official corpus cases pass on the final implementation. Host and 32-bit
workspace tests, Clippy and formatting pass; measured Rust line coverage is 95.32%.
The 12 hash-independent current-source proofs remain freshly extracted and kernel checked.

These changed codec implementations are **not formally proved**. Their equivalence laws
remain in [#25](https://github.com/NyxFoundation/nyx-ssz/issues/25); the direct-encode member
backlog remains [#29](https://github.com/NyxFoundation/nyx-ssz/issues/29). No axiom,
input-domain restriction or weakened fault relation is used to conceal that distinction.
