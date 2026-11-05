import Lean.Elab.Tactic.Omega
import NyxSszVerification.Gindex.Concat
import Ssz.Proofs.Type.PathLaws

open Aeneas Aeneas.Std Aeneas.Std.WP

namespace NyxSszVerification

private theorem gindex_u128_exact
    (actual : core.result.Result U128 NyxSszRust.merkle.ProofError) (expected : U128)
    (agrees : gindexResultAgrees UScalar.val U128.max actual (.ok expected.val)) :
    actual = .Ok expected := by
  cases actual with
  | Ok value =>
    have same : value = expected := UScalar.eq_of_val_eq agrees
    exact congrArg core.result.Result.Ok same
  | Err error =>
    cases error <;> simp_all [gindexResultAgrees]
    scalar_tac

private theorem gindex_success_reference {T : Type} (observe : T → Nat) (limit : Nat)
    (value : T) (expected : Except Ssz.Err Nat)
    (agrees : gindexResultAgrees observe limit (.Ok value) expected) :
    expected = .ok (observe value) := by
  cases expected with
  | ok result => exact congrArg Except.ok agrees.symm
  | error fault => cases agrees

/-- Transfer `Ssz.gindexBit_parity` over every native index and bit position. -/
theorem gindex_bit_parity (index : U128) (level : U32) :
    NyxSszRust.gindex.bit index level =
      RustM.ok (decide ((index.val >>> level.val) % 2 = 1)) := by
  obtain ⟨actual, run, agrees⟩ := spec_imp_exists (gindex_bit_refines index level)
  simpa only [agrees, Ssz.gindexBit_parity] using run

/-- Transfer `Ssz.gindexConcat_root_right` with precisely its named-index premise. -/
theorem gindex_concat_root_right (outer : U128) (named : 1 ≤ outer.val) :
    NyxSszRust.gindex.concat outer 1#u128 = RustM.ok (.Ok outer) := by
  obtain ⟨actual, run, agrees⟩ := spec_imp_exists (gindex_concat_refines outer 1#u128)
  have reference : Ssz.gindexConcat outer.val (1#u128).val = .ok outer.val :=
    Ssz.gindexConcat_root_right named
  rw [reference] at agrees
  simpa only [gindex_u128_exact actual outer agrees] using run

/-- Transfer `Ssz.gindexConcat_root_left` without assuming native success. -/
theorem gindex_concat_root_left (inner : U128) (named : 1 ≤ inner.val) :
    NyxSszRust.gindex.concat 1#u128 inner = RustM.ok (.Ok inner) := by
  obtain ⟨actual, run, agrees⟩ := spec_imp_exists (gindex_concat_refines 1#u128 inner)
  have reference : Ssz.gindexConcat (1#u128).val inner.val = .ok inner.val :=
    Ssz.gindexConcat_root_left named
  rw [reference] at agrees
  simpa only [gindex_u128_exact actual inner agrees] using run

/-- Transfer `Ssz.gindexConcat_positive` with the upstream successful-result premise. -/
theorem gindex_concat_positive (outer inner value : U128)
    (joined : NyxSszRust.gindex.concat outer inner = RustM.ok (.Ok value)) :
    1 ≤ value.val := by
  have agrees : gindexResultAgrees UScalar.val U128.max (.Ok value)
      (Ssz.gindexConcat outer.val inner.val) := by
    simpa only [joined, spec_ok] using gindex_concat_refines outer inner
  have reference : Ssz.gindexConcat outer.val inner.val = .ok value.val :=
    gindex_success_reference UScalar.val U128.max value
      (Ssz.gindexConcat outer.val inner.val) agrees
  exact Ssz.gindexConcat_positive reference

/-- Match `Ssz.gindexLength_ok` using existing checked helpers, without importing Tree proofs. -/
theorem gindex_length_ok (index : U128) (depth : U32)
    (measured : NyxSszRust.gindex.length index = RustM.ok (.Ok depth)) :
    depth.val = Nat.log2 index.val ∧ 1 ≤ index.val ∧ depth.val ≠ 0 := by
  have agrees : gindexResultAgrees UScalar.val U32.max (.Ok depth)
      (Ssz.gindexLength index.val) := by
    simpa only [measured, spec_ok] using gindex_length_refines index
  have reference : Ssz.gindexLength index.val = .ok depth.val :=
    gindex_success_reference UScalar.val U32.max depth
      (Ssz.gindexLength index.val) agrees
  have lower : 2 ≤ index.val := by
    by_cases zero : index.val = 0
    · rw [zero, reference_length_nonroot 0 (by decide), Ssz.gindexDepth_zero] at reference
      cases reference
    by_cases root : index.val = 1
    · rw [root, Ssz.gindexLength_root] at reference
      cases reference
    omega
  rw [gindex_length_positive index.val lower] at reference
  have depthValue : depth.val = index.val.log2 := (Except.ok.inj reference).symm
  have positive := index_depth_positive index.val lower
  exact ⟨depthValue, by omega, by rw [depthValue]; exact Nat.ne_of_gt positive⟩

end NyxSszVerification
