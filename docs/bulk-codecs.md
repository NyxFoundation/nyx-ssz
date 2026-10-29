---
title: Bulk codec optimization
last_updated: 2026-10-29
tags:
  - benchmarks
  - codecs
  - ownership
---
# Bulk codec optimization

Three implementation stages address the measured large-sequence bottlenecks without
changing SHA-256, Merkle algorithms, wire bytes, roots or crate-local unsafe policy.
Native correctness tests are not formal certification.

## Changes

1. **Root packing:** replace `Vec<Vec<u8>>` and per-byte assembly with one shared
   child buffer and block copies into 32-byte chunks. Reserve the ordinary basic
   chunk count once. Even external encodings crossing chunk boundaries are handled;
   no declared-width assumption is used for copying. Child validation still defers
   native unrepresentability behind later protocol errors. There is no whole-root
   four-byte serialization budget.
2. **Encode lengths:** sum child lengths exactly in u128 instead of clamping after
   every child. On supported 32/64-bit targets, at most usize::MAX lengths each at
   most usize::MAX fit in u128. Check both operands before their parent-envelope
   addition. Every child is still validated in order. Constant-width loops can now
   fold into arithmetic, including repeated length queries in allocating encode.
3. **Integer decode:** the provided `Ssz::decode_fixed_sequence` method preserves
   the generic callback/owned-prefix algorithm. Unsigned integer implementations
   use safe fixed-byte arrays, `from_le_bytes`, and exact-size collection. Boolean
   and external implementations are not reinterpreted based on metadata. Parents
   preflight count/capacity/layout before dispatch and retain every returned owner
   when rejecting a malformed override's success count.

The extra method requires no changes to existing manual implementations. See the
[public contract](native-api.md#fixed-sequence-decode-hook).

## Allocation and generated-code checks

The dedicated root test excludes construction of its 1,024-element u64 List:

| Root allocation counter | Before | After |
|---|---:|---:|
| Allocations | 1,026 | 2 |
| Reallocations | 14 | 0 |

Both remaining allocations are shared packing buffers, not one allocation per
integer. `root_allocations.rs` asserts the after counts. Other tests cover chunk
crossings, empty external encodings and native/protocol error precedence.

The actual measured Criterion executable's disassembly is retained in
[data/bulk-codecs-codegen.txt](data/bulk-codecs-codegen.txt), with its source and
binary hashes. The List length calculation uses a shift by three, not the previous
per-element clamp loop. Safe u64 collection uses two unaligned 16-byte load/store
pairs per four values, followed by a scalar tail. This is evidence for the measured
x86_64 compiler, not a promise of particular instructions on every target.

## Same-workload measurements

Before: `adec23ec0df2005813c5d7b690af454c4c70f8b5` (main before these changes).
After: `7e6716e08cebb8f3b11ead4df2facdff1a117da8` (all three runtime stages).
Later report/contract/test edits do not change the measured runtime bodies.
The unchanged harness hash is
`eac4409d171da42357d821f20e77af5846beecc57ab6459e8bd1915d9dc40dd9`.
Both reports contain byte-identical fixture wires, hashes and roots, the same 22
cases, CPU 0, five rounds, and the same pinned library/compiler versions.

All numbers below are **nanoseconds per operation**, median of five per-run means.
Speedup is before/after; below 1 means slower. Allocate-return encode is compared
with allocate-return encode, not the controls' separate reuse-buffer cases.

| Fixture / operation | nyx before | nyx after | Speedup | libssz after | fastssz after |
|---|---:|---:|---:|---:|---:|
| Header encode | 156.53 | 104.01 | 1.51x | 17.53 | 35.78 |
| Header decode | 137.79 | 153.85 | 0.90x | 7.23 | 47.40 |
| Header root | 752.79 | 800.78 | 0.94x | 519.45 | 678.10 |
| Mixed encode | 1,257.25 | 408.71 | **3.08x** | 153.71 | 1,591.00 |
| Mixed decode | 1,237.88 | 379.64 | **3.26x** | 216.50 | 7,494.00 |
| Mixed root | 70,654.91 | 43,841.64 | **1.61x** | 34,506.83 | 42,963.00 |

Mixed is the unchanged 12,304-byte Container with 1,024 u64 values and 4,096 payload
bytes. Header is 112 bytes. Large-case wins do **not** imply universal superiority:
nyx remains 2.66x/1.75x/1.27x libssz's mixed encode/decode/root latency. Small-header
decode and root did not improve in this run (11.7% and 6.4% slower respectively).
No further small-type/metadata optimization is included in this change.

Timing is noisy and the machine was not exclusively reserved. Across unchanged
control cases, the geometric-mean latency was 0.7% lower for libssz and 7.3% lower
for fastssz in the final after batch. An initial after batch had substantially
slower controls (13.5%/26.2%) and was repeated once after the before batch; its full
raw report is also retained, not silently discarded. Read per-run ranges before
interpreting small differences; these are local observations, not portable ratios.

- [Before: all runs and provenance](data/bulk-codecs-before.json)
- [Final after: all runs and provenance](data/bulk-codecs-after.json)
- [Initial after: all runs and provenance](data/bulk-codecs-first-after.json)

Reproduce on each revision in its own worktree:

```sh
python3 tools/bench-compare/run.py run --cpu 0 --rounds 5 \
  --out tools/bench-compare/.runs/bulk-codecs
cargo test -p nyx-ssz --test root_allocations
```

## Correctness and proof boundary

Workspace tests and clippy pass on the host; all-feature tests also pass on
`i686-unknown-linux-musl`. The full official nine-family run verifies 4,601 cases
with version-aligned roots and unchanged original witness metadata. Workspace line
coverage is 95.27% (80% gate). Tests additionally check every unsigned width,
unaligned little-endian input, invalid Boolean prefixes, layout/capacity precedence,
wrong-count override ownership, malformed metadata and unrepresentable fixed widths.

Current-source Aeneas extraction and Lean kernel checking now prove 24
hash-independent index and scalar operations. The other 93 APIs, 17 dictionaries and derive
transformations remain unproved/skipped. The new decode member and integer-override
laws are explicitly unproved under [#32](https://github.com/NyxFoundation/nyx-ssz/issues/32);
encode member laws remain [#29](https://github.com/NyxFoundation/nyx-ssz/issues/29).
The historical full gate still rejects extended API/extraction coverage. No axiom,
input-domain restriction, tool fork or new formal-proof success is claimed.
