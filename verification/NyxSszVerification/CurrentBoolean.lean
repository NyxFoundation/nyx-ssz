import NyxSszRust.CurrentScalar
import NyxSszVerification.Representation
import Ssz.Codec.Deserialize

open Aeneas Aeneas.Std

namespace NyxSszVerification

/-- Classify reference Boolean faults. -/
def booleanResult (result : Except Ssz.Err Ssz.Value) :
    core.result.Result Bool NyxSszRust.scalar.ScalarError :=
  match result with
  | .ok (.bool value) => .Ok value
  | .error (.scope _ _) => .Err .InvalidLength
  | _ => .Err .InvalidBoolean

/-- Exact encoding and termination for the current extracted implementation. -/
theorem encode_bool_refines (value : Bool) :
    ∃ encoded, NyxSszRust.scalar.encode_bool value = RustM.ok encoded ∧
      Ssz.serialize .bool (.bool value) = .ok (bytesToSpec encoded.val) := by
  cases value <;> simp [NyxSszRust.scalar.encode_bool, Ssz.serialize,
    bytesToSpec, byteToSpec, Array.make]

/-- Exact decoding of every slice without panic for the current implementation. -/
theorem decode_bool_refines (bytes : Slice U8) :
    NyxSszRust.scalar.decode_bool bytes =
      RustM.ok (booleanResult (Ssz.deserialize .bool (bytesToSpec bytes.val))) := by
  rcases bytes with ⟨bytes, bound⟩
  cases bytes with
  | nil =>
    simp [NyxSszRust.scalar.decode_bool, booleanResult, Ssz.deserialize,
      bytesToSpec, Slice.len]
    rfl
  | cons byte rest =>
    cases rest with
    | nil =>
      by_cases zero : byte = 0#u8
      · subst byte
        simp [NyxSszRust.scalar.decode_bool, booleanResult, Ssz.deserialize,
          bytesToSpec, Slice.len, Slice.index_usize, byteToSpec]
        rfl
      by_cases one : byte = 1#u8
      · subst byte
        simp [NyxSszRust.scalar.decode_bool, booleanResult, Ssz.deserialize,
          bytesToSpec, Slice.len, Slice.index_usize, byteToSpec]
        rfl
      have notZero : byteToSpec byte ≠ 0 := fun equal =>
        zero (byteToSpec_injective equal)
      have notOne : byteToSpec byte ≠ 1 := fun equal =>
        one (byteToSpec_injective equal)
      simp [NyxSszRust.scalar.decode_bool, booleanResult, Ssz.deserialize,
        bytesToSpec, Slice.len, Slice.index_usize]
      split <;> simp_all [UScalar.eq_equiv_bv_eq]
      rfl
    | cons next rest =>
      simp [NyxSszRust.scalar.decode_bool, booleanResult, Ssz.deserialize,
        bytesToSpec, Slice.len]
      rfl

end NyxSszVerification
