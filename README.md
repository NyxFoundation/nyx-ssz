# nyx-ssz

Fast, security-focused SSZ codecs, Merkle trees, typed paths, and proofs for Rust.

nyx-ssz implements the pinned 13-shape SSZ specification with immutable metadata, fail-fast
errors, pinned RustCrypto SHA-256, and one audited native integer copy module as its only
crate-local `unsafe`. It is not yet published to crates.io, formally verified in full, or
presented as a security audit.

## Design philosophy

- **Correctness before benchmark wins:** preserve canonical bytes, error precedence, checked
  arithmetic, and ordinary Rust ownership across every optimized path.
- **General specialization, not fixture-specific code:** derive codecs from static SSZ schemas;
  do not hard-code benchmark types, values, or collection sizes.
- **Safe by default:** keep `unsafe` confined to one audited native integer copy boundary and
  retain portable scalar paths where native assumptions do not hold.
- **Fail-closed evidence:** pin dependencies and corpora, publish reproducible measurements,
  and explicitly mark every operation outside the current Lean refinement slice as unproved.

## Capabilities

- Canonical codecs for scalars, byte/bit collections, Vector/List, Progressive collections,
  Container, ProgressiveContainer, and CompatibleUnion.
- `#[derive(nyx_ssz::Ssz)]`, allocation-aware direct encoding, sparse node access, and
  single/multiproof construction.
- Fail-fast `Result<T, CodecError>` decoding with validated offsets and no partial-value API.
- Complete coverage of 4,601 official cases and 92%+ workspace line coverage.

## Optimization techniques

- Const-evaluated container validation, field widths, and fixed layouts.
- Exact-capacity append encoding without complete-output zero-fills or field-sized temporary
  buffers.
- Generated fixed-container sequence codecs that validate once and write fields directly.
- Borrowed native integer byte views plus one sealed, audited bulk decode copy.
- Stack-based variable-container ranges and fail-fast child decoding.
- Precomputed zero hashes, iterative Merkle frontiers, and direct 32-byte basic-value packing.

The Aeneas pipeline extracts the pinned Rust sources through Charon/Aeneas, kernel-checks the
stated refinement slice in Lean, and rejects undocumented proof gaps.

## Performance snapshot

AMD Ryzen 9 PRO 8945HS, CPU 0, five-round medians; shared-host observations, not guaranteed
speedups:

| Operation | nyx-ssz | libssz 0.3.0 | fastssz |
| --- | ---: | ---: | ---: |
| Header encode | 14.270 ns | 22.692 ns | 52.030 ns |
| Mixed encode, 12 KB | 263.255 ns | 194.898 ns | 1,952 ns |
| Mixed decode, 12 KB | 217.424 ns | 273.705 ns | 9,823 ns |
| Mixed root | 53.490 µs | 48.079 µs | 59.962 µs |

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
