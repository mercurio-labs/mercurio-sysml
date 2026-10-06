"""Resolve pinned Xtext construction slots against effective runtime Ecore.

This is a static model contract, not a parser, linker, delegate, or validator.
The pinned Java exports are build-time inputs; production uses generated Rust.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from export_pilot_grammar_structure import validate_structure

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
GRAMMAR = PROFILE / "grammar.structure.extract.json"
ECORE = PROFILE / "ecore-effective.extract.json"
OUTPUT = PROFILE / "xtext-assignment-contracts.json"
RUST = ROOT / "crates/mercurio-sysml/src/xtext_assignment_contract_generated.rs"
SYSML = "https://www.omg.org/spec/SysML/20250201#//"
ECORE_NS = "http://www.eclipse.org/emf/2002/Ecore#//"


def ref(value):
    if not isinstance(value, dict) or set(value) != {"$ref"}:
        raise ValueError(f"Expected linked reference: {value!r}")
    return value["$ref"]


def name(uri):
    if uri.startswith(SYSML):
        return "SysML::" + uri.removeprefix(SYSML).replace("/", "::")
    if uri.startswith(ECORE_NS):
        return "Ecore::" + uri.removeprefix(ECORE_NS).replace("/", "::")
    raise ValueError("Unknown Ecore classifier URI: " + uri)


class EffectiveEcore:
    def __init__(self, doc):
        self.classes = {r["id"]: r for r in doc["classes"]}
        self.features = {(r["owner"], r["name"]): r for r in doc["features"]}
        if len(self.classes) != len(doc["classes"]) or len(self.features) != len(doc["features"]):
            raise ValueError("Duplicate effective Ecore identity")

    def ancestors(self, cls):
        seen, todo = set(), [cls]
        while todo:
            item = todo.pop()
            if item in seen:
                continue
            seen.add(item)
            if item in self.classes:
                todo.extend(self.classes[item]["super_types"])
        return seen

    def feature(self, cls, field):
        if cls not in self.classes:
            raise ValueError("Current Xtext type is not an Ecore class: " + name(cls))
        matches = [r for (owner, key), r in self.features.items()
                   if key == field and owner in self.ancestors(cls)]
        # A subtype redeclaration shadows its inherited feature.
        matches = [r for r in matches if not any(
            r["owner"] != other["owner"] and r["owner"] in self.ancestors(other["owner"])
            for other in matches)]
        if len(matches) != 1:
            raise ValueError(f"Missing or ambiguous Ecore feature {name(cls)}::{field}")
        return matches[0]


class Grammar:
    def __init__(self, doc):
        rules = [r for grammar in doc["grammars"] for r in grammar["rules"]]
        self.rules = {r["id"]: r for r in rules}
        if len(self.rules) != len(rules):
            raise ValueError("Duplicate qualified Xtext rule")
        self.returns = {}
        dependencies = {}
        for identity, rule in self.rules.items():
            declared = self.rule_type(identity)
            self.returns[identity] = {declared} if declared else set()
            dependencies[identity] = set()
            if rule["kind"] != "ParserRule":
                continue
            def collect(node):
                if node["kind"] == "Action":
                    self.returns[identity].add(ref(node["fields"]["type"]["fields"]["classifier"]))
                elif node["kind"] == "RuleCall":
                    dependencies[identity].add(ref(node["fields"]["rule"]))
                elif node["kind"] in ("Group", "Alternatives", "UnorderedGroup"):
                    for child in node["fields"]["elements"]:
                        collect(child)
            collect(rule["fields"]["alternatives"])
        self.unassigned_calls = {identity: sorted(calls) for identity, calls in dependencies.items()}
        while True:
            changed = False
            for identity, calls in dependencies.items():
                before = len(self.returns[identity])
                for call in calls:
                    self.returns[identity].update(self.returns[call])
                changed |= len(self.returns[identity]) != before
            if not changed:
                break

    def rule_type(self, identity):
        rule = self.rules.get(identity)
        if rule is None:
            raise ValueError("Unlinked Xtext rule: " + identity)
        type_ref = rule["fields"].get("type")
        return ref(type_ref["fields"]["classifier"]) if type_ref else None

    def result_types(self, identity):
        """Conservative fixed-point return set for unassigned calls."""
        if identity not in self.returns:
            raise ValueError("Unlinked Xtext rule: " + identity)
        return self.returns[identity]


def terminal(node, grammar):
    kind, fields = node["kind"], node["fields"]
    row = {"kind": kind}
    if kind == "RuleCall":
        row["rule"] = ref(fields["rule"])
        row["result_types"] = sorted(map(name, grammar.result_types(row["rule"])))
    elif kind == "CrossReference":
        row["type"] = name(ref(fields["type"]["fields"]["classifier"]))
        if fields.get("terminal"):
            row["terminal"] = terminal(fields["terminal"], grammar)
    elif kind == "Keyword":
        row["value"] = fields["value"]
    elif kind in ("Alternatives", "Group"):
        row["elements"] = [terminal(child, grammar) for child in fields["elements"]]
    else:
        row["unsupported"] = True
    return row


def terminal_dependencies(contract):
    kind = contract["kind"]
    if kind == "RuleCall":
        return {"rule_call_execution"}
    if kind == "CrossReference":
        return {"handwritten_scoping_and_linking"}
    if kind == "Keyword":
        return {"keyword_value_conversion"}
    if kind in ("Alternatives", "Group"):
        return {"terminal_branch_execution"}.union(
            *(terminal_dependencies(child) for child in contract["elements"]))
    return {"unsupported_terminal_execution"}


def bind(ecore, current, feature, operator):
    if not current:
        return None, "unknown_current_type"
    try:
        fields = [ecore.feature(cls, feature) for cls in sorted(current)]
    except ValueError as error:
        return None, str(error)
    if len({field["id"] for field in fields}) != 1:
        return None, "ambiguous_current_type_features"
    field = fields[0]
    if operator not in ("=", "+=", "?="):
        return None, "unsupported_assignment_operator"
    if operator == "+=" and field["upper_bound"] == 1:
        return None, "additive_assignment_to_singular_feature"
    if operator == "?=" and (field["kind"] != "attribute" or name(field["type"]) != "Ecore::EBoolean"):
        return None, "boolean_assignment_to_non_boolean_feature"
    return {key: (name(field[value]) if key in ("owner", "target") else field[value])
            for key, value in (("id", "id"), ("owner", "owner"), ("name", "name"),
                               ("kind", "kind"), ("target", "type"),
                               ("lower_bound", "lower_bound"), ("upper_bound", "upper_bound"),
                               ("containment", "containment"), ("derived", "derived"),
                               ("transient", "transient"), ("volatile", "volatile"))}, None


def build(grammar_doc, ecore_doc):
    validate_structure(grammar_doc)
    if grammar_doc["provenance"]["pilot_revision"] != ecore_doc["provenance"]["pilot_commit"]:
        raise ValueError("Grammar/Ecore Pilot revision mismatch")
    if grammar_doc["metamodel"]["sha256"] != ecore_doc["provenance"]["ecore_sha256"]:
        raise ValueError("Grammar/Ecore source hash mismatch")
    grammar, ecore = Grammar(grammar_doc), EffectiveEcore(ecore_doc)
    assignments, actions = [], []
    for rule in grammar.rules.values():
        if rule["kind"] != "ParserRule":
            continue
        initial = grammar.rule_type(rule["id"])
        def walk(node, current, context):
            kind, fields = node["kind"], node["fields"]
            context = context + [{"kind": kind, "cardinality": fields.get("cardinality"),
                                  "predicated": fields.get("predicated", False)}]
            if kind in ("Group", "UnorderedGroup"):
                for child in fields["elements"]:
                    current = walk(child, current, context)
                return current
            if kind == "Alternatives":
                return set().union(*(walk(child, current.copy(), context) for child in fields["elements"]))
            if kind == "Action":
                target = ref(fields["type"]["fields"]["classifier"])
                row = {"id": node["id"], "rule": rule["id"], "constructed_type": name(target),
                       "captured_types": sorted(map(name, current)), "feature": fields["feature"],
                       "operator": fields["operator"], "context": context, "span": node["span"]}
                row["ecore_feature"], row["unresolved"] = (
                    bind(ecore, {target}, fields["feature"], fields["operator"])
                    if fields["feature"] else (None, None))
                row["semantic_verified"] = False
                row["unverified_dependencies"] = (
                    ["action_capture_current_object"] if fields["feature"] else ["action_construction_execution"])
                actions.append(row)
                return {target}
            if kind == "Assignment":
                row = {"id": node["id"], "rule": rule["id"], "current_types": sorted(map(name, current)),
                       "feature": fields["feature"], "operator": fields["operator"],
                       "terminal": terminal(fields["terminal"], grammar),
                       "context": context, "span": node["span"]}
                row["ecore_feature"], row["unresolved"] = bind(ecore, current, fields["feature"], fields["operator"])
                dependencies = terminal_dependencies(row["terminal"])
                if grammar.unassigned_calls[rule["id"]]:
                    # Conservative rule-level inventory: branch reachability is
                    # still an Xtext parser-flow question.
                    dependencies.add("unassigned_rule_call_current_object_flow")
                row["semantic_verified"] = False
                row["unverified_dependencies"] = sorted(dependencies)
                assignments.append(row)
                return current
            if kind == "RuleCall":
                # An unassigned call can augment the current object or return
                # another one. Xtext runtime flow is not present in this AST;
                # bind later slots to the enclosing declared type/action type.
                # The JSON records this as an unverified flow dependency.
                return current
            if kind == "Keyword":
                return current
            raise ValueError(f"Unsupported parser node {kind}: {node['id']}")
        walk(rule["fields"]["alternatives"], {initial} if initial else set(), [])
    if len({r["id"] for r in assignments}) != len(assignments) or len({r["id"] for r in actions}) != len(actions):
        raise ValueError("Grammar node traversed more than once")
    unverified = Counter(dependency for row in assignments + actions
                         for dependency in row["unverified_dependencies"])
    return {
        "schema": "dev.mercurio.xtext-assignment-contracts.v1",
        "scope": "static Xtext construction slots; unassigned rule-call current-object flow, scoping, linking, delegates, and validation remain unverified/handwritten",
        "current_type_strategy": "enclosing declared Ecore type, changed only by local Xtext actions; unassigned rule calls do not change the static binding context",
        "counts": {"parser_rules": sum(r["kind"] == "ParserRule" for r in grammar.rules.values()),
                   "assignments": len(assignments), "resolved_assignments": sum(r["unresolved"] is None for r in assignments),
                   "actions": len(actions),
                   "resolved_action_features": sum(r["feature"] is not None and r["unresolved"] is None for r in actions),
                   "bare_actions": sum(r["feature"] is None for r in actions)},
        "unresolved_by_reason": dict(sorted(Counter(r["unresolved"] for r in assignments + actions if r["unresolved"]).items())),
        "unverified_semantics_by_dependency": dict(sorted(unverified.items())),
        "unassigned_calls": [{"rule": identity, "target": target}
                             for identity, targets in grammar.unassigned_calls.items() for target in targets],
        "assignments": assignments, "actions": actions,
    }


def render_rust(contract):
    lines = ["// Generated by tools/generate_xtext_assignment_contracts.py; do not edit.",
             "// Static construction contracts only; procedural semantics remain handwritten.",
             "use super::AssignmentContract;",
             "pub(super) const ASSIGNMENTS: &[AssignmentContract] = &["]
    for row in contract["assignments"]:
        field = row["ecore_feature"]
        if field is None:
            continue
        def lit(key):
            return json.dumps(key)
        lines.append("    AssignmentContract { rule: %s, feature: %s, operator: %s, ecore_owner: %s, ecore_target: %s, kind: %s, containment: %s },"
                     % (lit(row["rule"]), lit(row["feature"]), lit(row["operator"]), lit(field["owner"]),
                        lit(field["target"]), lit(field["kind"]), str(field["containment"]).lower()))
    lines.extend(["];", "pub(super) const UNASSIGNED_CALLS: &[(&str, &str)] = &["])
    for row in contract["unassigned_calls"]:
        lines.append(f'    ({json.dumps(row["rule"])}, {json.dumps(row["target"])}),')
    lines.extend(["];", ""])
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    contract = build(json.loads(GRAMMAR.read_text(encoding="utf-8")),
                     json.loads(ECORE.read_text(encoding="utf-8")))
    contract["source"] = {"grammar_sha256": hashlib.sha256(GRAMMAR.read_bytes()).hexdigest(),
                          "ecore_sha256": hashlib.sha256(ECORE.read_bytes()).hexdigest(),
                          "generator_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    for path, content in ((OUTPUT, json.dumps(contract, indent=2, ensure_ascii=False) + "\n"),
                          (RUST, render_rust(contract))):
        if args.check:
            if path.read_text(encoding="utf-8") != content:
                raise ValueError("Stale Xtext assignment contract: " + str(path))
        else:
            path.write_text(content, encoding="utf-8", newline="\n")
    print(contract["counts"], contract["unresolved_by_reason"])


if __name__ == "__main__":
    main()
