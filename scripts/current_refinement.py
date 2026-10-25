#!/usr/bin/env python3
"""Prove the current extractable slice and explicitly report issue-linked skips."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

import verify as legacy
from extracted_modules import split_model
from historical_models import archive_omitted
import tomllib

ROOT = legacy.ROOT
VERIFICATION = legacy.VERIFICATION
SCOPE = VERIFICATION / "current-scope.json"
MODULES = {f"NyxSszRust/{name}.lean" for name in ("Gindex", "Merkle", "Multiproof")}


def load_scope(path=SCOPE):
    scope = json.loads(path.read_text())
    if type(scope.get("format_version")) is not int or scope["format_version"] != 1:
        raise ValueError("unsupported current proof scope")
    for key in ("proven", "skipped", "skipped_traits"):
        values = scope.get(key)
        if not isinstance(values, list) or not values or not all(isinstance(v, str) for v in values):
            raise ValueError("proof scope requires explicit nonempty identifiers")
        if len(values) != len(set(values)):
            raise ValueError("duplicate proof scope identifiers")
    if set(scope["proven"]) & set(scope["skipped"]):
        raise ValueError("proved and skipped scopes overlap")
    for url in scope.get("issues", {}).values():
        if not re.fullmatch(r"https://github\.com/NyxFoundation/nyx-ssz/issues/[1-9][0-9]*", url):
            raise ValueError("proof skips require canonical issue links")
    if set(scope.get("issues", {})) != {"metadata", "remaining", "rustcrypto", "direct_encoding", "bulk_decode"}:
        raise ValueError("proof skip issues are missing")
    if set(scope.get("tool_revisions", {})) != {"aeneas_revision", "charon_revision"}:
        raise ValueError("proof skip tool revisions are missing")
    if not isinstance(scope.get("derive_status"), str) or not scope["derive_status"].startswith("unproved;"):
        raise ValueError("derive transformation proof status must be explicit")
    required_crypto = {f"nyx_ssz::{module}::{name}" for module, name in (
        ("sha256", "hash_pair"), ("merkle", "merkleize"),
        ("merkle", "calculate_merkle_root"), ("merkle", "verify_merkle_proof"),
        ("multiproof", "calculate_merkle_root"), ("multiproof", "verify_merkle_proof"))}
    crypto = scope.get("rustcrypto_dependent_skips", [])
    if not isinstance(crypto, list) or not all(isinstance(name, str) for name in crypto):
        raise ValueError("RustCrypto-dependent proof skips must be identifiers")
    if set(crypto) != required_crypto or len(crypto) != 6:
        raise ValueError("RustCrypto-dependent proof skips must be explicit")
    if not required_crypto <= set(scope["skipped"]):
        raise ValueError("RustCrypto-dependent APIs are not currently proved")
    for key in ("rustcrypto_source_sha256", "rustcrypto_probe_lock_sha256"):
        if not re.fullmatch(r"[0-9a-f]{64}", str(scope.get(key, ""))):
            raise ValueError("RustCrypto skip evidence fingerprint is missing")
    validate_unproved_members(scope)
    return scope


def validate_unproved_members(scope):
    expected = ["nyx_ssz::codec::Ssz::encoded_len", "nyx_ssz::codec::Ssz::encode_into",
                "nyx_ssz::codec::Ssz::fixed_sequence_bytes",
                "nyx_ssz::codec::Ssz::decode_fixed_sequence"]
    if scope.get("unproved_trait_members") != expected:
        raise ValueError("extended trait member laws must remain explicitly unproved")


def validate_roots():
    roots = {
        "current_gindex.rs": ("//! Current hash-independent production files, preserving public module paths.\n", ["gindex", "merkle/error", "multiproof/helpers"]),
        "current_metadata.rs": ("//! Extract the actual shared metadata implementation without Cargo host tools.\n", ["error", "type_info"]),
    }
    for name, (header, modules) in roots.items():
        expected = header + "".join(f'#[path = "../crates/nyx-ssz/src/{module}.rs"]\npub mod {module.split("/")[0]};\n' for module in modules)
        if (VERIFICATION / name).read_text() != expected:
            raise ValueError("extraction roots must include the actual production files only")


def current_targets(scope):
    registered = {row["rust"]: row for row in legacy.load_targets()}
    if set(scope["proven"]) - registered.keys():
        raise ValueError("current proof target lacks a registered theorem")
    return [registered[name] for name in scope["proven"]]


def check_source_bindings(document, scope):
    aliases = legacy.public_function_aliases(document)
    paths, index = document["paths"], document["index"]
    functions = {aliases.get(key, "::".join(row["path"])): key for key, row in paths.items()
                 if row["crate_id"] == 0 and row["kind"] == "function"}
    for name in scope["proven"]:
        module = "gindex.rs" if name.startswith("nyx_ssz::gindex::") else "multiproof/helpers.rs"
        identifier = functions[name]
        actual = index[identifier]["span"]["filename"]
        expected_name = name if module == "gindex.rs" else "nyx_ssz::multiproof::helpers::helper_indices"
        if "::".join(paths[identifier]["path"]) != expected_name:
            raise ValueError("public operation is not bound to its extracted production source")
        if (ROOT / actual).resolve() != ROOT / "crates/nyx-ssz/src" / module:
            raise ValueError("public operation is not bound to its extracted production source")
    for key, row in paths.items():
        if row["path"] != ["nyx_ssz", "merkle"]:
            continue
        for child in index[key]["inner"]["module"]["items"]:
            item = index[str(child)]
            imported = item["inner"].get("use")
            if not imported or imported["name"] != "ProofError" or item["visibility"] != "public":
                continue
            target = str(imported["id"])
            actual = index[target]["span"]["filename"]
            if (paths[target]["path"] == ["nyx_ssz", "merkle", "error", "ProofError"] and
                    (ROOT / actual).resolve() == ROOT / "crates/nyx-ssz/src/merkle/error.rs"):
                return
    raise ValueError("public proof error is not bound to its extracted production definition")


def check_crypto_evidence(scope):
    crypto = hashlib.sha256((ROOT / "crates/nyx-ssz/src/sha256.rs").read_bytes()).hexdigest()
    probe = hashlib.sha256((VERIFICATION / "rustcrypto-probe/Cargo.lock").read_bytes()).hexdigest()
    manifest = tomllib.loads((ROOT / "crates/nyx-ssz/Cargo.toml").read_text())
    if crypto != scope["rustcrypto_source_sha256"] or probe != scope["rustcrypto_probe_lock_sha256"]:
        raise ValueError("RustCrypto proof skip evidence changed")
    if manifest["dependencies"].get("sha2") != "=0.10.9":
        raise ValueError("RustCrypto backend dependency changed")


def check_scope(scope, pins):
    if scope["tool_revisions"] != {key: pins["tools"][key] for key in scope["tool_revisions"]}:
        raise ValueError("proof skip tool pins changed")
    digest = hashlib.sha256((ROOT / "crates/nyx-ssz/src/type_info.rs").read_bytes()).hexdigest()
    if digest != scope["metadata_source_sha256"]:
        raise ValueError("metadata changed; reassess the known unsupported skip")
    check_crypto_evidence(scope)
    contracts = legacy.load_contracts(VERIFICATION / "traits.json")
    historical = {"TYPE", "DecodeState", "encode", "decode", "hash_tree_root"}
    added = {f"{contract['rust']}::{name}" for contract in contracts
             for name in set(contract["members"]) - historical}
    if added != set(scope["unproved_trait_members"]):
        raise ValueError("extended trait member proof status is incomplete")
    directory = ROOT / "target" / "api-coverage"
    legacy.run(["cargo", "rustdoc", "--lib", "--package", "nyx-ssz", "--target-dir", directory,
                "--", "--output-format", "json", "-Z", "unstable-options"])
    document = json.loads((directory / "doc" / "nyx_ssz.json").read_text())
    actual = legacy.api_functions(document, contracts)
    check_source_bindings(document, scope)
    if actual != set(scope["proven"]) | set(scope["skipped"]):
        raise ValueError("public APIs must be exactly proved or explicitly issue-linked skipped")
    instances = {instance["lean"] for contract in contracts for instance in contract["instances"]}
    if instances != set(scope["skipped_traits"]):
        raise ValueError("trait dictionary proof status is incomplete")
    return current_targets(scope)


def charon_root(pins, destination, source, names):
    destination.mkdir(parents=True, exist_ok=True)
    llbc = destination / "nyx_ssz.llbc"
    arguments = [legacy.tool_path("charon", pins), "rustc", "--preset=aeneas", "--precise-drops",
                 "--sysroot", legacy.miri_sysroot(pins)]
    for name in names:
        arguments.extend(["--start-from", name])
    arguments.extend(["--dest-file", llbc, "--", source, "--crate-name", "nyx_ssz",
                      "--crate-type", "lib", "-A", "dead_code"])
    legacy.run(arguments)
    return llbc


def aeneas_command(pins, destination, llbc):
    return [str(item) for item in [legacy.tool_path("aeneas", pins), "-backend", "lean",
            "-namespace", "NyxSszRust", "-checks", "-abort-on-error", "-warnings-as-errors",
            "-no-progress-bar", "-dest", destination, llbc]]


def extract_core(pins, targets, destination):
    names = [row["rust"] for row in targets]
    llbc = charon_root(pins, destination, "verification/current_gindex.rs", names)
    legacy.assert_safe_drops(json.loads(llbc.read_text()))
    legacy.run(aeneas_command(pins, destination, llbc))
    artifacts = split_model((destination / "NyxSsz.lean").read_text())
    legacy.assert_extracted_targets(artifacts, targets)
    selected = {name: text.encode() for name, text in artifacts.items() if name != "NyxSszRust.lean"}
    if selected.keys() != MODULES:
        raise ValueError("current extraction dependency closure changed")
    return selected


def probe_metadata(pins, scope, destination):
    names = [f"nyx_ssz::type_info::{name}" for name in ("fixed_size", "is_basic", "is_compatible", "validate")]
    llbc = charon_root(pins, destination, "verification/current_metadata.rs", names)
    result = subprocess.run(aeneas_command(pins, destination, llbc), cwd=ROOT,
                            text=True, capture_output=True, check=False)
    output = result.stdout + result.stderr
    (destination / "aeneas.log").write_text(output)
    if result.returncode == 0:
        raise ValueError("metadata extraction now succeeds; remove skip and prove its laws")
    expected = ("Returns inside of nested loops are not supported yet",
                "type_info.rs", "PrePasses.ml, line 662")
    if result.returncode != 2 or not all(message in output for message in expected):
        raise ValueError("unexpected metadata extraction failure; not an approved skip")
    print(f"SKIPPED metadata equivalence (not proved): {scope['issues']['metadata']}")


def check_proofs(targets, directory):
    legacy.run(["lake", "build", "NyxSszVerification.CurrentAudit"], cwd=VERIFICATION)
    lines = ["import NyxSszVerification.CurrentAudit"]
    lines.extend(f"check_current_refinement {row['lean']} via {row['proof']}" for row in targets)
    path = directory / "CurrentTargets.lean"
    path.write_text("\n".join(lines) + "\n")
    legacy.run(["lake", "env", "lean", path], cwd=VERIFICATION)
    guards = {
        "Unrelated": (f"check_current_refinement {targets[0]['lean']} via {targets[1]['proof']}\n", "does not mention"),
        "ForeignAxiom": ("namespace NyxSszVerification\naxiom current_forbidden : False\nend NyxSszVerification\naudit_current_refinements\n", "forbidden axiom"),
        "NativeEvaluation": ("namespace NyxSszVerification\ntheorem nativeCurrent : (List.range 100).length = 100 := by native_decide\nend NyxSszVerification\naudit_current_refinements\n", "forbidden axiom"),
    }
    for name, (source, expected) in guards.items():
        path = directory / f"{name}.lean"
        path.write_text("import NyxSszVerification.CurrentAudit\n" + source)
        result = subprocess.run(["lake", "env", "lean", str(path)], cwd=VERIFICATION,
                                text=True, capture_output=True, check=False)
        if result.returncode == 0 or expected not in result.stdout + result.stderr:
            raise ValueError("current proof rejection guard failed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("extract", "check", "probe-metadata"))
    arguments = parser.parse_args()
    pins, scope = legacy.load_pins(), load_scope()
    legacy.validate_pins(pins)
    legacy.validate_tools(pins)
    validate_roots()
    targets = check_scope(scope, pins)
    with tempfile.TemporaryDirectory(prefix="nyx-current-proof-") as temporary:
        directory = Path(temporary)
        if arguments.command == "probe-metadata":
            probe_metadata(pins, scope, directory / "metadata")
            return
        artifacts = extract_core(pins, targets, directory / "core")
        if arguments.command == "extract":
            for relative, content in artifacts.items():
                path = VERIFICATION / relative
                if path.stem in {"Merkle", "Multiproof"}:
                    archive_omitted(path, content.decode())
                path.write_bytes(content)
            return
        expected = {name: (VERIFICATION / name).read_bytes() for name in MODULES}
        legacy.assert_same_artifact(artifacts, expected)
        probe_metadata(pins, scope, directory / "metadata")
        print(f"SKIPPED RustCrypto-dependent equivalence (not proved): {scope['issues']['rustcrypto']}")
        print(f"UNPROVED direct encoding member laws: {scope['issues']['direct_encoding']}")
        print(f"UNPROVED bulk decoding member laws: {scope['issues']['bulk_decode']}")
        check_proofs(targets, directory)
        legacy.validate_dependencies(pins)
    print(f"PROVED current-source slice: {len(targets)} operations (not full-library equivalence).")
    print(f"UNPROVED/SKIPPED: {len(scope['skipped'])} APIs, {len(scope['skipped_traits'])} dictionaries and derive transformations. {scope['issues']['remaining']}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError):
        print("current-source refinement failed; no new proof success is claimed", file=sys.stderr)
        sys.exit(1)
