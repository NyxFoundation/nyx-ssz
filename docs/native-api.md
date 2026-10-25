---
title: Native SSZ API
last_updated: 2026-10-25
tags:
  - ssz
  - rust
  - api
---
# Native SSZ API

Native completion and formal equivalence are separate. All 13 shapes in the pinned
specification have value codecs. Current-source proof status is tracked separately;
neither implementing nor deriving `Ssz` certifies a codec.

## Derive

```rust
use nyx_ssz::{codec::Ssz, collections::List, path::Step};

#[derive(nyx_ssz::Ssz)]
struct Message {
    slot: u64,
    votes: List<u16, 16>,
}
let value = Message {
    slot: 42,
    votes: nyx_ssz::collections::list(vec![1, 2]).unwrap(),
};
let bytes = value.encode().unwrap();
let decoded = Message::decode(&bytes).unwrap();
assert_eq!(decoded.encode().unwrap(), bytes);
let branch = nyx_ssz::proof::at_path(&value, &[Step::Field("votes"), Step::Length]).unwrap();
assert!(nyx_ssz::merkle::verify_merkle_proof(
    &branch.leaf, &branch.siblings, branch.index, &value.hash_tree_root().unwrap()
).unwrap());
```

Named structs are ordinary Containers. Generic field types acquire codec bounds.
Tuple/unit/empty structs are rejected. Width and packing metadata come from child declarations.

Add `#[ssz(progressive)]` to a named struct and require `#[ssz(position = N)]`
on each field, in 0..=255. Fields encode, decode and root in position order, not
Rust declaration order. Gaps become zero Merkle leaves; the packed active mask is mixed in.

Enums are CompatibleUnions. Each variant requires one unnamed payload and an explicit,
distinct `#[ssz(selector = N)]` in 1..=127. All options must have compatible declarations;
validation precedes wire/value processing. The encoded selector is followed by the selected
payload, and its root is mixed with the selector word. Empty/unknown wire selectors
return `NoSelector`/`UnknownSelector`.

Fields default to their Rust name (raw-identifier prefixes are removed).
`#[ssz(rename = "任意の名前")]` supplies an exact UTF-8 name; duplicate final names are rejected.
Renaming affects schema/path lookup, not bytes or roots.

## Direct encoding

`Ssz::encoded_len() -> Result<usize, CodecError>` performs complete value validation
and returns the exact encoded length without building encoded bytes.
`Ssz::encode_into(&mut [u8]) -> Result<(), CodecError>` writes into an exactly sized slice.
The library implementations need no encoded-child scratch buffers.
The existing `encode()` is now a default method: calculate length, allocate one
zero-initialized output Vec, then write directly into it.

```rust
use nyx_ssz::codec::Ssz;
let value = 0x1234_u16;
let mut output = vec![0; value.encoded_len().unwrap()];
value.encode_into(&mut output).unwrap();
assert_eq!(output, [0x34, 0x12]);
```

For valid values, an incorrectly sized destination returns `InvalidLength` before
any write. Other failures may leave partial output; discard it on error. Only the
supplied slice is writable, so a subslice's surrounding bytes remain untouched.
No rollback copy is made. Reusing a buffer is a separate operation from allocating
`encode()` and must be benchmarked separately.

Manual implementors must replace their old allocating `encode` implementation with
the two required methods. They must agree on validation, length and bytes, preserve
protocol-fault precedence, and never implement either by calling the default `encode()`.
For a simple wrapper, delegate both methods to the wrapped codec:

```rust
use nyx_ssz::{codec::Ssz, error::CodecError, type_info::TypeDescriptor};
struct Counter(u64);
impl Ssz for Counter {
    const TYPE: &'static TypeDescriptor = &TypeDescriptor::Uint(8);
    fn encoded_len(&self) -> Result<usize, CodecError> { self.0.encoded_len() }
    fn encode_into(&self, out: &mut [u8]) -> Result<(), CodecError> { self.0.encode_into(out) }
    fn decode(input: &[u8]) -> Result<Self, CodecError> { u64::decode(input).map(Self) }
    fn hash_tree_root(&self) -> Result<[u8; 32], CodecError> { self.0.hash_tree_root() }
}
assert_eq!(Counter(42).encode().unwrap(), 42_u64.to_le_bytes());
```

Length calculation must still inspect later children after an earlier
`NativeOutputOverflow`, so their protocol errors win over deferred parent
assembly refusal. Sequence/Container output retains the strict four-byte budget;
do not impose that budget on top-level opaque bytes or union prefixes.
`NativeOutputOverflow` remains distinct from protocol `SizeOverflow`.

