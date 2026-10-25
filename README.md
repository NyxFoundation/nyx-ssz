# nyx-ssz

Rust SSZ with canonical codecs, sparse tree access and Merkle proofs.

**Native implementation complete for the pinned 13-shape SSZ specification; formal verification is a separate, unfinished stage.**
Crate publishing remains disabled. This is not a security audit or a claim of performance superiority.

- Boolean and all six unsigned integer widths; owned and borrowed byte/bit collections.
- Vector/List, unbounded and bounded Progressive collections.
- Container, sparse ProgressiveContainer and CompatibleUnion through `#[derive(nyx_ssz::Ssz)]`.
- Type-level field/element paths, packed-element locations, node access and single/multiproof construction.
- All **4,601 cases in all nine families** of the immutable official corpus, with explicit current-version Progressive root alignment and preserved original metadata.
- Host and 32-bit tests; 95.27% workspace line coverage; reproducible Criterion benchmarks.

[Native API and limitations](docs/native-api.md) · [Benchmark results](docs/benchmarks.md) ·
[Current codec comparison](docs/bulk-codecs.md) · [Fail-fast decode](docs/fail-fast-decode.md) ·
[Native integer output](docs/native-sequence-output.md) · [Verification scope](docs/verification.md)

**Current-source Aeneas/Lean verification:** 12 hash-independent gindex/helper-index operations are freshly extracted and kernel checked.
The remaining 105 documented APIs, 17 library codec dictionaries and derive transformations are explicitly **unproved/skipped**, tracked in [#24](https://github.com/NyxFoundation/nyx-ssz/issues/24) and [#25](https://github.com/NyxFoundation/nyx-ssz/issues/25), with the six RustCrypto-dependent skips in [#27](https://github.com/NyxFoundation/nyx-ssz/issues/27).
Aeneas rejects the actual metadata's nested-loop returns. No tool fork, extra axiom or domain restriction was added.
Historical broader artifacts are not current-source certification. See [exact current proof scope](docs/current-verification.md).

`U256` uses pinned `ruint` 1.20.1. Runtime SHA-256 uses pinned RustCrypto
`sha2` 0.10.9 with its standard CPU-dispatched backend, like libssz. No local SHA fallback remains.

Typed encoding now uses `Ssz::encoded_len()` and `Ssz::encode_into(&mut [u8])`.
`encode()` keeps its allocating caller API. **Manual Ssz implementations must migrate**
to the two required methods; derive and library types are updated.
[Direct encoding contract and migration](docs/native-api.md#direct-encoding).
These new member laws are unproved under [#29](https://github.com/NyxFoundation/nyx-ssz/issues/29).
The provided `Ssz::decode_fixed_sequence` hook adds safe unsigned-integer bulk decoding
without requiring manual impl changes; its laws are unproved under
[#32](https://github.com/NyxFoundation/nyx-ssz/issues/32).
`Ssz::decode` now fails directly with `CodecError`; partial decode-state ownership has been
removed, and derived variable containers validate offsets with stack metadata. The breaking
change and measurements are documented in [fail-fast decode](docs/fail-fast-decode.md).
Aligned little-endian primitive integer lists use a safe validated native copy; the recorded
12KB mixed decode is 252.586ns versus libssz's 256.937ns in the same five-round batch.
The provided `Ssz::fixed_sequence_bytes` hook now reuses the same safe native view for
integer sequence encoding and Merkle packing. Mixed encode improved 3.16×, while Mixed root
is within 1.05× of libssz; these hook laws remain unproved under #25.

## Example

```rust
use nyx_ssz::scalar::{decode_bool, encode_bool};
assert_eq!(encode_bool(true), [1]);
assert_eq!(decode_bool(&[1]), Ok(true));
assert!(decode_bool(&[2]).is_err());
let parent = nyx_ssz::sha256::hash_pair(&[0; 32], &[0; 32]);
assert_eq!(parent[0], 0xf5);
assert_eq!(
    nyx_ssz::merkle::verify_merkle_proof(&[0; 32], &[[0; 32]], 2, &parent),
    Ok(true),
);
```

Add this Git dependency to Cargo's `[dependencies]` (the crate is not published):

```toml
nyx-ssz = { git = "https://github.com/NyxFoundation/nyx-ssz" }
```

## Typed codecs

```rust
use nyx_ssz::codec::{decode, encode, fixed_size, hash_tree_root, is_compatible, validate_type, Ssz};
let value = 0x1234u16;
let bytes = encode(&value).unwrap();
assert_eq!(bytes, [0x34, 0x12]);
let mut output = [0; 2];
value.encode_into(&mut output).unwrap();
assert_eq!(value.encoded_len(), Ok(2));
assert_eq!(output, [0x34, 0x12]);
assert_eq!(decode::<u16>(&bytes), Ok(value));
assert_eq!(fixed_size::<u16>(), Ok(Some(2)));
assert_eq!(validate_type::<u16>(), Ok(()));
assert!(!is_compatible::<bool, u8>());
let root = hash_tree_root(&value).unwrap();
assert_eq!(&root[..2], bytes.as_slice());
```

`Ssz::TYPE` is an immutable static declaration. `codec::fixed_size` derives fixed
widths, variable layouts (`Ok(None)`) or unrepresentable widths (`Err(SizeOverflow)`).
`codec::is_basic` derives packing classification; `codec::validate_type` checks the
declaration without invoking a value codec. `codec::is_compatible` compares declared
Merkle layouts but does not imply type validity. These are not independently overridable
metadata constants. The native descriptor now covers all 13 pinned shapes: Boolean,
unsigned integers, byte/bit vectors and lists, ProgressiveBitList, recursively nested
Vector/List/ProgressiveList, Container, ProgressiveContainer and CompatibleUnion.
Separate static arrays retain every name, mask, selector and child, including malformed
counts; names retain full UTF-8 bytes without normalization. No allocation constructs
metadata. All four internal implementations remain const functions. Vector widths use
checked multiplication; an outer zero count cancels even an unrepresentable fixed child
width, whereas a variable child stays variable. Containers sum declared field widths
without adding bytes for gaps; any variable field cancels fixed overflow, in either order.
Compatibility is distinct from equality and validity, with all-pairs union comparison.
These extended metadata behaviors are native-tested, not yet formally integrated.
`ProgressiveList<T>` owns an unbounded sequence, encodes fixed or variable
children using the same canonical list layout, and uses the Progressive tree with the element count.
`CodecError` implements the standard error interfaces. The seven basic implementations
and generic Vector/List/ProgressiveList, owned ByteVector/ByteList, and owned bitfield
implementations have historical proofs; current-source proof status is separate. Borrowed byte/bit codecs remain separately available.

```rust
use nyx_ssz::{codec::decode, error::CodecError};
let error = decode::<u16>(&[0]).unwrap_err();
assert_eq!(error, CodecError::InvalidLength);
```

Decoding is fail-fast and returns `Result<T, CodecError>`. Completed prefixes are
dropped normally when a later child fails; no partial-value recovery state is built or
returned. External implementations use the same error-only contract.

```rust
use nyx_ssz::{codec::{decode, encode, hash_tree_root},
    collections::{progressive_list, ProgressiveList}};
let value = progressive_list(vec![1_u16, 2, 3]);
let bytes = encode(&value).unwrap();
assert_eq!(bytes, [1, 0, 2, 0, 3, 0]);
let decoded = decode::<ProgressiveList<u16>>(&bytes).unwrap();
assert_eq!(decoded.as_slice(), &[1, 2, 3]);
assert_eq!(hash_tree_root(&decoded), hash_tree_root(&value));
```

Construction moves the original allocation without a context bound. Codec calls validate the
child declaration, including for empty values. Nested Progressive/bounded collections compose;
failed decoding drops completed values before returning the first error.

Explicitly bounded Progressive contexts use distinct types; existing unbounded types are unchanged:

```rust
use nyx_ssz::{collections::{bounded_progressive_list, BoundedProgressiveList},
    bitcollections::BoundedProgressiveBitList, codec::{decode, Ssz}};
let values = bounded_progressive_list::<_, 3>(vec![1_u16, 2]).unwrap();
let decoded = decode::<BoundedProgressiveList<u16, 3>>(&values.encode().unwrap()).unwrap();
assert_eq!(decoded.as_slice(), &[1, 2]);
let bits = BoundedProgressiveBitList::<3>::from_ssz_bytes(vec![13]).unwrap();
assert_eq!(bits.len_bits(), 3_u128);
assert_eq!(bits.get(3), None);
```

Both declarations retain `Some(MAX)` for every `MAX`, including zero and `usize::MAX`.
The list constructor returns rejected allocations in `LengthError<T>` without dropping elements.
Bounded decoding checks capacity during layout validation, before child callbacks. Accepted values
have the same bytes and Progressive roots as their unbounded counterparts, never ordinary List roots.
Full dictionary laws also cover arbitrary private storage: encoding enforces capacity, while rooting
matches the pinned reference's cap-independent Progressive layout even on over-cap states.

## Owned packed bitfields

```rust
use nyx_ssz::{bitcollections::{BitList, BitVector, ProgressiveBitList}, codec::{decode, encode}};
let vector = BitVector::<3>::from_bits(&[true, false, true]).unwrap();
assert_eq!(vector.as_ssz_bytes(), &[5]);
let bits = BitList::<8>::from_ssz_bytes(vec![13]).unwrap();
assert_eq!(bits.len_bits(), 3);
assert_eq!(bits.get(2), Some(true));
assert_eq!(bits.get(3), None); // The delimiter is not an element.
let progressive = decode::<ProgressiveBitList>(&[13]).unwrap();
assert_eq!(encode(&progressive), Ok(vec![13]));
```

Owned storage remains packed. `from_ssz_bytes(Vec<u8>)` validates and moves the original allocation;
`from_bits` packs Boolean input. Decoding rejects nonzero vector padding and malformed list delimiters
with distinct `CodecError` categories. Fields and byte projections are immutable. Logical lengths and
indices use `u128`, so a Progressive value never narrows its bit count to `usize`.
Encoding and roots allocate byte/node buffers but never expand packed values into Boolean arrays.
The same official bit-family cases exercise both borrowed APIs and these actual typed dictionaries.

## Owned byte arrays

```rust
use nyx_ssz::{bytecollections::{ByteList, ByteVector}, codec::{decode, encode, hash_tree_root}};
let inline = ByteVector::<3>::from_bytes(vec![0, 128, 255]).unwrap();
assert_eq!(inline.as_bytes(), &[0, 128, 255]);
let mut storage = Vec::with_capacity(33);
storage.extend([7; 33]);
let pointer = storage.as_ptr();
let value = ByteVector::<33>::from_bytes(storage).unwrap();
assert_eq!(value.as_bytes().as_ptr(), pointer);
assert_eq!(encode(&inline).unwrap(), [0, 128, 255]);
let decoded = decode::<ByteList<8>>(&[0, 128, 255]).unwrap();
assert_eq!(decoded.as_bytes(), inline.as_bytes());
assert_eq!(hash_tree_root(&inline).unwrap()[..3], [0, 128, 255]);
```

These dedicated types are not generic `Vector/List<u8>` aliases. A `ByteVector` of at most
32 bytes is copied into the value; larger vectors and every `ByteList` move the accepted
allocation unchanged. Vectors require a nonzero exact length, while lists permit empty values
and zero capacity. `Ssz` decoding of inline vectors does not allocate. Their dedicated byte descriptors are composite for parent packing
and Merkle-compatible with equal-bound uint8 vectors/lists, but retain the pinned opaque-byte
wire semantics: unlike generic sequences, standalone byte arrays have no four-byte sequence-size
envelope. All native bounds and input slices are covered by the proofs, not just allocatable test
examples. Generic parent collections still enforce their own sequence envelopes. Allocation/resource
failures remain outside the existing guarantee.

## Borrowed sequence layout

```rust
use nyx_ssz::sequence::variable_vector_view;
let bytes = [8, 0, 0, 0, 10, 0, 0, 0, 42, 43, 44];
let view = variable_vector_view(&bytes, 2).unwrap();
assert_eq!(view.len(), 2);
assert_eq!(view.get(0), Some(&bytes[8..10]));
assert_eq!(view.get(2), None);
```

This is an internal-layout API, not a complete typed Vector codec. Its caller
validates the type declaration; count zero follows the reference's internal slicing
rule. No element's value or declared limit is checked by this layout-only API.
`nyx_ssz::sequence::variable_list_view(&bytes, Some(capacity))` infers the count from
the first offset and checks that element-count capacity; `None` imposes no declared capacity.
It validates layout only, not each element's own bounds or containing type declaration.

For fixed-width bodies, use `fixed_vector_view(&bytes, width, count)` or
`fixed_list_view(&bytes, width, optional_capacity)`. Both enforce the same composite
size envelope. Vectors require the exact byte budget, including checked multiplication;
lists infer their count and reject widthless nonempty input or incomplete elements
before checking capacity. An empty fixed list has zero elements, even at zero width.
These are layout helpers, not complete typed collection codecs.

## Borrowed Container layout

```rust
use nyx_ssz::container::container_view;
let bytes = [42, 5, 0, 0, 0, 99];
let view = container_view(&bytes, &[Ok(Some(1)), Ok(None)]).unwrap();
assert_eq!(view.get(0), Some(&bytes[..1]));
assert_eq!(view.get(1), Some(&bytes[5..]));
```

Pass child metadata from `codec::fixed_size`: fixed widths are `Ok(Some(width))`,
variable fields are `Ok(None)`, and a fixed natural width exceeding `usize` is
`Err(SizeOverflow)`. Layout rejection follows the pinned `Ssz.structSlices` budget
and offset-check order. Internal zero fields/widths and empty variable bodies are
supported. Bytes are borrowed, not copied; only range/offset metadata is allocated.
The parent codec must validate field names, type declarations and field values.
Ordinary named structs use `#[derive(nyx_ssz::Ssz)]` for complete typed encoding,
decoding and roots. This lower-level helper alone is not an `Ssz` dictionary.
Generated conversion and fail-fast equivalence remains explicitly unproved; see the current scope.

## Typed element decoding

Decode already-sliced bodies with the same fail-fast error contract:

```rust
use nyx_ssz::{codec::decode_elements, error::CodecError, sequence::fixed_vector_view};
let view = fixed_vector_view(&[0, 2], 1, 2).unwrap();
assert_eq!(decode_elements::<bool>(&view), Err(CodecError::InvalidBoolean));
```

This lower-level helper does not choose or validate a containing sequence's type declaration;
the caller selects its layout before decoding elements. An empty view invokes no element decoder.
Completed elements are dropped if a later decoder fails. The input/view remains borrowed and
reusable; no element bytes are copied by the traversal. Typed Vector/List decoding selects and
validates the containing layout before reusing this traversal.

## Typed storage

```rust
use nyx_ssz::collections::{list, vector, List, Vector};
let pair: Vector<u64, 2> = vector(vec![10, 20]).unwrap();
let values: List<_, 4> = list(vec![pair]).unwrap();
assert_eq!(values.as_slice()[0].as_slice(), &[10, 20]);
let rejected = list::<_, 0>(vec![7u8]).unwrap_err();
assert_eq!(rejected.values, vec![7]);
```

Fields are private and projections are immutable, so safe callers cannot resize the
collections. Arbitrary nested/non-`Copy` elements are supported. When `T: Ssz`, both collections
also implement `Ssz`; their formal codec guarantees require the actual child's `CodecLaw`.
Serialization writes fixed values inline or checks and writes four-byte offset tables. Roots pack
basic children or Merkleize composite child roots; lists mix in the actual element count.
Decode failures return completed values and nested child state without dropping them inside the operation.
Allocation identity and absence of element drops are also checked by Rust tests, not runtime-cost theorems.

```rust
use nyx_ssz::{codec::{decode, encode, hash_tree_root}, collections::{list, vector, List, Vector}};
let value = vector::<_, 2>(vec![list::<_, 4>(vec![1u8, 2]).unwrap(), list(vec![3]).unwrap()]).unwrap();
let bytes = encode(&value).unwrap();
let decoded = decode::<Vector<List<u8, 4>, 2>>(&bytes).unwrap();
assert_eq!(decoded.as_slice()[0].as_slice(), &[1, 2]);
assert_eq!(hash_tree_root(&decoded), hash_tree_root(&value));
```

## Packed byte/bit roots

`tree_hash` provides byte/bit vector and list roots, Progressive byte/bit roots, and a
Progressive `BitListView` root without Boolean expansion. Ordinary roots use declared
capacities; Progressive roots use the EIP-7916 right-hand spine. Lists mix the actual
byte/bit count, excluding bitlist delimiters. Empty values and machine-maximum bounds
are supported without allocating padding.

```rust
use nyx_ssz::{bitfield::decode_progressive_bit_list, tree_hash::{
    hash_tree_root_progressive_bit_list, hash_tree_root_progressive_bit_list_view,
}};
let encoded = [0x0d];
let view = decode_progressive_bit_list(&encoded, None).unwrap();
assert_eq!(hash_tree_root_progressive_bit_list_view(&view, None),
           hash_tree_root_progressive_bit_list(&[true, false, true], None));
```

Optional Progressive bounds are context-policy checks: the proved reference is the explicit
composition `Ssz.boundCheck` followed by the unchanged upstream root, which alone ignores
those bounds. No narrower count assumption or generic `CodecLaw` change is made. Progressive
byte roots do not impose a serialization offset envelope. Root functions return `CodecError`.

## Official test vectors

The immutable `ethereum/consensus-spec-tests` commit in `sources.lock` is fetched into
an external cache, not vendored. Its independently pinned SHA-256 inventory and actual Git blob
hashes authenticate the fixtures without trusting cached Git metadata or filters.

```sh
python3 -m venv .venv-vectors
.venv-vectors/bin/pip install -r tools/official-vectors/requirements.txt
.venv-vectors/bin/python scripts/official_vectors.py check
```

All nine families contain 4,601 source cases. Wire bytes and YAML values remain the original
published data. The 2025 corpus predates the official 2026 Progressive-tree orientation fix;
1,744 Progressive and Container root expectations are therefore regenerated from an independently pinned,
authenticated current official reference implementation, not silently compared to obsolete roots.
Original metadata is preserved and paired exactly with each generated expectation. The old corpus
also labels 237 same-type, byte-identical copies of valid Progressive-list values as invalid
(including 30 empty cases). Each is explicitly linked to its published valid witness and tested
for exact value, bytes and current root; no case is skipped and no original file is changed.

Omitting `--family` checks the complete corpus, including both Container families.
Both native Rust CI architectures run the complete selection; unknown schemas fail rather than skip. See [runner provenance and version details](tools/official-vectors/README.md).

## Reproduce

Requires Linux x86_64, Git, Python 3.11+, Rustup/Cargo, Elan, Nix and network access.
Bootstrap builds Aeneas from the exact public source revision in `sources.lock`;
Charon remains pinned independently. The Nix output link retains the built extractor.
The pinned Rust toolchain includes Miri and rust-src. Extraction requires their prepared
standard-library MIR and fails if it is unavailable; generated function bodies are not rewritten.
Proof dependencies are large.

```sh
git clone https://github.com/NyxFoundation/nyx-ssz.git
cd nyx-ssz
cargo test --workspace
cargo install cargo-hax --version 0.4.0 --locked
python3 scripts/verify.py bootstrap
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/current_refinement.py check
```

After edits affecting the current proof slice, run `python3 scripts/current_refinement.py extract`,
review the generated diff, update proofs, then rerun its `check`. Never hand-edit `verification/NyxSszRust.lean`
or the generated `verification/NyxSszRust/` modules.
Success proves only the explicit 12-operation current slice and validates the issue-linked
unproved status of the rest. The legacy full `verify.py check` is not claimed to pass.

MIT; see [third-party notices](THIRD_PARTY_NOTICES.md).
