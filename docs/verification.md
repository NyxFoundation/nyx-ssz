---
title: Verification scope and trust boundary
last_updated: 2026-10-24
verification_status: current-12-operation-slice-explicit-issue-linked-skips
tags:
  - ssz
  - aeneas
  - formal-verification
  - bounded-progressive
---
# Verification scope and trust boundary

**Native implementation complete; current-source formal integration pending.**
All 13 shapes have fail-fast codecs returning `CodecError`; generated partial ownership
state has been removed. Typed paths, sparse node access and proof construction are native-tested.
All 4,601 pinned official corpus cases pass with preserved source/version-alignment checks.

The current gate freshly extracts and kernel-checks 12 hash-independent gindex/helper-index operations.
It explicitly reports the remaining 105 documented APIs, 17 codec dictionaries and derive
transformations as unproved with linked Issues. Actual metadata extraction is rejected for
nested-loop returns. The six RustCrypto-dependent laws are unproved under
[#27](https://github.com/NyxFoundation/nyx-ssz/issues/27). See [exact current scope and reproduction](current-verification.md).
Direct encoding length/destination laws and the default allocating bridge are separately
unproved under [#29](https://github.com/NyxFoundation/nyx-ssz/issues/29). Historical `CodecLaw`
proofs do not cover the extended trait, and the full gate explicitly rejects that gap.
Aeneas itself is unchanged; no axiom or input-domain assumption is added.
The broader proof descriptions below are historical registrations, not certification of the
enlarged current program. In particular, historical codec laws over `DecodeFailure<DecodeState>`
do not certify the current fail-fast result type or stack-range container decoder. These obligations
remain under [#25](https://github.com/NyxFoundation/nyx-ssz/issues/25). See [native API](native-api.md)
and [descriptor evidence](full-descriptors.md).

**Historical proof suite, not current-source certification:** Boolean, uint8/16/32/64/128/256, Bitvector/Bitlist/ProgressiveBitlist and borrowed
ByteVector/ByteList/ProgressiveByteList codecs, SHA-256 node combination and
single-/multi-claim Merkle verification, generic node Merkleization and all basic-type
roots, plus borrowed packed byte/bit and Progressive/view roots, are implemented.
`verification/targets.json` links 109 registered operations to proofs over actual
Aeneas output: the thirty-nine existing codec/hash/view operations, eight owned collection
storage operations, three generic codec dispatchers, numeric offset-span validation,
fixed/variable-vector/list and mixed Container layout validation, three immutable layout-view projections and typed
body-range decoding, four declaration-derived metadata APIs, eleven generalized-index operations and
seven packed byte/bit root entrypoints, nineteen owned-bitfield constructors/projections, four owned
byte-array methods, three multiproof operations and shared mixed-slot Container wire assembly.
`verification/traits.json` separately registers every public `Ssz` declaration member and its seventeen
library dictionaries: seven basic instances, four generic sequence constructors, four owned bitfields
and two dedicated owned byte arrays. Each proves the full `CodecLaw`;
methods are not silently excluded. The four generic laws compose with the law of their actual
child dictionary, at every machine-size bound and arbitrary nesting depth.
Every slice/bound yields `RustM.ok`. Encoding has exactly three semantic outcomes:
representable reference success returns exact bytes/order/length; reference protocol error
retains the existing exact fault relation; successful reference bytes longer than
`usize::MAX` permit only the distinct `CodecError::NativeOutputOverflow` refusal.
No representable success or reference error may use that marker. This is the sole
approved extension to `codecEncodingAgrees`, not a resource-failure policy, input-domain
premise, offset-envelope relaxation or additional `CodecLaw` field. Decoder, root,
metadata and fault relations remain unchanged and exclude the new variant.
Conversion preserves bits, order and length. Byte codecs validate `Desc.wellFormed`.
Their relation permits only EmptyVector/vectorEmpty, InvalidLength/scope and
OverLimit/overLimit; other outcomes are false. Boolean errors classify scope/non-bit
faults. Bounds cover `usize`, not all naturals. Zero-copy is source/test evidence,
not a cost theorem. Derived error traits are not protocol proof targets.

## Exact native-output boundary

Pinned `Ssz.serialize` for CompatibleUnion returns the selector prefix followed by the
child bytes without a union-level offset envelope. A lawful single-option union with
a `usize::MAX`-byte opaque ByteVector body therefore reference-succeeds with
`usize::MAX + 1` bytes. `compatible_union_prefix_size`,
`compatible_union_native_boundary` and `compatible_union_native_exists` prove this
symbolically with natural array-size lemmas, not native evaluation or a giant allocation.
`codec_encoding_representable`, `codec_encoding_oversized` and
`codec_encoding_reference_error` prove exact success/refusal/error correspondence.
Only `propext`, `Classical.choice` and `Quot.sound` remain permitted axioms.

The internal collector interface is
`collection_wire::encode_parts<T: Ssz>(&[T]) -> Result<Option<Vec<Vec<u8>>>, CodecError>`.
`Ok(Some(parts))` contains every exact stored part. `Ok(None)` means every child
reference-succeeded but at least one body exceeds native capacity; it is not a fake
empty byte buffer. `Err(error)` is the first genuine child protocol error.
The loop keeps checking subsequent child encoders after a native marker, so a later
protocol refusal wins. An earlier genuine `SizeOverflow` stops immediately. Only after
all children succeed and fixed/variable metadata is checked does `encode_sequence`
turn `None` into the reference's existing `SizeOverflow`/`offsetOverflow`: every
sequence's total must be strictly less than 2^32, even for fixed children. Overflowing
fixed metadata still means fixed, not variable. No combined working-budget or `n*width`
refusal is introduced. Basic root paths prove `None` impossible (each child ≤32 bytes);
they do not gain native-error permission. Encoded temporaries may be discarded, never
owned user input or decoder prefixes/states. The precise-drop gate now also checks
closed standard `Option` temporaries recursively, retaining rejection of user drops.

All fifteen current dictionaries are reproved; generic Vector/List/ProgressiveList
compose solely with the true child's `CodecLaw`, over the full existing domain.
Scripted Rust dictionaries test both fixed and variable layouts and all three parents,
including deferred-marker/later-error precedence; they are not claimed certified.
The kernel fixture gate rejects an actual dictionary claiming native overflow for
one-byte Boolean success, protocol faults hidden by the marker, and native decoder/root
faults. Positive fixtures exercise the symbolic union boundary and representable
exclusion. The registry stays at 103 operations / 15 dictionaries.
Future typed unions and Container/derive codecs will consume this policy; none is
implemented or declared complete by this boundary change.

Boundary validation (2026-10-22): `python3 scripts/verify.py check` passes 103/15,
including fresh extraction/API/trait/import/axiom and positive/negative kernel gates,
all 3,412 unchanged source cases, fmt/clippy/workspace tests and doctests. The complete
`cargo test --locked --workspace --all-features --target i686-unknown-linux-musl`
suite passes, including all four new scripted runtime regressions. All 104 Python
regressions pass. `cargo llvm-cov --workspace --fail-under-lines 80` reports 95.44%
library line coverage.

`container_view_refines` connects the actual extracted `container::container_view` to
pinned `Ssz.structSlices` (the reference's Container layout executable). Its pointwise
`containerMetadata` relation uses each supplied `Desc.fixedSize` exactly: `Ok(None)`
for variable fields, `Ok(Some(width))` for representable fixed widths, and
`Err(SizeOverflow)` for fixed natural widths above `usize::MAX`. It assumes no parent
validity, desired output, representable child-width domain or reduced field/input bound.
The size envelope precedes the all-fixed exact budget or mixed header minimum; budget
faults precede first-offset, offset-order and final-scope faults. Natural sums exceeding
the machine word are handled without arithmetic overflow. Zero fields/widths, repeated
offsets, empty bodies and interspersed fixed fields retain exact field-order bytes.
Success includes the existing `sequenceViewValid` range invariant, permitting immutable
`SequenceView` projections without rechecking each range. The ≥2^32 input branch is
proved symbolically, not exercised by allocating multi-gigabyte test inputs.
This primitive validates layout only. No Container trait dictionary, derive macro,
field-name/type validation, generated conversion/ownership proof or Container root is
claimed here; all remain required for the future ordinary-named-struct codec.

ProgressiveByteList is checked against `.progressiveList (.uint 1) limit`, not
an opaque-byte descriptor. Its injective value view maps every byte to a uint8,
preserving order and length. The sequence layout and singleton-byte slicing are
proved to encode/decode exactly the borrowed input. `None` imposes no declared
capacity; optional capacities cover every `usize`. Both forms enforce the pinned
reference's sequence size of less than 2^32 bytes. Above that size, serialization
checks capacity first and deserialization checks offset width first; both error
precedences are proved, including inputs too large to allocate in unit tests.

Unsigned native codecs reuse Rust's standard little-endian and checked slice/array
conversions. Their actual models are proved against `Ssz.serialize`/`Ssz.deserialize`
for every value and input slice. Generic bitvector/byte lemmas establish both
encoding order and decoding value; only incorrect lengths are refused.
`U256` is `ruint::Uint<256, 4>` pinned to 1.20.1. Its value view concatenates all
four least-significant-first limbs, preserving all 256 bits. The reached library
getters, constructor, mask/count checks and Rust's `usize::div_ceil` are extracted
as real definitions and their required calls are proved. Ruint's unsafe byte casts
and arithmetic APIs are neither used nor covered. No library-correctness axiom is added.

Bitvector codecs cover every `usize` declaration and typed input, including
zero-length declarations, wrong lengths and noncanonical high padding. Their
actual Vec loops are proved terminating with decreasing natural measures; byte
counts, indices, shifts and increments are proved safe. The value view preserves
Boolean order exactly. Bitlist proofs cover delimiter insertion, empty values,
byte-boundary crossings, every final-byte pattern, capacity violations and the
separate empty-encoding/no-delimiter/trailing-zero error classes. Decoder byte-to-bit
arithmetic is guarded before multiplication/addition; both guards are proved equivalent
to the reference's natural-number count check. The all-zero scan has a decreasing
measure and prefix invariant. Vec allocation uses Aeneas's model; OS/resource failures
remain outside the guarantee.

ProgressiveBitlist accepts optional capacities, including no capacity, without
imposing a new format limit. The borrowed `BitListView` stores only validated,
immutable encoded bytes. Its value view is the reference's packed-bit decoding,
not a projection discarding bit contents. Returned views are proved nonempty and
canonical; their length getter's nonempty precondition is discharged by the decoder
and preserved by private fields and borrowing. The exact storage getter is also
audited. Bit counts use `u128` as an implementation choice to represent up to eight
times a `usize` byte count, not as an SSZ requirement. Widening, arithmetic safety,
all error classes and both bounded/unbounded codecs are proved.

Basic `hash_tree_root` operations cover Boolean and all six unsigned widths, for
every typed value. They refine the unchanged `Ssz.hashTreeRoot`, through its actual
layout, packing, bounded merkleization and depth-zero tree rules. The extracted
array-prefix mutation and standard `copy_from_slice` call are proved safe and
preserve every encoded byte, with zero padding to 32 bytes. A uint256 fills the
node exactly. Basic roots do not SHA-256 hash their single leaf. Generic Vector/List roots
are separately proved below. Packed byte/bit and Progressive roots are also proved; other family
root APIs remain unfinished.

Packed roots cover every native byte/Boolean slice, declaration and machine-size/optional capacity.
The real loop preserves all bytes and masks only a borrowed bitlist's delimiter position, then produces
exact zero-padded nodes. Bounded capacities use ceil(bytes/32) or ceil(bits/256). Progressive recursion
uses capacities 1,4,16,... on the right-hand spine, terminating as the remaining chunk count decreases;
all shifts, depth increments, width multiplication, slices and pushes are proved safe. List mixins retain
the actual count, not capacity. The canonical borrowed-view invariant established by its decoder is the
only view precondition, and its full u128 bit count is not narrowed to usize. Input encoding is immutable;
no intermediate encoding copy or Boolean expansion is required, though a node buffer is allocated.

The pinned upstream root deliberately ignores optional Progressive bounds. The three raw Progressive
APIs therefore refine explicit local reference wrappers composing unchanged `Ssz.boundCheck` with
unchanged `checkedHashTreeRoot`. Counterexample lemmas record why direct equality would be false.
This preserves every input/bound, exact fault category and complete root; it does not alter `CodecLaw`,
`codecFaultAgrees` or the pinned executable. Progressive byte roots do not inherit the unrelated <2^32
serialization-envelope restriction.

`hash_pair_refines` proves termination and exact equality with upstream `Ssz.combine`
for every 512-bit input. This covers SSZ's 32-byte-child/32-byte-child hashing path,
not a general-purpose arbitrary-message SHA-256 API. Schedule, all 64 rounds,
both compression blocks, padding and output encoding are connected to the unchanged
upstream SHA-256 implementation. No external hash-function correctness is assumed.
This paragraph describes the historical scalar implementation only. The current runtime
uses RustCrypto `sha2` 0.10.9; neither it nor its dependent root laws are covered by this theorem.
`HistoricalMerkle.lean` and `HistoricalMultiproof.lean` retain omitted snapshot definitions,
not current-source hash certification or a runtime fallback.

`merkleize_refines` covers all complete-node slices and every optional `usize`
capacity against `Ssz.merkleizeBounded`. Empty trees, zero capacity, undersized
capacity errors and rounding up `usize::MAX` are covered. Depth uses overflow-free
halving rather than materializing the next power of two. Recursive calls decrease
the count or depth; every shift and slice boundary is proved safe. Zero subtrees
are computed without allocating padding and proved equal to the reference's cache.
These are correctness/termination guarantees, not allocation or runtime cost theorems.

`calculate_merkle_root_refines` and `verify_merkle_proof_refines` connect the actual
Rust APIs to `Ssz.calculateMerkleRoot` and `Ssz.verifyMerkleProof`. All `u128`
indices and representable branch slices are covered. Node widths are enforced by
`[u8; 32]`; their reference width checks are proved successful. Zero, the root
index and incorrect branch lengths return the corresponding error category;
a well-formed nonmatching proof returns false. Proofs cover index guards, sibling
order, recursive termination, SHA-256 composition and bytewise root comparison.
Error relations classify reasons, not the reference error's numeric diagnostics.
Typed paths, proof generation and full-library conformance are not yet proved.

`gindex` implements raw numeric tree operations, not typed path resolution. Depth and
branch length refine `Ssz.gindexDepth`/`gindexLength`, distinguishing zero, root and
ordinary indices. Bit queries, truncation, sibling and parent preserve the reference's
raw zero/root semantics and zero extension beyond the word width. Child construction,
concatenation and rebasing return the exact reference natural number when representable;
`IndexOverflow` is permitted only when that number exceeds `u128::MAX`, never on a
representable result. Concatenation splices the inner path rather than multiplying indices.
Bottom-up path and sibling lists refine `getPathIndices` and `getBranchIndices`, preserving
all nodes and their order. Their real Vec loops terminate with bounded shifts, increments
and pushes. Positions/depths use `u32`; indices use every `u128` value. The leading-zero
primitive is the pinned Aeneas standard model, with its exact log2 arithmetic proved here.
Native overflow checks and error precedence are part of these proofs. Typed field/element
paths and progressive chunk positions remain separate.

`multiproof::{helper_indices, calculate_merkle_root, verify_merkle_proof}` refine
`Ssz.getHelperIndices`, `Ssz.calculateMultiMerkleRoot` and `Ssz.verifyMerkleMultiproof`.
Their unconditional kernel theorems cover all native slices and every `u128` index,
including depth 127. Leaf-count refusal precedes helper validation. Empty/repeated
requests, per-claim zero/root/path validation, related claims and proof-length errors
preserve the reference order and error categories. Descending helper order is exact;
complete frontiers reconstruct a root and well-formed mismatches return false.

Helper selection scans sibling candidates, excludes all claim paths, and selects the
next distinct greatest helper before pushing. It never accumulates flattened paths.
`SizeOverflow` is allowed exactly when the mathematical helper list exceeds `usize::MAX`;
there is no input, path-count or working-set restriction. The verifier validates first
and stops helper collection once another helper exceeds the supplied proof length.
Its refusal is therefore exact `proofLength`, even for an unrepresentable helper list,
not `SizeOverflow`. The checked pushes depend only on actual output length.

Root reconstruction borrows leaves and helpers separately and recursively hashes the
finite frontier tree, using at most 128 frames. No combined `leaves + helpers` vector,
count addition or machine-size budget is needed. Checked child arithmetic and decreasing
depth cover top-bit indices. Antichain, uniqueness and complete-frontier lemmas connect
this traversal to the unchanged reference level fold. Existing `hash_pair` proofs
supply every parent hash. Allocation failure and finite stack/runtime resources remain
trust boundaries, not excuses for machine-index overflow or weakened contracts.

`Multiproof/{Frontier,Antichain,Fold,Cut}.lean` adapt relevant pinned SSZ proof lemmas
under the project's Lean version; they prove statements about the unchanged executable
specification. The upstream proof-module dependency tree requires its newer toolchain,
so these kernel-checked compatibility proofs avoid importing it. No pinned source or
old fault cases are changed. The full gate also runs `tools/multiproof-oracle.lean`
against the actual pinned executable definitions, checking precedence, explicit trees,
wrong roots/order and every supported depth. This baseline makes no performance claim.

`bytecollections::ByteVector<N>` and `bytecollections::ByteList<MAX>` are dedicated owned
opaque-byte types, not aliases for generic uint8 sequences. Current source copies `ByteVector`
values of at most 32 bytes into the value; larger vectors and byte lists still move `Vec<u8>`.
Historical proofs assumed universal vector retention and are not current-source certification.
Their four registered public methods remain explicitly unproved. The old statements covered
constructor fault classes and immutable byte-for-byte projections. Rejected byte storage
can be dropped without invoking arbitrary user element destructors. Runtime tests check pointer
identity above 32 bytes, zero/maximum bounds and bounded Debug output (unit-array markers are not formatted).
Historical extracted dictionaries are not current-source certification of the changed storage.
Dedicated descriptors
preserve pinned `.byteVector`/`.byteList` metadata: fixed/variable width, vector nonzero validity,
composite packing classification, and compatibility with equal-bound vectors/lists of uint8.
Compatibility does not identify wire specifications: opaque-byte serialization/deserialization has
no generic sequence <2^32 size envelope. Containing generic sequences retain their own envelopes.
Allocation/resource failures retain the existing trust boundary. This paragraph does not extend the current 12-operation proof.

`collections::Vector<T, N>` and `collections::List<T, MAX>` own a private `Vec<T>`.
Their generic constructors are proved for every element type and machine-size bound:
vector declarations must be nonzero and match exactly; list lengths must not exceed
the capacity, including zero capacity. Every success contains the original vector;
every failure returns every original element with the exact error kind. Immutable
projections preserve all elements and their order. Returning rejected storage avoids
invoking user element destructors inside a constructor. Zero-sized unit-array fields
retain const bounds in generic extraction without storing runtime bytes; Debug omits
those markers so formatting is not proportional to the declared bound. Rust tests
check allocation identity, no in-constructor element drops, nested non-Copy elements,
formatting and maximum machine counts for zero-sized elements. These are shape/storage
contracts, not element validity, serialization, tree-root or allocation-cost theorems.
Separate compositional proofs now establish their generic `Ssz` implementations. Parent
observations preserve the full ordered sequence of child observations. Fixed and variable
serialization, offset arithmetic, exact decoded values and packed/composite roots refine the
unchanged checked reference operations. No extra well-formedness or bound restrictions are
assumed: zero vector counts, invalid child declarations, empty/zero-capacity lists and
unrepresentable fixed widths follow the reference error precedence. List roots mix in the
actual element count, not capacity. Every byte and node survives packing; capacity arithmetic
is widened and bounded before narrowing. Declaration/layout failures carry empty owned state;
callback failures forward the exact typed completed prefix and exact nested child state.
These ownership equalities are proved separately from the protocol error observation.

`collections::ProgressiveList<T>` has the exact `.progressiveList child none` declaration.
Its constructor and immutable projection preserve every owner, including machine-maximum zero-sized
storage. Its complete dictionary proves the unchanged `CodecLaw` using only its actual child's law:
canonical fixed/variable serialization, exact decode observations and owned partial state, and the
Progressive root over packed basic elements or complete child roots. Roots mix the element count,
not packed-byte length. Declaration failures and overflowing fixed child widths retain existing error
precedence, including successful empty lists. No context bound is silently encoded in this type;
explicitly bounded Progressive contexts use the separate types below.

`collections::BoundedProgressiveList<T, MAX>` preserves the exact
`.progressiveList child (some MAX)` declaration and private immutable owned storage.
Its constructor returns every input owner on both success and `LengthError` rejection; projections
retain the complete ordered vector. Every native capacity is covered, including zero and machine
maximum; `Some(MAX)` is never a sentinel for `None`. The full dictionary law requires only the actual
child `CodecLaw`, including malformed child declarations and overflowing fixed widths. Existing list
layout proofs establish capacity rejection at the exact reference stage before child callbacks;
declaration/layout errors carry empty state and callback errors retain the exact completed prefix
and failing child's owned state. Serialization retains the unchanged four-byte sequence envelope.
Root proofs reuse the Progressive spine over packed basic values or child roots, not ordinary bounded
List Merkleization. As in the unchanged reference, rooting ignores the context cap even for arbitrary
over-cap private states; encoding still refuses those states. No constructor-validity assumption is added.

`bitcollections::BoundedProgressiveBitList<MAX>` retains the exact
`.progressiveBitList (some MAX)` declaration, canonical accepted byte allocation and unit decode state.
Its four public methods and complete dictionary are proved for every native bound and arbitrary packed
private storage. Logical observations preserve every bit, order and the full widened count; getters
cover every `u128` index without count truncation. Existing normalization proofs handle malformed
private bytes, while wire decoding retains all delimiter fault categories and capacity precedence.
Encoding checks capacity in `u128`; rooting normalizes logical bits without imposing the context cap,
matching the pinned reference's complete Progressive-root behavior. Neither path expands Boolean arrays.
Runtime tests compare both new dictionaries with independent accepted unbounded values and exercise
nested fixed/variable children, zero/one/maximum bounds, allocation retention and exact failure states.
There is no distinct bounded family in the pinned source corpus: all unchanged 3,412 source cases
continue exercising their existing real dictionaries. This completion is not full SSZ release completion.

Owned `BitVector<N>`, `BitList<MAX>`, `BoundedProgressiveBitList<MAX>` and unbounded `ProgressiveBitList` retain packed bytes.
Byte constructors establish exact canonical decoder invariants and retain the entire accepted Vec;
Boolean constructors prove the corresponding serialization and decoded logical bits. Immutable byte
projections preserve storage exactly. List counts/getters cover every u128 index and their full widened
logical count; vector logical getters use the canonical invariant established by construction/decoding.
No progressive bit-count-to-usize assumption is added. Type declarations and all four distinct
padding/delimiter fault categories are preserved. Debug omits zero-sized const markers.

All four complete dictionary laws quantify over every extracted struct value, not only canonical
storage. Their observation retains every logical data bit: vector storage uses its final declared bit
width, and delimited storage uses every byte before the highest final bit. Empty private storage
observes empty bits. Canonical re-encoding masks vector padding and supplies a missing logical delimiter
for otherwise inaccessible malformed stored states. This does not relax wire decoding: byte constructors
and decoders still reject noncanonical input. No canonical-storage premise is added to `CodecLaw`.
Roots prove exact byte/vector capacity equivalence, delimiter masking, length mixins and the Progressive
spine. Runtime encode/root paths allocate packed byte/node buffers, never Boolean expansions; pointer
identity and allocation behavior remain source/test evidence rather than cost theorems. Official bit
families now exercise the actual owned dictionaries in addition to the existing borrowed operations.

`container_wire::assemble` is a proved raw internal-layout assembler over paired
`(inline, Vec<u8>)` slots. `container_assemble_refines` relates the actual extracted
implementation to unchanged `Ssz.assemble`, with flags and bytes projected from the
same ordered slots. The size step, size loop and wrapper prove the exact four-byte
budget check: totals at or above 2^32 return `SizeOverflow`, matching the reference
`offsetOverflow` category. Header and body loops use decreasing remaining-index
measures, preserve exact inline bytes, little-endian offsets and variable bodies,
and prove native indexing, increments, casts, additions and Vec capacity safe.
The registered wrapper has no extra count, width, byte-data or validity premises;
empty slots and empty inline/variable parts are included. No Rust or generated
extraction body is replaced by a paper model. This proves wire assembly only:
Container declarations, ordinary-struct derive, generated ownership/conversions,
value encoding/decoding and Container roots are not completed by this theorem.
The fifteen existing `Ssz` dictionaries and their `CodecLaw` obligations are unchanged.

`codec::Ssz` has a static immutable `TYPE` declaration and encode/decode/tree-root
operations. Boolean and uint8/16/32/64/128/256 dictionaries prove that the complete
declaration maps to their exact reference descriptor, in addition to their value-codec
laws. Size and classification are no longer independently overridable constants. The array-to-Vec copy loop
preserves every byte; a decreasing measure proves termination and all indices/increments
are checked safe. Scalar error conversion preserves exact error classes. Dispatcher proofs
are conditional on `CodecLaw`: an external handwritten dictionary is callable but is not
presumed lawful. Only supported, explicitly registered library instances are certified.
The trait's `DecodeState` associated type is inventoried in declarations and every
implementation. Decoding returns `DecodeFailure<DecodeState>`, whose kind is related
to the reference fault. All seven basic dictionaries have unit state; the unit-state
lemma proves error state is empty. A separate function-equality theorem proves that
dispatch forwards the exact result, including arbitrary owned state, not only its
protocol observation. Element-loop ownership is proved separately below; the wire law
alone would not establish it. Error formatting and external implementations are Rust tests,
not protocol proof targets; a non-Debug state/drop-counter test checks that forwarding
and formatting do not invoke its destructor or expose it.

`codec::fixed_size`, `is_basic`, `validate_type` and `is_compatible` derive metadata
from `Ssz::TYPE` without invoking value codecs. At baseline `155619d`, the shared
implementations refined `Desc.fixedSize`, `Desc.isBasic`, `Desc.wellFormed` and
`Ssz.isCompatible` for Boolean, machine-size integer widths and recursively nested
Vector/List/ProgressiveList declarations. Those artifacts are now stale relative to
the native full-descriptor extension; extending the descriptor view and proofs over
every finite declaration, including malformed declarations and all native bounds,
is still required. The negative fixture rejecting a bool dictionary mislabeled
uint8 and all existing `CodecLaw` contracts are preserved unchanged.

Native metadata now additionally covers byte/bit vectors and lists,
ProgressiveBitList, Container, ProgressiveContainer and CompatibleUnion. Separate
static arrays preserve every name, mask, selector, child and mismatched count.
Safe const byte comparison preserves complete UTF-8 names, including NUL and
canonically distinct spellings, without hashing or normalization. Progressive
bounds remain in declarations and equality; only compatibility ignores them.
Compatibility follows structural equality's shortcut even for invalid declarations,
checks progressive shared positions and distinct-name placement, and compares
every union cross-pair without assuming transitivity or declaration validity.
Validation follows the pinned executable's exact error categories and order.
Vector width arithmetic reports unrepresentable products. Zero counts cancel
fixed overflow, not variable width. Container gaps add no bytes, empty internal
field lists have width zero, and a variable field before or after fixed overflow
makes the entire container variable. These are descriptor behaviors, not Container,
ProgressiveContainer, Union, derive or other missing value-codec completion.

External declarations must still satisfy the declaration law; cyclic static graphs
have no finite descriptor view and are not certified. Static storage and no
metadata-construction allocation are source/compiler properties, not runtime-cost
theorems. No new trust-boundary exception or well-formedness assumption is introduced.

`codec::decode_elements` traverses validated body ranges with any explicitly lawful
child dictionary. The actual Vec loop terminates and preserves element order, with
safe indexing, pushes and increments. Its wire result agrees with per-body checked
deserialization; for a well-formed child declaration this is the unchanged upstream
`Ssz.deserializeEach`. A second, exact callback-trace equality retains all actual typed
values in the successful prefix and the failing child's opaque owned state, not only
their protocol observations. Empty views invoke no callbacks; the first child failure
stops traversal. The parent type/layout declaration remains the caller's responsibility.
This lower-level helper does not itself select a Vector/List type; the new parent dictionaries
perform those checks. Non-Copy/drop-counter Rust tests exercise both the traversal and typed
nested parent failures, verifying no element/state destructor runs before returning ownership.

`sequence::offset_spans` refines `Ssz.offsetSpans` for every representable offset
slice and scope, preserving every span and both exact ordering/scope error categories.
Repeated offsets yield empty bodies. Unordered pairs are checked before the final
scope error, matching reference recursion. The loop's prefix/suffix invariant and
decreasing measure prove termination; indexing, increments, subtraction and pushes
are safe even at machine maxima. This numeric helper does not enforce a four-byte
wire envelope, validate the first offset/header, or parse input bytes; those are
separate layout checks, not silently presumed complete here.

`sequence::variable_vector_view` refines `Ssz.vectorSlices` for every variable-width
element descriptor, machine-size count and input slice. It checks the less-than-2^32
composite envelope, fixed-count header budget and exact first offset before numeric
span validation. Its offset-read and range-construction loops terminate and have safe
arithmetic, indexing and pushes. Successful `SequenceView` values retain the original
input and validated immutable ranges; `len`, `is_empty` and `get` preserve every body
and order, including empty bodies and out-of-range queries. Getter safety uses the
range invariant established by decoding and preserved by private, immutable fields.
Zero-copy is source/pointer-test evidence, not a cost theorem. The helper validates
layout, not the containing type declaration or any element value. A zero count follows
the reference's internal helper, not a legal zero-length Vector declaration.

`sequence::variable_list_view` refines `Ssz.listSlices` for variable-width elements
and every optional machine-size capacity. Empty input gives an empty view. Nonempty
input must supply a complete first offset, at least four, aligned to the table width
and within the byte scope. It determines the element count before checking capacity;
capacity errors precede later offset-order errors. The inferred count and all error
precedences match the reference. Bounded and unbounded forms reuse the proved
fixed-count layout validator and its view invariant, without adding a format limit.

`sequence::fixed_vector_view` and `fixed_list_view` refine the fixed-width branches
of the same `Ssz.vectorSlices` and `Ssz.listSlices`, for every representable width,
count, input and optional capacity. Vector byte-budget multiplication is checked;
unrepresentable or mismatched budgets return the reference scope error class.
The range loop terminates and proves every multiplication, increment, push and slice
safe. Empty lists return no bodies even at zero width; nonempty lists reject zero
width and incomplete elements before capacity. Envelope checks precede all these
checks. Zero-width/count internal slicing does not certify a legal type declaration.
Both paths establish the existing immutable view invariant, so typed element traversal
and getters need no new assumptions. Element validity and collection codecs remain separate.

Checks cover Rust tests/doctests/lints, extraction/API/import coverage, proofs and
pinned source integrity. For the integrated owned byte/bit, multiproof and raw Container proof slice,
`python3 scripts/verify.py check` passes all 103 operations, 15 dictionaries and 3,412 source
cases, including the actual owned byte and bit dictionaries. All 103 Python regression tests
also pass (`python3 -m unittest discover -s scripts -p 'test_*.py'`). The library-source
coverage gate (`cargo llvm-cov --workspace --ignore-filename-regex '/tests/' --fail-under-lines 80`)
reports 95.51% lines overall, 96.75% for `container.rs` and 100% for `container_wire.rs`.
This excludes integration-test files but includes in-module unit tests; coverage is not a
formal branch or cost guarantee. The x86 CI job also executes the complete workspace tests
on `i686-unknown-linux-musl` to exercise 32-bit native size boundaries.
Generated declarations are partitioned into dependency-ordered
modules with qualified names and declaration bodies preserved. Dependency-aware
canonical ordering and inter-declaration whitespace avoid unrelated extraction
order changes invalidating cached proofs. Registry source-location comments use
portable package-relative paths; executable declaration bodies are unchanged.
Aeneas is source-built through the immutable Nix reference in `sources.lock`, using
NyxFoundation's backport of the nested-static-reference repair to the existing pinned
baseline. Charon, Rust, Lean and the Lean standard-library definitions are unchanged.
The repair retains loans for expanded static shared references, consistently erases
runtime lifetime annotations and preserves the type comparison; invariant checks are
now enabled for every SDK extraction. Rust and Lean regressions cover nested static
declarations. Existing 57-operation extraction was byte-identical with the repair.
The second minimal backport visits nested constant operands during effect analysis, including
aggregates, repeats and call arguments. Trait-associated constant reads must not be classified
as pure while their generated bodies use monadic binds. Native regressions and three regenerated
Lean fixtures pass; all committed extraction artifacts for the existing 70-operation baseline are
byte-identical with this fix. Neither patch changes the standard Lean models or proves the compiler correct.
The standalone Nix target avoids rebuilding Charon's unrelated Rust executable and
keeps a GC-rooted output link. Bootstrap requires Nix; source or build failures stop
verification, with no prebuilt/default-extractor fallback. This changes a trusted
translation component, not a proved compiler-correctness theorem.

Extraction requires the pinned Rust toolchain's Miri and rust-src components. It prepares
the standard-library MIR with `cargo +nightly-2026-08-18 miri setup --print-sysroot`
and passes that sysroot explicitly to Charon. Preparation failures stop extraction;
there is no default-sysroot fallback and no executable-body rewriting. Each extraction
uses a fresh Cargo target directory to avoid reusing artifacts from a different sysroot.
Procedural macros discovered in locked Cargo metadata are first compiled with the normal
host sysroot: they execute during compilation and cannot link against Miri's runtime.
Charon then compiles the library dependencies and implementation against the explicit MIR sysroot.
The former Python `usize::div_ceil` body replacement was not machine-checked and has
been removed; its byte-equality check did not establish equivalence to the original output.
`sources.lock` explicitly names transparent foreign items; opaque foreign axioms
are refused rather than treated as library contracts. The complete module inventory
is checked against fresh extraction. Public inherent methods are discovered from
Rustdoc's implementation/type paths, and unknown or ambiguous paths are refused.
Pinned Charon cannot select inherent methods by explicit paths. They use real call
graphs or explicit `verify::start_from` declaration attributes. The `verification`
Cargo feature enables only tool registration and those attributes; it does not select
alternate executable bodies. Normal consumers do not need the feature. Extraction
selects the annotated methods, while exact Rustdoc registration and concrete-body
checks remain mandatory; opaque declarations cannot satisfy coverage. Unregistered public traits fail. For `Ssz`,
Rustdoc declaration/implementation inventories, including associated types, and
concretely extracted dictionaries must match the registry exactly. Lean checks that each theorem establishes the supported
`CodecLaw` for that precise dictionary: either an unconditional concrete law or a universal
constructor law requiring only laws for its exact child dictionaries. Constructor arguments must
be distinct universal type/dictionary/usize parameters, with independent child shapes and observations.
Specialized bounds, extra data domains (including `Fin 0`), assumed parent laws, unrelated theorems,
impossible premises, reflexive/vacuous predicates and another dictionary's proof are refused. Trait declarations and implementations are
partitioned by their Rust source module while preserving native-type Lean namespaces.
Statement adequacy still requires review. Negative fixtures reject custom/native axioms
and unrelated proofs. Only propext, Classical.choice and Quot.sound are allowed.

Pinned Cryspen Aeneas uses Lean 4.31.0; upstream SSZ pins 4.33.1.
The unchanged executable modules and imported codec lemmas build with 4.31.0 here.
Not all upstream proof modules do (`Codec.Admits` exceeds its recursion limit;
`Proofs.Merkle.Tree` has incompatible monadic simplification). Needed tree cache
and evaluation laws are re-proved locally over the unchanged executable definitions.
Unqualified `lake update` changes the toolchain.
Trust includes Lean's kernel, Charon/Aeneas/models, Rust's compiler/runtime and
downloaded tools/caches. Pins do not prove binary integrity; English/formal-model
alignment remains upstream.
Pinned Aeneas `filter_marker_traits` removes `core::marker::Destruct`, even when
Charon receives `--precise-drops`. A runtime counterexample with a panicking destructor
and the generated model confirmed that this does not verify user destructor execution.
The axiom audit cannot detect this semantic erasure. Generic constructors return rejected
storage, and typed decode errors retain partial ownership rather than silently drop
arbitrary elements inside the operation. The element traversal now proves exact retention
of values and nested state, separately from wire conformance. Before backend translation,
`llbc_drops.py` checks successful precise, undesugared LLBC and rejects local normal-flow
Drop statements except closed standard scalar/reference/array/tuple/Vec/Option types and transparent
local non-generic records with recursively safe fields and no user destructor. Real freshly emitted
LLBC fixtures accept closed bookkeeping and node arrays while rejecting an actual user `Drop`.
It resolves interned types, rejects unknown/generic owned types and
explicit `mem::drop`/`mem::forget` calls, and does not rewrite the artifact. Compiler-generated
destructor bodies and unwind cleanup are excluded: caller cleanup, arbitrary foreign
library models and callback internals are not verified by this syntactic check. This
adds a fail-closed regression guard, not a formal destructor-execution/cost theorem.
The official-vector runner pins `ethereum/consensus-spec-tests` independently of the formal model.
Its external cache is checked against the pinned source, an independently pinned SHA-256 inventory
and actual blob contents. The inventory pin was cross-checked against the authoritative upstream Git tree
API, so locally corrupted nested Git objects cannot redefine the expected fixtures;
local modifications, symlinks and extra fixtures are refused. Git replacement objects are ignored
and cache-local hooks/fsmonitor are disabled. Verification avoids filter-capable worktree commands,
checking files directly instead of consulting index cleanliness. Real repository regressions exercise
replacement refs, fsmonitor and clean-filter execution, corrupt nested objects, and modifications hidden
by index flags. Fresh checkout attributes force raw blobs without filters or text/encoding conversions;
this also avoids host Git LFS settings falsely reporting upstream YAML files as modified.
The current explicit CI selection covers all 3,412 Boolean/unsigned/basic-Vector/Bitvector/Bitlist/
ProgressiveBitlist/basic-ProgressiveList source cases, comparing decoded YAML values, re-encoding,
roots and invalid-input rejection under the explicit current-version profile.
The published 2025 corpus uses the pre-fix left-hand Progressive spine. Its source data and old root
metadata remain immutable; 1,020 current root expectations are regenerated, with 730 differences, by
`ethereum/remerkleable` commit 2f0baeef0082d4278acaef7d822deb7009d7db7e. The official source archive's
SHA-256 is pinned and checked before extraction/execution. Fresh extraction rejects path traversal,
links, duplicate entries and oversized sources; an isolated Python process uses only that authenticated
package. Generated expectations identify both revisions, must match each original metadata root and
must all be consumed exactly once; there is no old-root fallback.

The original basic-ProgressiveList data also has 237 contradictory invalid labels (30 empty, 207
nonempty): every affected compressed blob is exactly equal to a published valid blob of the same
primitive type. The version-2 profile records each concrete valid witness, never edits the corpus and
never skips an invalid case. The runner resolves only published same-type witnesses, rechecks their
original root metadata and decompressed byte equality, then requires the supposedly-invalid input to
produce that valid witness's full value, encoding and current root. Missing, repeated, cross-type,
escaping or unconsumed witnesses fail. All other originally-invalid cases still require rejection.
Two corpus families remain unsupported. This is neither a universal proof of the external oracle nor
full corpus coverage.
OS faults/resource exhaustion and whole-library/cryptographic security are excluded.
x86_64 extraction requires the prepared Miri sysroot. Native aarch64 extraction has not been exercised.
