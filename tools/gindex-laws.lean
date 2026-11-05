import NyxSszVerification.CurrentAudit

open Aeneas Aeneas.Std Aeneas.Std.WP NyxSszVerification

-- Check complete property signatures over actual production calls, including compositions.
-- Unlike API occurrence checks, these reject weakened conclusions and additional premises.
example (index : U128) :
    (NyxSszRust.gindex.sibling index >>= NyxSszRust.gindex.sibling) = RustM.ok index :=
  gindex_sibling_sibling index

example (index : U128) :
    (NyxSszRust.gindex.sibling index >>= NyxSszRust.gindex.parent)
      ⦃ parent => parent.val = index.val / 2 ⦄ :=
  gindex_sibling_half index

example (index : U128) :
    NyxSszRust.gindex.sibling index ⦃ sibling => sibling.val % 2 = 1 - index.val % 2 ⦄ :=
  gindex_sibling_parity index

example (index : U128) (even : index.val % 2 = 0) :
    NyxSszRust.gindex.sibling index ⦃ sibling =>
      index.val = 2 * (index.val / 2) ∧ sibling.val = 2 * (index.val / 2) + 1 ⦄ :=
  gindex_sibling_even index even

example (index : U128) (odd : index.val % 2 = 1) :
    NyxSszRust.gindex.sibling index ⦃ sibling =>
      index.val = 2 * (index.val / 2) + 1 ∧ sibling.val = 2 * (index.val / 2) ⦄ :=
  gindex_sibling_odd index odd

example : NyxSszRust.gindex.depth 0#u128 = RustM.ok (.Err .InvalidIndex) :=
  gindex_depth_zero

example : NyxSszRust.gindex.length 1#u128 = RustM.ok (.Err .RootHasNoBranch) :=
  gindex_length_root

example (index : U128) (level : U32) :
    NyxSszRust.gindex.bit index level =
      RustM.ok (decide ((index.val >>> level.val) % 2 = 1)) :=
  gindex_bit_parity index level

example (outer : U128) (named : 1 ≤ outer.val) :
    NyxSszRust.gindex.concat outer 1#u128 = RustM.ok (.Ok outer) :=
  gindex_concat_root_right outer named

example (inner : U128) (named : 1 ≤ inner.val) :
    NyxSszRust.gindex.concat 1#u128 inner = RustM.ok (.Ok inner) :=
  gindex_concat_root_left inner named

example (outer inner value : U128)
    (joined : NyxSszRust.gindex.concat outer inner = RustM.ok (.Ok value)) :
    1 ≤ value.val :=
  gindex_concat_positive outer inner value joined

example (index : U128) (depth : U32)
    (measured : NyxSszRust.gindex.length index = RustM.ok (.Ok depth)) :
    depth.val = Nat.log2 index.val ∧ 1 ≤ index.val ∧ depth.val ≠ 0 :=
  gindex_length_ok index depth measured

-- Boundary instantiations and kernel reduction cover zero, maximum and native overflow.
example : NyxSszRust.gindex.sibling 0#u128 ⦃ sibling =>
    (0#u128).val = 2 * ((0#u128).val / 2) ∧
      sibling.val = 2 * ((0#u128).val / 2) + 1 ⦄ :=
  gindex_sibling_even 0#u128 (by decide)

example : NyxSszRust.gindex.sibling core.num.U128.MAX ⦃ sibling =>
    core.num.U128.MAX.val = 2 * (core.num.U128.MAX.val / 2) + 1 ∧
      sibling.val = 2 * (core.num.U128.MAX.val / 2) ⦄ :=
  gindex_sibling_odd core.num.U128.MAX (by decide)

example : NyxSszRust.gindex.bit core.num.U128.MAX 127#u32 = RustM.ok true :=
  gindex_bit_parity core.num.U128.MAX 127#u32

example : NyxSszRust.gindex.bit core.num.U128.MAX 128#u32 = RustM.ok false :=
  gindex_bit_parity core.num.U128.MAX 128#u32

example : NyxSszRust.gindex.concat core.num.U128.MAX 1#u128 =
    RustM.ok (.Ok core.num.U128.MAX) :=
  gindex_concat_root_right core.num.U128.MAX (by decide)

example : NyxSszRust.gindex.concat 1#u128 core.num.U128.MAX =
    RustM.ok (.Ok core.num.U128.MAX) :=
  gindex_concat_root_left core.num.U128.MAX (by decide)

example : NyxSszRust.gindex.concat core.num.U128.MAX 2#u128 =
    RustM.ok (.Err .IndexOverflow) := by rfl

example : NyxSszRust.gindex.concat 0#u128 1#u128 =
    RustM.ok (.Err .InvalidIndex) := by rfl

example : 1 ≤ (1#u128).val :=
  gindex_concat_positive 1#u128 1#u128 1#u128
    (gindex_concat_root_right 1#u128 (by decide))

example : (1#u32).val = Nat.log2 (2#u128).val ∧ 1 ≤ (2#u128).val ∧ (1#u32).val ≠ 0 :=
  gindex_length_ok 2#u128 1#u32 (by rfl)

-- Wrong conclusions and opposite-parity premises must fail, regardless of diagnostics.
example : True := by
  fail_if_success
    have : (NyxSszRust.gindex.sibling 0#u128 >>= NyxSszRust.gindex.sibling) =
        RustM.ok 1#u128 := by
      exact gindex_sibling_sibling 0#u128
  trivial

example : True := by
  fail_if_success
    have : (NyxSszRust.gindex.sibling 3#u128 >>= NyxSszRust.gindex.parent)
        ⦃ parent => parent.val = 0 ⦄ := by
      exact gindex_sibling_half 3#u128
  trivial

example : True := by
  fail_if_success
    have : NyxSszRust.gindex.sibling 0#u128 ⦃ sibling => sibling.val % 2 = 0 ⦄ := by
      exact gindex_sibling_parity 0#u128
  trivial

example : True := by
  fail_if_success
    have := gindex_sibling_even 1#u128 (by decide)
  trivial

example : True := by
  fail_if_success
    have := gindex_sibling_odd 0#u128 (by decide)
  trivial

example : True := by
  fail_if_success
    have : NyxSszRust.gindex.depth 0#u128 = RustM.ok (.Ok 0#u32) := by
      exact gindex_depth_zero
  trivial

example : True := by
  fail_if_success
    have : NyxSszRust.gindex.length 1#u128 = RustM.ok (.Err .InvalidIndex) := by
      exact gindex_length_root
  trivial

example : True := by
  fail_if_success
    have : NyxSszRust.gindex.bit 0#u128 0#u32 = RustM.ok true := by
      exact gindex_bit_parity 0#u128 0#u32
  trivial

example : True := by
  fail_if_success
    have := gindex_concat_root_right 0#u128 (by decide)
  trivial

example : True := by
  fail_if_success
    have := gindex_concat_root_left 0#u128 (by decide)
  trivial

example : True := by
  fail_if_success
    have : 2 ≤ (1#u128).val := by
      exact gindex_concat_positive 1#u128 1#u128 1#u128
        (gindex_concat_root_right 1#u128 (by decide))
  trivial

example : True := by
  fail_if_success
    have : (1#u32).val = Nat.log2 (2#u128).val ∧ 1 ≤ (2#u128).val ∧ (1#u32).val = 0 := by
      exact gindex_length_ok 2#u128 1#u32 (by rfl)
  trivial
