---
title: Native integer sequence encoding and Merkle packing
last_updated: 2026-10-25
tags:
  - ssz
  - rust
  - performance
  - merkle
---
# Native integer sequence encoding and Merkle packing

Primitive integer sequences now expose their canonical fixed-width bytes through the
provided `Ssz::fixed_sequence_bytes` hook. On little-endian targets, u8 through u128 use
`bytemuck::cast_slice`, whose source alignment is already guaranteed by `&[T]`. The hook
returns a borrowed view and performs no allocation.

Sequence encoding uses the view length directly and copies the complete body once. Basic
sequence tree hashing splits the same view into 32-byte nodes and copies only the final
partial node into zero padding. This removes per-element `encoded_len`/`encode_into` calls
and reduces a 1,024-u64 root from two packing allocations to one chunk-buffer allocation.

External codecs inherit `None` and retain the existing validated element loop. Boolean,
U256 and big-endian targets also retain canonical scalar conversion. A non-default hook is
checked against the declared fixed width and element count before use. The crate continues
to forbid unsafe code.

## Observed timing

2026-10-25, AMD Ryzen 9 PRO 8945HS, Linux CPU 0, five rounds. Times are medians
of per-run means in nanoseconds.

| Operation | Previous nyx-ssz | Current nyx-ssz | Observed speedup |
| --- | ---: | ---: | ---: |
| Mixed encode, 12,304 bytes | 1,154.427 | **364.890** | **3.16×** |
| Mixed root, 1,024 u64 values | 63,721.296 | **36,964.741** | **1.72×** |

Same-batch comparison after the change:

| Operation | nyx-ssz | libssz 0.3.0 | fastssz | nyx/libssz time |
| --- | ---: | ---: | ---: | ---: |
| Mixed encode | **364.890** | 171.381 | 2,179.000 | 2.13× |
| Mixed root | **36,964.741** | 35,098.104 | 43,361.000 | **1.05×** |

The agreed targets were at least a 2× Mixed encode improvement and Mixed root within 1.10×
of libssz; both were met. Header and decode code paths were not changed. Their measured
medians also did not regress relative to the preceding recorded batch, but shared-host load
changed substantially, so cross-batch ratios are observations rather than causal estimates.

- [Before JSON](data/native-integer-decode-after.json), source
  `ad49728d417ade2c3fcfb97abf7714cf94442fb3`.
- [After JSON](data/native-sequence-output-after.json), source
  `3890e95cc2e38323fb7bb006770637be85fa6757`.
- Both reports contain all 22 cases, individual runs, fixtures, tool versions, CPU affinity,
  load averages and the same benchmark harness digest.
- Reproduce with `python3 tools/bench-compare/run.py run --cpu 0 --rounds 5 --out <dir>`.

## Validation and formal status

Workspace tests, documentation tests, Clippy, 32-bit tests, 117 verification-script tests,
allocation assertions and all 4,601 pinned official corpus cases pass locally. The current
Aeneas scope accepted the updated trait inventory and reached Lean dependency compilation;
the bounded local run did not finish the full build. Exact hook, fallback, encoding and
Merkle-packing equivalence therefore remains explicitly unproved under
[#25](https://github.com/NyxFoundation/nyx-ssz/issues/25). No axiom, fallback model, tool fork
or weakened input domain is used.
