"""Map concrete ordinary bindings to grammar sites and existing Pilot observations.

Return types and actions are construction candidates, never proof of reachable
syntax or native support. Controlled factory cases remain separate evidence.
"""
import argparse
import hashlib
import json
from pathlib import Path
from export_pilot_feature_redefinitions import bounded

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
OUTPUT = ROOT / "docs/conformance/2026-08-support/ordinary-strategy-integration.json"

def nodes(value):
    if isinstance(value, dict):
        yield value
        for child in value.values(): yield from nodes(child)
    elif isinstance(value, list):
        for child in value: yield from nodes(child)

def classifier(value):
    if not isinstance(value, dict): return None
    ref = value.get("fields", {}).get("classifier", {}).get("$ref", "")
    return ref.rsplit("#//", 1)[-1] if "#//" in ref else None

def inventory(bindings, grammar, observations, defaults=None):
    kinds = sorted(b["kind"] for b in bindings["bindings"] if bounded(b))
    if len(kinds) != 30 or len(set(kinds)) != 30:
        raise ValueError("Review changed ordinary binding inventory")
    rows = []
    for kind in kinds:
        sites = []
        for g in grammar["grammars"]:
            for rule in g["rules"]:
                if rule["kind"] != "ParserRule": continue
                if classifier(rule["fields"].get("type")) == kind:
                    sites.append({"rule": rule["id"], "site": rule["id"], "category": "return_type"})
                for node in nodes(rule["fields"].get("alternatives")):
                    if node.get("kind") == "Action" and classifier(node["fields"].get("type")) == kind:
                        sites.append({"rule": rule["id"], "site": node["id"], "category": "action"})
        controls = []
        for case in observations["controls"] + observations.get("ordinary_strategy_source_controls", []) + (defaults or {}).get("usage_source_controls", []):
            if case.get("accepted") is not True: continue
            paths = sorted(n["path"] for n in case["nodes"] if n["kind"] == kind)
            if paths:
                identity = json.dumps([case["language"], case["source"]], separators=(",", ":"))
                controls.append({"source_sha256": hashlib.sha256(identity.encode()).hexdigest(), "language": case["language"], "paths": paths})
        rows.append({"kind": kind, "grammar_candidates": sites, "accepted_document_observations": controls,
                     "integration_status": "existing_document_evidence_requires_strategy_context_review" if controls else "missing_document_evidence",
                     "factory_lifecycle_cases": 6,
                     "remaining": "Review ordinary strategy contexts and concrete grammar/link coverage; no construction site is not proof of grammar unreachability."})
    return {"schema": "dev.mercurio.ordinary-strategy-integration.v1", "bindings": rows,
            "bindings_with_document_observations": sum(bool(r["accepted_document_observations"]) for r in rows),
            "bindings_without_grammar_candidates": [r["kind"] for r in rows if not r["grammar_candidates"]],
            "qualified_bindings": 0}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    paths = [PROFILE / n for n in ("feature-redefinitions.extract.json", "grammar.structure.extract.json", "definition-document-pilot-controls.json", "feature-defaults.extract.json")]
    result = inventory(*(json.loads(p.read_text(encoding="utf-8")) for p in paths))
    result["inputs"] = {p.relative_to(ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths + [Path(__file__)]}
    data = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != data: raise ValueError("Stale strategy integration inventory")
    else: OUTPUT.write_text(data, encoding="utf-8", newline="\n")
    print(f"{result['bindings_with_document_observations']}/30 bindings have existing document observations; no new qualification inferred")

if __name__ == "__main__": main()
