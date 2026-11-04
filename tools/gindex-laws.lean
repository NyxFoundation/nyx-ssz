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

-- Boundary instantiations use the same total laws, not native evaluation.
example : NyxSszRust.gindex.sibling 0#u128 ⦃ sibling =>
    (0#u128).val = 2 * ((0#u128).val / 2) ∧
      sibling.val = 2 * ((0#u128).val / 2) + 1 ⦄ :=
  gindex_sibling_even 0#u128 (by decide)

example : NyxSszRust.gindex.sibling core.num.U128.MAX ⦃ sibling =>
    core.num.U128.MAX.val = 2 * (core.num.U128.MAX.val / 2) + 1 ∧
      sibling.val = 2 * (core.num.U128.MAX.val / 2) ⦄ :=
  gindex_sibling_odd core.num.U128.MAX (by decide)

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
