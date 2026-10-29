"""Keep explicit proof skips narrow, stale-sensitive and fail closed."""
from copy import deepcopy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import current_refinement as current


class CurrentRefinementTests(unittest.TestCase):
    def scope(self):
        return current.load_scope()

    def load(self, value):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "scope.json"
            path.write_text(json.dumps(value))
            return current.load_scope(path)

    def test_should_classify_every_current_operation_without_overlapping_proof_claims(self):
        scope = self.scope()
        self.assertEqual(len(current.current_targets(scope)), 24)
        self.assertEqual(len(scope["skipped"]), 93)
        self.assertEqual(len(scope["skipped_traits"]), 17)
        current.validate_roots()
        self.assertFalse(set(scope["proven"]) & set(scope["skipped"]))

    def test_should_refuse_missing_or_ambiguous_skip_evidence(self):
        scope = self.scope()
        invalid = []
        for key in ("issues", "tool_revisions", "derive_status", "unproved_trait_members"):
            value = deepcopy(scope)
            value.pop(key)
            invalid.append(value)
        value = deepcopy(scope); value["skipped"].append(value["proven"][0]); invalid.append(value)
        value = deepcopy(scope); value["proven"].append(value["proven"][0]); invalid.append(value)
        value = deepcopy(scope); value["format_version"] = True; invalid.append(value)
        value = deepcopy(scope); value["issues"]["metadata"] = "https://example.com/24"; invalid.append(value)
        for value in invalid:
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.load(value)

    def test_should_reject_substituted_source_roots_and_unregistered_theorems(self):
        with patch.object(Path, "read_text", return_value="pub fn fake() {}"):
            with self.assertRaisesRegex(ValueError, "production"):
                current.validate_roots()
        scope = self.scope(); scope["proven"].append("nyx_ssz::unknown::function")
        with self.assertRaisesRegex(ValueError, "registered theorem"):
            current.current_targets(scope)

    def test_should_accept_only_the_recorded_aeneas_metadata_refusal(self):
        message = "Returns inside of nested loops are not supported yet type_info.rs PrePasses.ml, line 662"
        cases = [(2, message, True), (0, "", False), (1, message, False), (2, "different error", False)]
        for code, output, accepted in cases:
            with self.subTest(code=code, output=output), tempfile.TemporaryDirectory() as temporary:
                directory = Path(temporary)
                result = subprocess.CompletedProcess([], code, stdout=output, stderr="")
                with patch.object(current, "charon_root", return_value=directory/"code.llbc"), \
                        patch.object(current, "aeneas_command", return_value=["aeneas"]), \
                        patch.object(current.subprocess, "run", return_value=result), patch("builtins.print"):
                    if accepted:
                        current.probe_metadata({}, self.scope(), directory)
                    else:
                        with self.assertRaises(ValueError):
                            current.probe_metadata({}, self.scope(), directory)


if __name__ == "__main__":
    unittest.main()
