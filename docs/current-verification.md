---
title: Current-source proof scope and explicit skips
last_updated: 2026-10-25
tags:
  - ssz
  - aeneas
  - formal-verification
---
# Current-source proof scope and explicit skips

Native completion and formal completion are different. The native implementation passes
all 4,601 pinned corpus cases. **Full-library equivalence is not proved.**

## Freshly proved slice

| Current production implementation | Public operations |
| --- | --- |
| Generalized-index arithmetic and paths | 11 |
| Multiproof helper indices | 1 |
| Total | 12 |

`verification/current_gindex.rs` includes the actual production `gindex.rs`,
`merkle/error.rs` and `multiproof/helpers.rs` by Rust path, without copying their code.
The latter two are re-exported by the runtime under the same public `merkle` and
`multiproof` paths. Extraction preserves those public namespaces. Root contents are checked.
Compiled rustdoc identities/source spans bind those public re-exports to the extracted
production files. The error/helper split removes the hashing dependency without a fake hasher.

Charon and Aeneas extract those files with the pinned compiler/MIR sysroot and tools.
Three checked-in generated modules must match fresh extraction byte for byte.
Existing equivalence theorems over these definitions are rechecked by Lean's kernel.
The audit binds every registered operation to its actual theorem and rejects foreign axioms,
native-decision axioms and unrelated theorem registrations. Allowed foundational axioms remain
`propext`, `Classical.choice` and `Quot.sound`, as in the existing audit.

## Explicitly unproved

`verification/current-scope.json` enumerates all **105 remaining documented APIs**
and **17 codec dictionaries**, plus the derive-transformation backlog.
Every documented API must be exactly in the proved or skipped set; additions, stale
registrations, overlaps or missing dictionary status fail the gate. Public re-exports are
resolved to their callable public paths. Hidden generated-code helpers belong to the
unproved derive obligations.

- [#24: metadata extraction](https://github.com/NyxFoundation/nyx-ssz/issues/24):
  `current_metadata.rs` includes the real 13-shape metadata implementation.
  Charon produces LLBC; Aeneas reports **Returns inside of nested loops are not supported yet**
  in `union_options_agree`. Only the recorded tool revisions, source hash, error status and
  diagnostic permit this skip. Success invalidates it; unexpected failures remain failures.
- [#27: RustCrypto hashing](https://github.com/NyxFoundation/nyx-ssz/issues/27):
  runtime `hash_pair` now uses `sha2` 0.10.9's standard CPU-dispatched backend.
  The six formerly checked hash/Merkle reconstruction and verification APIs are unproved.
  Standalone extraction of the actual wrapper exceeded bounded 90-second expanded-dependency
  and 45-second opaque-dependency attempts before producing LLBC. This is not a claim of
  mathematical impossibility or a diagnosed upstream bug. The gate pins the wrapper,
  probe dependency lock and tool revisions; it does not rerun that timed resource probe.
  See `verification/rustcrypto-probe/README.md` for reproduction. No opaque-hash axiom,
  forced portable backend or local runtime fallback is introduced.
- [#29: direct encoding laws](https://github.com/NyxFoundation/nyx-ssz/issues/29):
  required `encoded_len` and `encode_into` methods, the default allocating bridge and
  generated writes are unproved. The member inventory checks real inherited default bodies.
  The full gate rejects the historical `CodecLaw` for extended traits rather than treating
  coverage of encode/decode/root as proof of length or destination behavior.
- [#32: fixed-sequence decode dispatch](https://github.com/NyxFoundation/nyx-ssz/issues/32):
  the provided `decode_fixed_sequence` method, integer overrides and parent dispatch
  are unproved. All added trait members are explicitly inventoried as unproved; the
  historical `CodecLaw` is not treated as coverage of the new behavior.
- [#25: remaining equivalence laws](https://github.com/NyxFoundation/nyx-ssz/issues/25):
  the remaining skips include Cargo/MIR host-tool integration, metadata/codec laws,
  fail-fast decode/drop behavior, generated conversions, typed paths, node access and proof construction.
  They are outstanding obligations, not claims that every operation is individually impossible.
  The stack-range container decoder, table-free fixed decoder and bulk-copy optimizations are
  native-tested changes within that backlog, not newly proved codec equivalence laws. The
  `fixed_sequence_bytes` hook additionally requires exact canonical-byte, width, fallback,
  encode and Merkle-packing agreement; it is inventoried as an unproved trait member.

Historical broader generated artifacts and registrations are not current-source certification.
`HistoricalMerkle.lean` and `HistoricalMultiproof.lean` preserve omitted Aeneas declaration
bodies unchanged, importing the unchanged current error/helper definitions. The old generated
scalar-SHA model remains solely for those historical proofs, never in the runtime or current
proof closure. Unproved/skipped is never counted as proof success.

## Reproduce

Install the pinned tools described by `sources.lock`, then:

```sh
python3 scripts/verify.py bootstrap
python3 scripts/current_refinement.py check
```

The check performs real extraction, freshness/API checks, metadata refusal probing, crypto
skip fingerprint validation, kernel checking, negative audit tests and dependency-pin validation.
`extract` regenerates the three current modules but does not establish proof success.
`lake build` defaults to the current audit. The older `current_merkle.rs` extraction root
includes the hash-dependent program and is not an active passing proof entry point.

A green current-slice CI check means only this bounded proof/status contract passed.
The original `scripts/verify.py check` remains fail closed for the historical full
registration set; it is not advertised as passing against the enlarged library.
