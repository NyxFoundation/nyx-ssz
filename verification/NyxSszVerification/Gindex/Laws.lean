import NyxSszVerification.Gindex.Arithmetic
import Ssz.Proofs.Merkle.Gindex

open Aeneas Aeneas.Std Aeneas.Std.WP

namespace NyxSszVerification

/-- Transfer `Ssz.gindexSibling_sibling` to two actual Rust calls, for every u128. -/
theorem gindex_sibling_sibling (index : U128) :
    (NyxSszRust.gindex.sibling index >>= NyxSszRust.gindex.sibling) = RustM.ok index := by
  obtain ⟨sibling, firstRun, firstValue⟩ := spec_imp_exists (gindex_sibling_refines index)
  obtain ⟨result, secondRun, secondValue⟩ := spec_imp_exists (gindex_sibling_refines sibling)
  have restored : result = index := by
    apply UScalar.eq_of_val_eq
    calc
      result.val = Ssz.gindexSibling sibling.val := secondValue
      _ = Ssz.gindexSibling (Ssz.gindexSibling index.val) := congrArg Ssz.gindexSibling firstValue
      _ = index.val := Ssz.gindexSibling_sibling index.val
  simp only [firstRun, bind_tc_ok, secondRun, restored]

/-- Transfer `Ssz.gindexSibling_half` through actual sibling and parent calls. -/
theorem gindex_sibling_half (index : U128) :
    (NyxSszRust.gindex.sibling index >>= NyxSszRust.gindex.parent)
      ⦃ parent => parent.val = index.val / 2 ⦄ := by
  apply spec_bind (gindex_sibling_refines index)
  intro sibling siblingValue
  apply spec_mono (gindex_parent_refines sibling)
  intro parent parentValue
  rw [parentValue, Ssz.gindexParent, siblingValue]
  exact Ssz.gindexSibling_half index.val

/-- Transfer `Ssz.gindexSibling_parity` without restricting the input index. -/
theorem gindex_sibling_parity (index : U128) :
    NyxSszRust.gindex.sibling index ⦃ sibling => sibling.val % 2 = 1 - index.val % 2 ⦄ := by
  apply spec_mono (gindex_sibling_refines index)
  intro sibling siblingValue
  rw [siblingValue]
  exact Ssz.gindexSibling_parity index.val

/-- Preserve exactly the even-index premise of `Ssz.gindexSibling_even`. -/
theorem gindex_sibling_even (index : U128) (even : index.val % 2 = 0) :
    NyxSszRust.gindex.sibling index ⦃ sibling =>
      index.val = 2 * (index.val / 2) ∧ sibling.val = 2 * (index.val / 2) + 1 ⦄ := by
  apply spec_mono (gindex_sibling_refines index)
  intro sibling siblingValue
  simpa only [siblingValue] using Ssz.gindexSibling_even even

/-- Preserve exactly the odd-index premise of `Ssz.gindexSibling_odd`. -/
theorem gindex_sibling_odd (index : U128) (odd : index.val % 2 = 1) :
    NyxSszRust.gindex.sibling index ⦃ sibling =>
      index.val = 2 * (index.val / 2) + 1 ∧ sibling.val = 2 * (index.val / 2) ⦄ := by
  apply spec_mono (gindex_sibling_refines index)
  intro sibling siblingValue
  simpa only [siblingValue] using Ssz.gindexSibling_odd odd

/-- Transfer `Ssz.gindexDepth_zero` to the exact production error, not a Rust panic. -/
theorem gindex_depth_zero :
    NyxSszRust.gindex.depth 0#u128 = RustM.ok (.Err .InvalidIndex) := by
  obtain ⟨actual, run, agrees⟩ := spec_imp_exists (gindex_depth_refines 0#u128)
  have reference : Ssz.gindexDepth (0#u128).val = .error (.notAGindex 0) :=
    Ssz.gindexDepth_zero
  rw [reference] at agrees
  have refused : actual = .Err .InvalidIndex := by
    cases actual with
    | Ok _ => simp [gindexResultAgrees] at agrees
    | Err error => cases error <;> simp_all [gindexResultAgrees, proofFaultAgrees]
  simpa only [refused] using run

/-- Transfer `Ssz.gindexLength_root` with its distinct root refusal category. -/
theorem gindex_length_root :
    NyxSszRust.gindex.length 1#u128 = RustM.ok (.Err .RootHasNoBranch) := by
  obtain ⟨actual, run, agrees⟩ := spec_imp_exists (gindex_length_refines 1#u128)
  have reference : Ssz.gindexLength (1#u128).val = .error .rootHasNoBranch :=
    Ssz.gindexLength_root
  rw [reference] at agrees
  have refused : actual = .Err .RootHasNoBranch := by
    cases actual with
    | Ok _ => simp [gindexResultAgrees] at agrees
    | Err error => cases error <;> simp_all [gindexResultAgrees, proofFaultAgrees]
  simpa only [refused] using run

end NyxSszVerification
