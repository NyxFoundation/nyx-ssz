---
title: Fixed container encode optimization
last_updated: 2026-10-25
tags:
  - benchmarks
  - codecs
  - performance
---
# Fixed container encode optimization

All-fixed, non-progressive derived containers now validate every child in order and write
field bytes directly. They do not construct offset bookkeeping. A later protocol error still
wins over an earlier deferred native overflow, and allocating `encode()` creates exactly one
output buffer.

The derive uses compile-time fixed-layout metadata to select this path. Variable containers
bypass fixed-child classification and retain the existing `ContainerSize`/`ContainerWriter`
implementation without duplicate validation. Progressive containers also retain that path.
`ByteVector<N>` for `N <= 32` copies directly from inline storage.

These changed laws remain unproved under
[#25](https://github.com/NyxFoundation/nyx-ssz/issues/25) and
[#29](https://github.com/NyxFoundation/nyx-ssz/issues/29).

## Measurement

2026-10-25, AMD Ryzen 9 PRO 8945HS, Linux CPU 0, five rounds. Numbers are medians
of per-run means in nanoseconds.

| Operation | Current main before | PR #35 | Observed change | libssz in PR batch |
| --- | ---: | ---: | ---: | ---: |
| Header encode | 103.580 | **45.854** | **2.26× faster** | 15.479 |
| Mixed encode | 364.890 | **327.242** | 1.11× faster | 151.958 |
| Header decode | 7.290 | 6.901 | no regression | 6.607 |
| Mixed decode | 211.226 | 221.596 | 4.9% slower | 199.537 |
| Header root | 736.183 | 704.687 | no regression | 493.302 |
| Mixed root | 36,964.741 | 36,203.902 | no regression | 33,567.703 |

Only Header encode uses the new fixed-container path in these fixtures. Mixed encode is
variable-layout and its generated fallback is behaviorally the same as current main; the
recorded change is not attributed to this optimization. Decode and root are unchanged code
paths. Shared-host measurements are not randomized causal trials.

- [Before JSON](data/native-sequence-output-after.json), source
  `3890e95cc2e38323fb7bb006770637be85fa6757`.
- [After JSON](data/fixed-encode-after.json), source
  `7e314ace12fcd641b7f8ca97f4b557417c5f51b0`.
- Both reports contain all 22 cases, individual runs, fixtures, tool versions, CPU affinity,
  load averages and the same benchmark harness digest.
- Reproduce with `python3 tools/bench-compare/run.py run --cpu 0 --rounds 5 --out <dir>`.

## Validation

Workspace tests, documentation tests, Clippy, 32-bit tests, 117 verification-script tests,
allocation assertions and all 4,601 pinned official corpus cases pass. The current Aeneas
slice remains separate from these unproved generated encoding laws.
