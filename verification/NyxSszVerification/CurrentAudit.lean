import Lean
import NyxSszVerification.CurrentBoolean
import NyxSszVerification.CurrentUnsigned
import NyxSszVerification.Gindex
import NyxSszVerification.Multiproof.Collect

open Lean Elab Command

/-- Only kernel-checked theorems over the actual registered operation qualify. -/
elab "check_current_refinement " target:ident " via " claim:ident : command => do
  let targetName := target.getId
  unless (← getEnv).contains targetName do
    throwError "unknown current extraction target {targetName}"
  let claimInfo ← getConstInfo claim.getId
  match claimInfo with
  | .thmInfo _ => pure ()
  | _ => throwError "refinement claim is not a theorem"
  unless claimInfo.type.getUsedConstants.contains targetName do
    throwError "refinement theorem does not mention the registered target"
  for assumption in (← Lean.collectAxioms claim.getId) do
    unless [``propext, ``Classical.choice, ``Quot.sound].contains assumption do
      throwError "refinement depends on forbidden axiom {assumption}"

/-- Audit generated definitions and imported proof infrastructure transitively. -/
elab "audit_current_refinements" : command => do
  let environment ← getEnv
  for entry in environment.constants.toList do
    let declName := entry.1
    let publicName := (privateToUserName? declName).getD declName
    if (`NyxSszRust).isPrefixOf publicName || (`NyxSszVerification).isPrefixOf publicName then
      for assumption in (← Lean.collectAxioms declName) do
        unless [``propext, ``Classical.choice, ``Quot.sound].contains assumption do
          throwError "current proof environment depends on forbidden axiom {assumption}"

audit_current_refinements
