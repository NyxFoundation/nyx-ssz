# nyx-ssz

Fast, security-focused SSZ codecs, Merkle trees, typed paths, and proofs for Rust.

nyx-ssz implements the pinned 13-shape SSZ specification with fail-fast errors, one audited
native integer copy module as its only crate-local `unsafe`, pinned RustCrypto SHA-256,
immutable metadata, and complete coverage of all **4,601**
official corpus cases, with native tests across x86_64, ARM64, and 32-bit CI. It is not yet published to crates.io,
formally verified in full, or presented as a security audit.

## Why nyx-ssz

- Canonical codecs for scalars, byte/bit collections, Vector/List, Progressive collections,
  Container, ProgressiveContainer, and CompatibleUnion.
- `#[derive(nyx_ssz::Ssz)]`, allocation-aware direct encoding, sparse node access, and
  single/multiproof construction.
- Fail-fast `Result<T, CodecError>` decoding with validated offsets, checked arithmetic, and
  ordinary Rust drop behavior.
- 92%+ workspace line coverage, reproducible benchmarks, pinned dependencies, and explicit
  proof-status tracking for every public operation.

**Optimization method:** nyx-ssz combines compile-time layouts, single-pass destination writes, generated fixed-container bulk codecs, audited native integer copies, precomputed zero hashes, and direct 32-byte Merkle packing to remove redundant scans, temporary buffers, and allocator traffic.

**Aeneas method:** the verification pipeline extracts the actual pinned Rust sources through Charon/Aeneas, kernel-checks the declared refinement slice in Lean, and fails closed unless every remaining API is explicitly linked to an unproved issue.

## Performance snapshot

AMD Ryzen 9 PRO 8945HS, CPU 0, five-round medians; shared-host observations, not guaranteed
speedups:

| Operation | nyx-ssz | libssz 0.3.0 | fastssz |
| --- | ---: | ---: | ---: |
| Header encode | 46.454 ns | 22.748 ns | 53.220 ns |
| Mixed encode, 12 KB | 409.144 ns | 233.949 ns | 2,167 ns |
| Mixed decode, 12 KB | 252.279 ns | 319.613 ns | 10,026 ns |
| Mixed root | 56.401 µs | 49.994 µs | 63.766 µs |

Full runs and provenance: [benchmarks](docs/benchmarks.md),
[libssz-inspired optimizations](docs/libssz-optimizations.md), and
[native integer output](docs/native-sequence-output.md).

## Install

The crate is currently consumed from Git:

```toml
[dependencies]
nyx-ssz = { git = "https://github.com/NyxFoundation/nyx-ssz" }
```

## Example

```rust
use nyx_ssz::codec::{decode, encode, hash_tree_root, Ssz};

let value = 0x1234_u16;
let bytes = encode(&value)?;
assert_eq!(bytes, [0x34, 0x12]);
assert_eq!(decode::<u16>(&bytes)?, value);
assert_eq!(&hash_tree_root(&value)?[..2], bytes.as_slice());
# Ok::<(), nyx_ssz::error::CodecError>(())
```

## Verification status

The current-source pipeline freshly extracts and Lean-kernel-checks 12 hash-independent
index operations; the remaining 105 public APIs, 17 codec dictionaries, derive transformations,
and RustCrypto-dependent laws are explicitly unproved under
[#24](https://github.com/NyxFoundation/nyx-ssz/issues/24),
[#25](https://github.com/NyxFoundation/nyx-ssz/issues/25), and
[#27](https://github.com/NyxFoundation/nyx-ssz/issues/27), without extra axioms or weakened domains.

See [native API](docs/native-api.md), [exact proof scope](docs/current-verification.md), and
[verification design](docs/verification.md).

## Validate

```sh
cargo test --workspace
python3 scripts/official_vectors.py check
python3 scripts/current_refinement.py check
```

MIT; see [third-party notices](THIRD_PARTY_NOTICES.md).
