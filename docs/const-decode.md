---
title: Compile-time fixed container decode layouts
last_updated: 2026-10-24
tags:
  - ssz
  - rust
  - performance
---
# Compile-time fixed container decode layouts

Derived fixed-width containers now evaluate declaration validity, field widths and total
wire width in inline constant expressions. Their runtime decode path checks the protocol
size envelope and exact input length, then slices each field at its precomputed width.
It no longer walks the type descriptor or rebuilds the fixed layout for every value.

This document records the source revision measured before the later fail-fast decode API.
That later change also replaces variable-container heap metadata with stack ranges. At the
revision measured here, variable-width and progressive containers retained the validated
view path and partial failure-state contract.

## Observed timing

2026-10-24, AMD Ryzen 9 PRO 8945HS, Linux CPU 0, five rounds. Times are medians
of per-run means in nanoseconds.

| 112-byte Header decode | Before | After | Observed change |
| --- | ---: | ---: | ---: |
| nyx-ssz | 83.637 | 20.301 | 4.12× faster |
| libssz control | 7.240 | 8.550 | 18.1% slower |
| fastssz control | 51.100 | 64.010 | 25.3% slower |

The controls became slower between batches, so the measured nyx-ssz reduction is not
explained by a generally faster host. In the final batch, nyx-ssz was 2.37× slower than
libssz and 3.15× faster than fastssz for this fixture.

The 12,304-byte mixed fixture is variable-width and does not use this fast path. Its
nyx-ssz decode median moved from 480.694 ns to 545.743 ns; no improvement is claimed.
These are shared-host observations, not guaranteed speedups or randomized causal trials.

- [Before JSON](data/small-decode-after.json), source
  `ab8a39385305b26c16bf5d08c844c3b180134af8`.
- [After JSON](data/const-decode-after.json), source
  `ba460e0cab7054debe5530902434043d17a0bd47`.
- Both reports contain all 22 cases, individual runs, fixture bytes and roots, tool
  versions, CPU affinity, load averages and the benchmark harness digest.
- Reproduce with `python3 tools/bench-compare/run.py run --cpu 0 --rounds 5 --out <dir>`.

## Validation

The full workspace test suite, documentation tests, formatting and Clippy with warnings
denied pass. Compile-time regression tests cover exact, variable and overflowing layouts.
At the recorded revision, tests covered generic derived containers, malformed declarations,
partial child failures, owner retention and fixed-layout error precedence.

This optimization is not formally proved. Derived codec equivalence remains tracked in
[#25](https://github.com/NyxFoundation/nyx-ssz/issues/25).
