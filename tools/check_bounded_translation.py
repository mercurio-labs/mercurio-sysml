"""Check the frozen translation trial and report counts without claiming conformance.

This joins resolved complete ASTs to actual generated Rust evidence. Resolution,
generation and qualification are distinct; percentage is a check-count metric.
"""
import argparse
from collections import Counter
import json
from pathlib import Path

from export_pilot_xtend import ROOT, SOURCE, SYSML_SOURCE, digest, encode, validate_structure
from check_validation_bindings import build_report

PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
SCOPE = PROFILE / "bounded-translation-scope.json"
AST = PROFILE / "validators.bounded.ast.extract.json"
OUTPUT = ROOT / "docs/conformance/2026-08-support/bounded-translation-progress.json"
BASE = {"org.omg.kerml.xtext.validation.KerMLValidator#checkAnnotation",
        "org.omg.kerml.xtext.validation.KerMLValidator#checkImport",
        "org.omg.sysml.xtext.validation.SysMLValidator#checkReferenceUsage",
        "org.omg.sysml.xtext.validation.SysMLValidator#checkEnumerationDefinition"}
HELPER = "org.omg.sysml.xtext.validation.SysMLValidator#checkOneType"


def nodes(value):
    if isinstance(value, dict):
        if "kind" in value and "fields" in value:
            yield value
        for child in value.values():
            yield from nodes(child)
    elif isinstance(value, list):
        for child in value:
            yield from nodes(child)


def report(scope, ast, boundary):
    if scope.get("schema_version") != 1 or scope.get("scope_id") != "bounded-xtend-one-type-v1":
        raise ValueError("Unsupported bounded scope")
    checks, helpers = scope["checks"], scope["helpers"]
    if len(checks) != 11 or len(set(checks)) != 11 or not BASE <= set(checks):
        raise ValueError("Scope must contain four baseline and seven family checks")
    if helpers != [HELPER] or set(checks) & set(helpers):
        raise ValueError("Expected exactly the one bounded helper")
    sources = {"org.omg.kerml.xtext.validation.KerMLValidator": SOURCE,
               "org.omg.sysml.xtext.validation.SysMLValidator": SYSML_SOURCE}
    selection = {}
    for identity in checks + helpers:
        owner, name = identity.split("#")
        selection.setdefault(sources[owner], set()).add(name)
    validate_structure(ast, selection)
    methods = {method["id"]: method for method in ast["methods"]}
    if set(methods) != set(checks + helpers):
        raise ValueError("Resolved AST differs from exact bounded scope")
    # A complete selected family caller is one helper call, not a slice cut out
    # of a larger validator with unhandled guards or additional checks.
    helper_symbol = HELPER.replace("#", ".") + "(org.omg.sysml.lang.sysml.Feature,java.lang.Class,java.lang.String,org.eclipse.emf.ecore.EReference,java.lang.String)"
    for identity in set(checks) - BASE:
        body = methods[identity]["fields"]["expression"]
        expressions = body["fields"].get("expressions", [])
        if body["kind"] != "XBlockExpression" or len(expressions) != 1:
            raise ValueError("Family caller must retain one complete helper-call body: " + identity)
        call = expressions[0]
        if call["kind"] != "XFeatureCall" or call["fields"].get("feature") != {"$ref": helper_symbol}:
            raise ValueError("Family caller targets an unselected helper: " + identity)
        if len(call["fields"].get("featureCallArguments", [])) != 5:
            raise ValueError("Family call arguments changed: " + identity)
    generated = {rule["id"] for rule in boundary["rules"]}
    if not generated <= set(checks):
        raise ValueError("Generated selection escaped frozen scope")
    inlined = {helper["id"] for helper in boundary.get("inlined_helpers", [])}
    if not inlined <= set(helpers):
        raise ValueError("Generated helper escaped frozen scope")
    rows = []
    for identity in checks + helpers:
        method = methods[identity]
        counts = Counter(node["kind"] for node in nodes(method["fields"]["expression"]))
        symbols = sorted({node["fields"]["feature"]["$ref"] for node in nodes(method["fields"]["expression"])
                          if isinstance(node["fields"].get("feature"), dict)
                          and node["fields"]["feature"].get("$ref") in ast["symbols"]})
        rows.append({"id": identity, "role": "check" if identity in checks else "helper",
                     "resolved_ast": True, "generated_rust": identity in generated | inlined,
                     "qualification": "see selected binding report; full semantic qualification not established" if identity in generated | inlined else "pending",
                     "source": method["source"], "span": method["span"],
                     "body_node_kinds": dict(sorted(counts.items())), "resolved_symbols": symbols})
    complete = len(generated & set(checks))
    return {"schema_version": 1, "scope_id": scope["scope_id"],
            "check_count": len(checks), "generated_check_count": complete,
            "generated_check_percent": round(100 * complete / len(checks), 1),
            "percent_meaning": "Generated selected checks / selected checks; not elapsed effort, overall completion, or specification compliance",
            "resolved_helper_count": len(helpers), "generated_helper_count": len(inlined),
            "trial_complete": False,
            "completion_gates": [{"requirement": gate, "qualification": "not established by this static report"} for gate in scope["completion_gates"]],
            "methods": rows, "native_services": scope["native_services"], "excluded": scope["excluded"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    scope = json.loads(SCOPE.read_text(encoding="utf-8"))
    ast = json.loads(AST.read_text(encoding="utf-8"))
    if ast["provenance"].get("scope_sha256") != digest(SCOPE):
        raise ValueError("Stale scope AST provenance")
    from export_pilot_xtend import HELPER as EXPORTER
    import export_pilot_xtend
    if ast["provenance"]["exporter_sha256"] != digest(EXPORTER) or ast["provenance"]["driver_sha256"] != digest(Path(export_pilot_xtend.__file__)):
        raise ValueError("Stale bounded AST exporter provenance")
    production = json.loads((PROFILE / "validation-rules.extract.json").read_text(encoding="utf-8"))["source"]
    for field in ("pilot_revision", "jar_sha256", "xtend_jar_sha256", "sources_sha256"):
        if ast["provenance"].get(field) != production.get(field):
            raise ValueError("Bounded/production source provenance differs: " + field)
    boundary = build_report(ROOT)
    result = report(scope, ast, boundary)
    result["provenance"] = {"scope_sha256": digest(SCOPE), "ast_sha256": digest(AST),
                            "checker_sha256": digest(Path(__file__)),
                            "boundary": boundary["provenance"]}
    content = encode(result)
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_bytes() != content:
            raise ValueError("Stale bounded translation progress")
    else:
        OUTPUT.write_bytes(content)
    print(f"{result['generated_check_count']}/{result['check_count']} selected checks generated ({result['generated_check_percent']}% by count); helper generation {result['generated_helper_count']}/{result['resolved_helper_count']}; execution qualification is separate")


if __name__ == "__main__":
    main()
