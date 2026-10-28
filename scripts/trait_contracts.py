"""Inventory public traits and require proved, concretely extracted local instances."""
import json
from pathlib import Path
import re
from extracted_modules import NAMESPACE, declaration_units, declared_names

IDENTIFIER = r"[A-Za-z_][A-Za-z_0-9]*"
IMPLEMENTATION = re.compile(
    rf"^/-- Trait implementation: \[[^\n]*\{{impl ({IDENTIFIER}(?:::{IDENTIFIER})*)(?=[<\s])"
)


def qualified(value, separator):
    return isinstance(value, str) and all(
        re.fullmatch(IDENTIFIER, part) for part in value.split(separator)
    )


def validate_contract(contract):
    if not isinstance(contract, dict) or set(contract) != {"rust", "law", "members", "instances", "modules"}:
        raise ValueError("invalid trait contract fields")
    if not qualified(contract["rust"], "::") or not qualified(contract["law"], "."):
        raise ValueError("invalid trait contract identifier")
    if (contract["rust"], contract["law"]) != (
            "nyx_ssz::codec::Ssz", "NyxSszVerification.CodecLaw"):
        raise ValueError("unsupported trait correctness law")
    modules = contract["modules"]
    if (not isinstance(modules, list) or not modules
            or any(not qualified(module, "::") for module in modules)
            or len(set(modules)) != len(modules)):
        raise ValueError("invalid trait extraction modules")
    members = contract["members"]
    if not isinstance(members, dict) or not members:
        raise ValueError("nonempty trait member inventory required")
    if any(not qualified(name, "::") or kind not in ("function", "assoc_const", "assoc_type")
           for name, kind in members.items()):
        raise ValueError("unsupported trait member")
    instances = contract["instances"]
    if not isinstance(instances, list) or not instances:
        raise ValueError("nonempty trait instance inventory required")
    for instance in instances:
        validate_instance(instance)
    for field in ("type", "lean", "proof"):
        if len({instance[field] for instance in instances}) != len(instances):
            raise ValueError("duplicate trait instance")


def validate_instance(instance):
    if not isinstance(instance, dict) or set(instance) != {"type", "lean", "proof"}:
        raise ValueError("invalid trait instance fields")
    signature = instance["type"]
    pattern = rf"({IDENTIFIER}(?:::{IDENTIFIER})*)(?:<({IDENTIFIER}(?:,{IDENTIFIER})*)>)?"
    match = re.fullmatch(pattern, signature) if isinstance(signature, str) else None
    if match is None or (match[2] and len(set(match[2].split(","))) != len(match[2].split(","))):
        raise ValueError("unsupported trait instance type")
    if not qualified(instance["lean"], ".") or not qualified(instance["proof"], "."):
        raise ValueError("invalid trait instance identifier")


def load_contracts(path):
    contracts = json.loads(Path(path).read_text())
    if not isinstance(contracts, list):
        raise ValueError("trait contracts must be a list")
    for contract in contracts:
        validate_contract(contract)
    if len({contract["rust"] for contract in contracts}) != len(contracts):
        raise ValueError("duplicate trait contract")
    return contracts


def member_inventory(index, identifiers):
    members = {}
    for identifier in identifiers:
        item = index[str(identifier)]
        if item["name"] in members or len(item["inner"]) != 1:
            raise ValueError("ambiguous trait member")
        members[item["name"]] = next(iter(item["inner"]))
    return members


def generic_argument(argument, parameters, trait_identifier):
    if set(argument) == {"type"} and set(argument["type"]) == {"generic"}:
        name = argument["type"]["generic"]
        kind = parameters.get(name, {}).get("kind", {}).get("type")
        if (kind is None or kind.get("default") is not None
                or kind.get("is_synthetic") is not False or len(kind["bounds"]) != 1):
            raise ValueError("unsupported generic codec type parameter")
        bound = kind["bounds"][0].get("trait_bound", {})
        if (str(bound.get("trait", {}).get("id")) != str(trait_identifier)
                or bound.get("trait", {}).get("args") is not None
                or bound.get("generic_params") != [] or bound.get("modifier") != "none"):
            raise ValueError("generic child must have exactly the registered trait bound")
        return name
    if set(argument) == {"const"}:
        constant = argument["const"]
        name = constant["expr"]
        kind = parameters.get(name, {}).get("kind", {}).get("const")
        if (kind is None or kind.get("type") != {"primitive": "usize"}
                or kind.get("default") is not None or constant.get("value") is not None
                or constant.get("is_literal") is not False):
            raise ValueError("generic codec const must be an unconstrained usize parameter")
        return name
    raise ValueError("specialized codec arguments are not a generic constructor")