The new member laws and default-method bridge are **not formally proved**.
The historical `CodecLaw` does not cover these methods; the full proof gate rejects
that incomplete contract until [#29](https://github.com/NyxFoundation/nyx-ssz/issues/29)
is resolved. The 12 hash-independent current-source proofs remain checked.

## Fixed-width decoding

Typed Vector/List and both ProgressiveList variants validate the entire fixed layout
before invoking any child decoder, then read element slices directly without a
temporary range table. Derived variable-width containers use compile-time field positions
and stack-allocated ranges instead of `Vec` offset/range/span metadata.

Decoded byte/bit storage uses standard bulk copying into its owned allocation.
Generic fixed sequence decoding reserves its output only after the first child succeeds.
The initial capacity hint is bounded by the encoded input size, not the potentially
much larger full Rust representation. It never restricts the decoded element count;
the vector grows as needed. This preserves later child errors even when eagerly
reserving the potential full result would overflow native allocation limits.

See [measurements and regression evidence](fixed-decode.md).
These changed codec laws remain unproved under the existing equivalence backlog.

## Small fixed decoding

All-fixed, non-progressive containers validate the declaration and exact body length, then
split field slices directly. They do not build a range table. A width sum that overflows
`usize` is `InvalidLength`, and inputs of at least four gigabytes remain `SizeOverflow`,
both before any field decoder. Completed fields are dropped normally if a later field fails.

`ByteVector<N>` for `N <= 32` stores those bytes inline. `from_bytes` therefore does not
preserve the input allocation at those sizes. `N > 32` and every `ByteList` still move the
accepted `Vec`. Decode of an inline vector performs no allocation. These layout changes are
native-tested and remain under the unproved codec backlog in
[#25](https://github.com/NyxFoundation/nyx-ssz/issues/25).

## Fixed-sequence byte hook

`Ssz::fixed_sequence_bytes(values)` is a provided optimization hook. The default returns
`None`, so existing manual implementations keep element-wise validation and encoding.
On little-endian targets, u8 through u128 return a borrowed canonical byte view through
safe `bytemuck::cast_slice`; boolean, U256 and big-endian targets keep the scalar path.

Parents verify that a returned view matches the declared fixed width and element count.
Encoding copies the complete view once, while basic collection roots split it directly into
32-byte Merkle chunks. No borrowed bytes escape the source slice and no crate-local unsafe
code is used. The hook contract and optimized parent equivalence are unproved under
[#25](https://github.com/NyxFoundation/nyx-ssz/issues/25). See
[native integer sequence output](native-sequence-output.md).

## Fixed-sequence decode hook

`Ssz::decode_fixed_sequence(input)` decodes a sequence of a fixed-width type
under the standard four-byte envelope, without an extra element-count bound, and
returns `Result<Vec<Self>, CodecError>`. This is a provided method: existing manual
implementations and derive users need no override. Its generic default keeps individual
decode callbacks. Variable-width declarations return `BadDeclaration`.

Unsigned integer implementations override it with exact-size collection. On little-endian
targets, aligned u8 through u128 input uses `bytemuck::try_cast_slice` followed by a safe
slice clone; unaligned input, big-endian targets and U256 retain fixed-byte conversion.
There is no crate-local unsafe code, and alignment is checked rather than assumed. Boolean
values keep per-element validity checking.

Typed parents validate declaration, exact count/alignment, capacity and their
four-byte envelope before dispatch. Returned success counts are checked; a custom
override returning the wrong count produces `InvalidLength` and drops the returned
values. Overrides must obey the same fixed-sequence bytes and error contract.

The member and integer-override laws are **unproved** under
[#32](https://github.com/NyxFoundation/nyx-ssz/issues/32).
See [bulk codec results](bulk-codecs.md) for all three optimization stages.

## Decode failures

`Ssz::decode` returns `Result<Self, CodecError>`. There is no associated decode-state
type and no generated failure-state structure. Derived containers and unions stop at the
first child error; Rust drops already completed values before the error is returned.
This keeps the success path smaller and makes manual implementations use one error model.

## Paths, nodes and proofs

`path::resolve::<T>(&steps)` resolves `Position`, `Field`, `Length`,
`ActiveFields` and `Selector` to a checked `u128` generalized index.
Positions are schema ordinals for Container fields, logical element/bit indices for
collections, and explicit selectors for unions. Progressive bounds do not restrict
type-level positions, matching the pinned specification.
`Location` includes packed byte ranges or a bit offset. Reserved words terminate a path.

`node::get(&value, index)` reads the root (one) or an existing descendant.
`node::at_path` resolves then reads. Packed elements share their enclosing node.
Bounded padding is addressable without allocating it. A path is type-level: it does
not establish that a list currently contains that element or a union selects that option.
Absent progressive-spine descendants can be unaddressable; requesting one returns an error.

`proof::create`/`at_path` build bottom-up single branches.
`proof::create_multi` preserves claim order and emits canonical descending helpers.
Zero/root, duplicate and nested claims retain the existing verifier's refusal rules.
Use `merkle::verify_merkle_proof` or `multiproof::verify_merkle_proof` to verify.

Tree access currently materializes the occupied tree from canonical encoded bytes.
It is not a persistent/lazy tree, and proof construction is not sublinear.
All padding is sparse. Index overflow is an error, never truncation.

## Scope

All 4,601 immutable official corpus cases are checked, including every Container and
ProgressiveContainer fixture. The pinned corpus has no CompatibleUnion family; independent
tests cover selectors, compatibility, roots, malformed bytes, generics and fail-fast drops.
Allocator exhaustion, cyclic external static declarations, arbitrary external trait
implementations and a production security audit are outside the correctness claim.
