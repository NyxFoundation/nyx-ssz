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

end NyxSszVerification
