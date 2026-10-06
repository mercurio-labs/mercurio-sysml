"""Classify the complete resolved validator inventory without claiming translation.

The graph covers calls in the inventoried Xtend files. External Java method bodies,
dynamic dispatch and model delegates remain external boundaries, not resolved
transitively by this report. Only existing generated artifacts establish generation.
"""
from __future__ import annotations
import argparse
from collections import Counter, defaultdict
import json
from pathlib import Path
from export_validator_inventory import validate, PROFILE, ROOT, HELPER
from export_pilot_xtend import digest, encode
from translate_pilot_validators import AST_FIELDS, GETTERS, OPERATORS, DIAGNOSTIC_OWNER
from check_validation_bindings import build_report

INVENTORY = PROFILE / "validators.inventory.extract.json"
OUTPUT = ROOT / "docs/conformance/2026-08-support/validator-implementation-inventory.json"


def classify(document, boundary, overlay, legacy):
    summary = validate(document)
    methods = {m["id"]: m for m in document["methods"]}
    checks = [m for m in methods.values() if m["is_check"]]
    legacy_keys = {(c["source_file"], c["method"]) for c in legacy["checks"]}
    current_keys = {(c["source"], c["name"]) for c in checks}
    if len(legacy_keys) != len(legacy["checks"]) or len(current_keys) != len(checks) or legacy_keys != current_keys:
        raise ValueError("Frontend check inventory differs from the independent legacy inventory; reconcile explicitly")
    if document["provenance"]["pilot_revision"] != legacy["source"]["pilot"]["commit"]:
        raise ValueError("Inventory revisions differ")
    if document["provenance"]["sources_sha256"] != {f["path"]: f["sha256"] for f in legacy["source"]["source_files"]}:
        raise ValueError("Inventory source hashes differ")
    generated = {r["id"]: r for r in boundary["rules"]}
    if not set(generated) <= {m["rule_id"] for m in checks}:
        raise ValueError("Generated rule absent from frontend check inventory")
    bound_sources = boundary["provenance"]["pilot_source_provenance"]
    if any(bound_sources[key] != document["provenance"][key] for key in ("pilot_revision", "jar_sha256", "xtend_version", "xtend_jar_sha256")) or any(document["provenance"]["sources_sha256"].get(k) != v for k, v in bound_sources["sources_sha256"].items()):
        raise ValueError("Generated rules have different upstream provenance")
    jvm_methods = {}
    for method in methods.values():
        for symbol in method["jvm_ids"]:
            if symbol in jvm_methods:
                raise ValueError("Ambiguous source method JVM binding")
            jvm_methods[symbol] = method["id"]
    bound_rules = defaultdict(set)
    for rule in generated.values():
        for binding in rule["handwritten_semantic_bindings"]:
            bound_rules[binding["symbol"]].add(rule["id"])
    overlay_rows = {c["id"]: c for c in overlay["checks"]}
    if len(overlay_rows) != len(overlay["checks"]) or set(overlay_rows) != {c["id"] for c in legacy["checks"]}:
        raise ValueError("Prior handwritten ledger does not cover the same checks")

    def category(target):
        kind, identity = target["kind"], target.get("id")
        if kind != "JvmOperation":
            return "constructor" if kind == "JvmConstructor" else "field-or-enum-read" if kind in {"JvmField", "JvmEnumerationLiteral"} else "type-package-or-local"
        if identity in jvm_methods:
            return "inventoried-xtend-call"
        if identity in OPERATORS:
            return "supported-operator"
        if (target.get("declaring_type") == DIAGNOSTIC_OWNER and target.get("name") in {"error", "warning"}
                and target.get("parameter_types") == ["java.lang.String", "org.eclipse.emf.ecore.EObject", "org.eclipse.emf.ecore.EStructuralFeature", "java.lang.String", "java.lang.String[]"]
                and target.get("result_type") == "void" and target.get("varargs") and not target.get("static")):
            return "supported-diagnostic-signature"
        if target.get("declaring_type", "").startswith("org.omg.sysml.lang.sysml."):
            return "metamodel-getter" if not target.get("static") and not target.get("parameter_types") else "metamodel-operation"
        return "external-operation"

    edges = {}
    dependencies = {}
    direct = {}
    for method in methods.values():
        targets = [c["target"] for c in method["calls"] if c["resolved"]]
        edges[method["id"]] = {jvm_methods[t["id"]] for t in targets if t.get("id") in jvm_methods}
        direct[method["id"]] = set()
        for target in targets:
            kind = category(target)
            if kind == "type-package-or-local":
                continue
            identity = target.get("id")
            if not identity:
                raise ValueError("Nonlocal dependency lacks resolved identity")
            row = {**target, "category": kind,
                   "known_translator_getter": identity in GETTERS,
                   "native_binding_rule_scope": sorted(bound_rules[identity])}
            if identity in dependencies and dependencies[identity] != row:
                raise ValueError("Inconsistent resolved dependency metadata: " + identity)
            dependencies[identity] = row
            direct[method["id"]].add(identity)

    report = []
    usage = defaultdict(set)
    for method in sorted(checks, key=lambda m: m["id"]):
        reached, pending = set(), [method["id"]]
        while pending:
            current = pending.pop()
            if current not in reached:
                reached.add(current)
                pending.extend(edges[current] - reached)
        transitive = set().union(*(direct[member] for member in reached))
        nodes = set().union(*(set(methods[member]["body_node_kinds"]) for member in reached))
        unsupported_nodes = sorted(k for k in nodes if k.startswith("X") and k not in AST_FIELDS)
        unbound = sorted(symbol for symbol in transitive if dependencies[symbol]["category"] in {"metamodel-getter", "metamodel-operation"} and method["rule_id"] not in bound_rules[symbol])
        external = sorted(symbol for symbol in transitive if dependencies[symbol]["category"] in {"external-operation", "constructor"})
        for symbol in transitive:
            usage[symbol].add(method["rule_id"])
        flags = {
            "mutable_locals": sum(methods[member].get("mutable_locals", 0) for member in reached),
            "explicit_annotation_values": sum(methods[member].get("explicit_annotation_values", 0) for member in reached),
            "generic_parameters": sum(methods[member].get("generic_parameters", 0) for member in reached),
            "null_safe_calls": sum(bool(c.get("null_safe")) for member in reached for c in methods[member]["calls"]),
            "explicit_generic_calls": sum(bool(c.get("explicit_type_arguments")) for member in reached for c in methods[member]["calls"]),
        }
        blockers = {"unsupported_body_node_kinds": unsupported_nodes, "unbound_model_dependencies": unbound,
                    "external_operations": external, "source_helpers": sorted(reached - {method["id"]}),
                    "incomplete_resolution_methods": sorted(member for member in reached if methods[member]["resolution_status"] != "resolved"), **flags}
        is_generated = method["rule_id"] in generated
        if is_generated and blockers["incomplete_resolution_methods"]:
            raise ValueError("Generated rule has incomplete frontend resolution")
        prior_id = method["declaring_type"].rsplit(".", 1)[-1] + "::" + method["name"]
        prior = overlay_rows[prior_id]
        report.append({
            "id": method["id"], "rule_id": method["rule_id"], "source": method["source"], "span": method["span"],
            "resolution": method["resolution_status"], "resolution_issues": method["resolution_issues"],
            "implementation": "generated-rust" if is_generated else "not-translated",
            "full_semantic_qualification": "not-established",
            "upstream_empty_body": method["body_node_kinds"] == {"XBlockExpression": 1},
            "direct_dependencies": sorted(direct[method["id"]]), "transitive_dependencies": sorted(transitive),
            "translation_planning": blockers,
            "prior_handwritten_evidence": {k: v for k, v in prior.items() if k in {"status", "implementation", "scope", "evidence"}},
        })
    used_dependencies = [{**dependencies[symbol], "checks_using_dependency": sorted(usage[symbol])} for symbol in sorted(dependencies)]
    return {
        "schema_version": 1,
        "scope": "All declared @Check methods in the three SysML/KerML textual validator sources; external Java bodies and dynamic dispatch are not recursively inventoried",
        "planning_limits": "Node/call classification is a planning inventory, not a substitute for full AST translation. No unselected method is marked translatable or native-complete.",
        "summary": {**summary, "helpers": len(methods) - len(checks), "generated_rust": len(generated),
                    "not_translated": len(checks) - len(generated),
                    "prior_handwritten_status_counts": dict(sorted(Counter(c["status"] for c in overlay_rows.values()).items())),
                    "upstream_empty_checks": sum(r["upstream_empty_body"] for r in report),
                    "resolved_dependencies": len(dependencies), "dependency_categories": dict(sorted(Counter(d["category"] for d in dependencies.values()).items()))},
        "checks": report, "dependencies": used_dependencies,
        "helper_call_graph": [{"id": m["id"], "is_check": m["is_check"], "calls": sorted(edges[m["id"]])} for m in sorted(methods.values(), key=lambda m: m["id"])],
    }


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out", type=Path, default=OUTPUT)
    p.add_argument("--check", action="store_true")
    a = p.parse_args(argv)
    inventory = json.loads(INVENTORY.read_text(encoding="utf-8"))
    exporter_inputs = [HELPER, ROOT / "tools/export_validator_inventory.py", ROOT / "tools/export_pilot_xtend.py"]
    if inventory["provenance"]["tools_sha256"] != {path.name: digest(path) for path in exporter_inputs}:
        raise ValueError("Inventory exporter inputs changed; regenerate with the upstream frontend")
    boundary = build_report(ROOT)
    saved_boundary = ROOT / "docs/conformance/2026-08-support/validation-implementation-boundary.json"
    if boundary != json.loads(saved_boundary.read_text(encoding="utf-8")):
        raise ValueError("Regenerate stale selected-rule implementation boundary first")
    legacy_path = PROFILE / "validators.extract.json"
    overlay_path = PROFILE / "validator-coverage.overlay.json"
    legacy = json.loads(legacy_path.read_text(encoding="utf-8"))
    overlay = json.loads(overlay_path.read_text(encoding="utf-8"))
    if overlay["source"]["inventory_sha256"] != digest(legacy_path):
        raise ValueError("Historical handwritten ledger inventory hash differs")
    result = classify(inventory, boundary, overlay, legacy)
    inputs = [INVENTORY, saved_boundary, legacy_path, overlay_path, Path(__file__),
              ROOT / "tools/export_validator_inventory.py", ROOT / "tools/translate_pilot_validators.py", ROOT / "tools/check_validation_bindings.py", HELPER, ROOT / "tools/export_pilot_xtend.py"]
    result["provenance"] = {"inputs_sha256": {path.relative_to(ROOT).as_posix(): digest(path) for path in inputs}, "upstream": inventory["provenance"]}
    output = encode(result)
    if a.check:
        if not a.out.is_file() or a.out.read_bytes() != output:
            raise ValueError("Stale complete validator implementation inventory")
    else:
        a.out.parent.mkdir(parents=True, exist_ok=True)
        a.out.write_bytes(output)
    print(json.dumps(result["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
