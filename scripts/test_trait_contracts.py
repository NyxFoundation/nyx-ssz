"""Trait declarations and all local instances require explicit proof coverage."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from trait_contracts import load_contracts, validate_traits, assert_extracted_instances

CONTRACT = {
    "rust": "nyx_ssz::codec::Ssz", "law": "NyxSszVerification.CodecLaw",
    "modules": ["nyx_ssz::codec"],
    "members": {"TYPE": "assoc_const", "encode": "function", "DecodeState": "assoc_type"},
    "instances": [{"type": "bool", "lean": "NyxSszRust.Bool.Insts.Ssz",
                   "proof": "NyxSszVerification.bool_law"}],
}


def document():
    return {"paths": {"1": {"path": ["nyx_ssz", "codec", "Ssz"]}}, "index": {
        "1": {"crate_id": 0, "visibility": "public", "name": "Ssz",
              "inner": {"trait": {"items": [2, 3, 7], "implementations": [4]}}},
        "2": {"name": "TYPE", "inner": {"assoc_const": {}}},
        "3": {"name": "encode", "inner": {"function": {}}},
        "4": {"inner": {"impl": {"for": {"primitive": "bool"}, "items": [5, 6, 8],
              "generics": {"params": [], "where_predicates": []}}}},
        "5": {"name": "TYPE", "inner": {"assoc_const": {}}},
        "6": {"name": "encode", "inner": {"function": {}}},
        "7": {"name": "DecodeState", "inner": {"assoc_type": {}}},
        "8": {"name": "DecodeState", "inner": {"assoc_type": {}}},
    }}


def generic_document():
    data = document()
    impl = data["index"]["4"]["inner"]["impl"]
    impl["for"] = {"resolved_path": {"path": "crate::collections::Vector", "id": 10, "args": {
        "angle_bracketed": {"args": [{"type": {"generic": "T"}},
            {"const": {"expr": "N", "value": None, "is_literal": False}}], "constraints": []}}}}
    impl["generics"] = {"params": [
        {"name": "T", "kind": {"type": {"bounds": [{"trait_bound": {
            "trait": {"path": "Ssz", "id": 1, "args": None}, "generic_params": [], "modifier": "none"}}],
            "default": None, "is_synthetic": False}}},
        {"name": "N", "kind": {"const": {"type": {"primitive": "usize"}, "default": None}}},
    ], "where_predicates": []}
    return data


class TraitContractTests(unittest.TestCase):
    def load(self, value):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "traits.json"
            path.write_text(json.dumps(value))
            return load_contracts(path)

    def test_should_require_exact_declaration_and_instance_inventories(self):
        contracts = self.load([CONTRACT])
        self.assertEqual(validate_traits(document(), contracts), {"2", "3", "5", "6", "7", "8"})
        for mutation in ("trait", "member", "instance"):
            data = document()
            if mutation == "trait":
                contracts = []
            elif mutation == "member":
                data["index"]["3"]["name"] = "unregistered"
            else:
                data["index"]["4"]["inner"]["impl"]["for"] = {"primitive": "u8"}
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                validate_traits(data, contracts)
            contracts = self.load([CONTRACT])

    def test_should_inventory_inherited_methods_only_when_the_declaration_provides_a_body(self):
        data = document()
        data["index"]["3"]["inner"]["function"]["has_body"] = True
        implementation = data["index"]["4"]["inner"]["impl"]
        implementation["items"].remove(6)
        implementation["provided_trait_methods"] = ["encode"]
        self.assertEqual(validate_traits(data, [CONTRACT]), {"2", "3", "5", "7", "8"})
        for names in (["TYPE"], ["encode", "encode"], ["unknown"], [None]):
            invalid = copy.deepcopy(data)
            invalid["index"]["4"]["inner"]["impl"]["provided_trait_methods"] = names
            with self.subTest(names=names), self.assertRaises(ValueError):
                validate_traits(invalid, [CONTRACT])
        overridden = copy.deepcopy(data)
        overridden["index"]["4"]["inner"]["impl"]["items"].append(6)
        self.assertEqual(validate_traits(overridden, [CONTRACT]), {"2", "3", "5", "6", "7", "8"})
        data["index"]["3"]["inner"]["function"]["has_body"] = False
        with self.assertRaisesRegex(ValueError, "declared default"):
            validate_traits(data, [CONTRACT])

    def test_should_allow_inherited_associated_constants_only_when_they_have_defaults(self):
        data = document()
        data["index"]["1"]["inner"]["trait"]["items"].append(9)
        data["index"]["9"] = {
            "name": "TRUSTED",
            "inner": {"assoc_const": {"value": "false"}},
        }
        contract = copy.deepcopy(CONTRACT)
        contract["members"]["TRUSTED"] = "assoc_const"
        self.assertIn("9", validate_traits(data, [contract]))
        data["index"]["9"]["inner"]["assoc_const"]["value"] = None
        with self.assertRaisesRegex(ValueError, "member inventory"):
            validate_traits(data, [contract])

    def test_should_inventory_generic_type_and_const_arguments_when_a_collection_is_registered(self):
        contracts = copy.deepcopy([CONTRACT])
        contracts[0]["instances"][0]["type"] = "crate::collections::Vector<T,N>"
        data = generic_document()
        self.assertEqual(validate_traits(data, self.load(contracts)), {"2", "3", "5", "6", "7", "8"})

    def test_should_reject_specialized_or_constrained_instances_when_a_generic_codec_is_registered(self):
        contracts = copy.deepcopy([CONTRACT])
        contracts[0]["instances"][0]["type"] = "crate::collections::Vector<T,N>"
        for mutation in ("concrete-type", "constant", "extra-bound", "where", "unused", "duplicate"):
            data = generic_document()
            impl = data["index"]["4"]["inner"]["impl"]
            arguments = impl["for"]["resolved_path"]["args"]["angle_bracketed"]["args"]
            if mutation == "concrete-type":
                arguments[0] = {"type": {"primitive": "u8"}}
            elif mutation == "constant":
                arguments[1]["const"] = {"expr": "2", "value": "2", "is_literal": True}
            elif mutation == "extra-bound":
                impl["generics"]["params"][0]["kind"]["type"]["bounds"].append({"outlives": "'static"})
            elif mutation == "where":
                impl["generics"]["where_predicates"] = [{"unsupported": "predicate"}]
            elif mutation == "unused":
                impl["generics"]["params"].append({"name": "U", "kind": {"type": {"bounds": []}}})
            else:
                arguments[1] = arguments[0]
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                validate_traits(data, self.load(contracts))

    def test_should_reject_missing_or_miskinded_associated_types(self):
        contracts = self.load([CONTRACT])
        for owner, field, identifier in (("1", "trait", 7), ("4", "impl", 8)):
            for change in ("missing", "kind"):
                data = document()
                if change == "missing":
                    data["index"][owner]["inner"][field]["items"].remove(identifier)
                else:
                    data["index"][str(identifier)]["inner"] = {"function": {}}
                with self.subTest(owner=owner, change=change), self.assertRaises(ValueError):
                    validate_traits(data, contracts)

    def test_should_reject_duplicate_traits_instances_and_malformed_identifiers(self):
        duplicate_instance = copy.deepcopy(CONTRACT)
        duplicate_instance["instances"].append(duplicate_instance["instances"][0])
        malformed = {**CONTRACT, "law": "invalid\naxiom injected"}
        vacuous = {**CONTRACT, "law": "Eq"}
        for value in ([CONTRACT, CONTRACT], [duplicate_instance], [malformed], [vacuous], [{}], [None]):
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.load(value)

    def test_should_reject_stale_contracts_for_removed_traits(self):
        with self.assertRaisesRegex(ValueError, "trait"):
            validate_traits({"paths": {}, "index": {}}, [CONTRACT])

    def test_should_require_the_exact_extracted_trait_dictionary_inventory(self):
        source = (
            "/-- Trait implementation: [nyx_ssz::codec::{impl nyx_ssz::codec::Ssz<()> for bool}] -/\n"
            "def Bool.Insts.Ssz : codec.Ssz Bool := dictionary\n"
        )
        assert_extracted_instances({"Codec.lean": source}, [CONTRACT])
        for artifacts in ({}, {"Codec.lean": source.replace("Bool.Insts.Ssz", "U8.Insts.Ssz")}):
            with self.subTest(artifacts=artifacts), self.assertRaises(ValueError):
                assert_extracted_instances(artifacts, [CONTRACT])


if __name__ == "__main__":
    unittest.main()