def instance_type(implementation, trait_identifier):
    value, generics = implementation["for"], implementation["generics"]
    if generics["where_predicates"]:
        raise ValueError("extra generic codec constraints are unsupported")
    parameters = {parameter["name"]: parameter for parameter in generics["params"]}
    if len(parameters) != len(generics["params"]):
        raise ValueError("duplicate generic codec parameters")
    if set(value) == {"primitive"} and not parameters:
        return value["primitive"]
    if set(value) != {"resolved_path"}:
        raise ValueError("trait instance type needs proof inventory support")
    reference = value["resolved_path"]
    if reference["args"] is None and not parameters:
        return reference["path"]
    angle = (reference["args"] or {}).get("angle_bracketed")
    if angle is None or angle["constraints"]:
        raise ValueError("unsupported generic codec argument form")
    names = [generic_argument(argument, parameters, trait_identifier) for argument in angle["args"]]
    if len(set(names)) != len(names) or set(names) != set(parameters):
        raise ValueError("generic codec parameters must each occur exactly once")
    return reference["path"] + "<" + ",".join(names) + ">"


def validate_implementations(index, trait_identifier, declaration, contract):
    discovered, items = [], set()
    for identifier in declaration["implementations"]:
        implementation = index[str(identifier)]["inner"]["impl"]
        discovered.append(instance_type(implementation, trait_identifier))
        defaults = {index[str(item)]["name"] for item in declaration["items"]
                    if (index[str(item)]["inner"].get("function", {}).get("has_body") is True or
                        index[str(item)]["inner"].get("assoc_const", {}).get("value") is not None)}
        method_defaults = {index[str(item)]["name"] for item in declaration["items"]
                           if index[str(item)]["inner"].get("function", {}).get("has_body") is True}
        provided = implementation.get("provided_trait_methods", [])
        if not isinstance(provided, list) or any(not isinstance(name, str) for name in provided):
            raise ValueError("invalid inherited trait methods")
        if len(provided) != len(set(provided)) or not set(provided) <= method_defaults:
            raise ValueError("inherited trait method has no declared default body")
        actual_members = member_inventory(index, implementation["items"])
        # Inherited defaults are absent; explicit overrides remain in the impl inventory.
        expected_members = {name: kind for name, kind in contract["members"].items()
                            if name not in defaults or name in actual_members}
        if actual_members != expected_members:
            raise ValueError("trait implementation member inventory differs")
        items.update(str(value) for value in implementation["items"])
    expected = [instance["type"] for instance in contract["instances"]]
    if sorted(discovered) != sorted(expected):
        raise ValueError("trait implementation inventory differs")
    return items


def validate_traits(document, contracts):
    registered = {contract["rust"]: contract for contract in contracts}
    reserved = set()
    index = document.get("index", {})
    for identifier, item in index.items():
        declaration = item["inner"].get("trait")
        if item.get("crate_id") != 0 or item.get("visibility") != "public" or declaration is None:
            continue
        name = "::".join(document["paths"].get(identifier, {}).get("path", []))
        if name not in registered:
            raise ValueError("public trait has no registered correctness contract")
        contract = registered.pop(name)
        if member_inventory(index, declaration["items"]) != contract["members"]:
            raise ValueError("trait declaration member inventory differs")
        reserved.update(str(value) for value in declaration["items"])
        reserved.update(validate_implementations(index, identifier, declaration, contract))
    if registered:
        raise ValueError("registered trait is not a public declaration")
    return reserved


def assert_extracted_instances(artifacts, contracts):
    registered = {contract["rust"] for contract in contracts}
    expected = {instance["lean"] for contract in contracts for instance in contract["instances"]}
    discovered = set()
    for source in artifacts.values():
        for unit in declaration_units(source):
            match = IMPLEMENTATION.match(unit.lstrip())
            if match is None or not match[1].startswith("nyx_ssz::"):
                continue
            names = declared_names(unit)
            if match[1] not in registered or len(names) != 1:
                raise ValueError("unregistered or ambiguous extracted trait instance")
            discovered.update(f"{NAMESPACE}.{name}" for name in names)
    if discovered != expected:
        raise ValueError("extracted trait dictionary inventory differs")
