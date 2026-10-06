
"""Resolve the complete frozen V01 lifecycle dependency bundle before execution.

This is a finite planning contract, never semantic qualification. Model getters
are connected to imported Ecore; Java/framework calls and model algorithms retain
resolved identities and explicit native implementation/verification boundaries.
"""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path
from value_result_bundle import getter_candidates, validator_method_closure

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs/conformance/2026-08-support"
EVIDENCE = BASE / "definition-pipeline-evidence"
META = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
OUTPUT = EVIDENCE / "value-result-lifecycle-dependency-plan.json"

def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def require(value, message):
    if not value:
        raise ValueError(message)

def build_plan(bundle, inventory, effective, validators, native_rows, parse_negative):
    contexts = bundle["contexts"]
    require(len(contexts) == len(native_rows) == 26
        and [c["id"] for c in contexts] == [r["relative_path"] for r in native_rows],
        "All26 frozen source contexts in original order are required")
    require(parse_negative["relative_path"] == "valuation-missing-value"
        and parse_negative["status"] == "blocked" and parse_negative["stage"] == "syntax"
        and parse_negative["semantic_qualification"] == "not_assessed",
        "The parse-negative context requires its typed native syntax failure")
    require(parse_negative["input_files"] == next(r["input_files"] for r in native_rows
        if r["relative_path"] == "valuation-missing-value"), "Parse-negative native source differs")
    classes = {c["name"]: c for c in effective["classes"]}
    feature_by_id = {f["id"]: f for f in effective["features"]}
    methods = {m["id"]: m for m in validators["methods"]}
    require(len(methods) == len(validators["methods"]), "Duplicate resolved method identity")
    def ancestors(kind):
        todo, result = [kind], set()
        while todo:
            current = todo.pop()
            require(current in classes, "Unknown pinned native metaclass: " + current)
            if current in result:
                continue
            result.add(current)
            todo.extend(p.split("#//")[-1].split("/")[0] for p in classes[current]["super_types"])
        return result
    checks = inventory["applicable_check_candidates"]
    require(len(checks) == 35 and len({r["id"] for r in checks}) == 35,
        "All35 candidate checks must remain in scope")
    reached, external, callers = set(), set(), {}
    for check in checks:
        closure = validator_method_closure(check["id"], validators)
        require(not closure["resolution_issues"], "Unresolved imported check method closure")
        reached.update(closure["methods"])
        external.update(closure["external_symbols"])
        for symbol in closure["external_symbols"]:
            callers.setdefault(symbol, []).append(check["id"])
    require(len(reached) == 57 and len(external) == 183, "Frozen resolved dependency denominator changed")
    targets = {}
    for method_id in sorted(reached):
        for call in methods[method_id]["calls"]:
            target = call.get("target") or {}
            if target.get("id") in external:
                require(target["kind"] == "JvmOperation", "External dependency is not a resolved JVM operation")
                identity = target["id"]
                if identity in targets:
                    require(targets[identity]["name"] == target["name"]
                        and targets[identity]["declaring_type"] == target["declaring_type"],
                        "Resolved JVM identity became ambiguous")
                else:
                    targets[identity] = target
    require(set(targets) == external, "Every transitive JVM operation requires its resolved target")
    scopes = []
    for context, native in zip(contexts, native_rows):
        require(native["publication"] == "not_attempted", "Source scope inspection became publication")
        if native["status"] == "blocked":
            require(context["id"] == "valuation-missing-value" and context["required_rejection_stage"] == "parse",
                "An unrelated implementation failure cannot supply source scope")
            kinds = set()
            boundary = "Native source did not construct. Semantic check applicability was not reached; no complete terminal credit is awarded by this plan."
        else:
            require(native["status"] == "unlinked_inspection" and native["semantic_qualification"] == "not_assessed"
                and native["inspection"]["linking"] == "not_run", "Source scope inspection became semantic execution")
            graph = native["inspection"]["constructed_elements"]
            require(len(graph) == native["element_count"]
                and len({e["id"] for e in graph}) == len(graph), "Source graph identities/count changed")
            require(all(not e["properties"].get("is_implied_included", False) for e in graph),
                "Source inspection contains prepared completion flags")
            kinds = {e["kind"].split("::")[-1] for e in graph}
            boundary = "Constructed base-source kinds establish candidacy only. Transformation-created kinds, mutations, validator registration/dispatch and semantic branch applicability remain required."
        known = set().union(*(ancestors(k) for k in kinds)) if kinds else set()
        source_checks = sorted(c["id"] for c in checks if c["subject"] in known)
        scopes.append({"id": context["id"], "native_construction_status": native["status"],
            "constructed_base_source_kinds": sorted(kinds), "candidate_checks": source_checks,
            "specified_mutation": context["mutation"], "mutation_applied": False,
            "full_semantic_applicability": "not_verified", "native_context_complete": False,
            "boundary": boundary})
    source_contexts = {c["id"]: set(c["candidate_checks"]) for c in scopes}
    dependencies = []
    for symbol in sorted(external):
        target = targets[symbol]
        declaring = target["declaring_type"]
        candidates = getter_candidates(target["name"], declaring, effective["features"], classes)
        entry = {"symbol": symbol, "resolved_target": target,
            "required_check_roots": sorted(callers[symbol]),
            "base_source_context_candidates": sorted(c for c, roots in source_contexts.items() if roots.intersection(callers[symbol])),
            "native_semantically_verified": False}
        if candidates:
            entry.update(category="resolved_ecore_getter",
                imported_features=[feature_by_id[c] for c in candidates],
                native_consumer_boundary="Shared typed reference or attribute consumers are candidates. Concrete binding dispatch, absent-value/delegate semantics and context evidence are required; imported getter mapping is not implementation.")
        elif declaring == "org.omg.sysml.lang.sysml.SysMLPackage":
            entry.update(category="ecore_descriptor_dependency",
                native_consumer_boundary="Resolve exact EClass/EStructuralFeature diagnostic tokens from imported definitions; recording a JVM accessor is not token implementation.")
        elif declaring == "org.omg.sysml.lang.sysml.FeatureDirectionKind":
            entry.update(category="enum_conversion_dependency",
                native_consumer_boundary="Imported literal identity and explicit native string conversion contract required.")
        elif declaring.startswith("org.omg.sysml.lang.sysml."):
            owner = declaring.rsplit(".",1)[-1]
            require(owner in classes, "Unknown operation owner metaclass")
            lineage = ancestors(owner)
            operations = [o for o in effective["operations"]
                if o["owner"].split("#//")[-1] in lineage and o["name"] == target["name"]]
            entry.update(category="resolved_model_operation", imported_operation_candidates=operations,
                native_consumer_boundary="Resolved signature/dispatch must reach a named native algorithm. Type.directionOf has bounded independent evidence; other operation/context obligations remain unverified here.")
        elif declaring.startswith("org.omg.sysml.util."):
            entry.update(category="model_utility_algorithm",
                strategy_group=declaring.rsplit(".",1)[-1],
                native_consumer_boundary="Review resolved utility body and every transitive delegate/algorithm against existing native consumers. Handwritten Rust remains explicit; a method signature cannot close this dependency.")
        elif declaring.startswith(("java.", "org.eclipse.")):
            entry.update(category="ordinary_or_framework_call", strategy_group=declaring,
                native_consumer_boundary="Preserve actual ordered collection, short-circuit, arithmetic, diagnostic and framework side-effect semantics. Do not silently discard a framework call.")
        else:
            raise ValueError("Unclassified resolved dependency: " + symbol)
        dependencies.append(entry)
    counts = dict(sorted(Counter(d["category"] for d in dependencies).items()))
    require(counts == {"ecore_descriptor_dependency":22, "enum_conversion_dependency":1,
        "model_utility_algorithm":31, "ordinary_or_framework_call":58,
        "resolved_ecore_getter":63, "resolved_model_operation":8}, "Reviewed category inventory changed")
    return {"schema":"dev.mercurio.value-result-lifecycle-dependency-plan.v1",
        "qualification_certificate":False, "native_complete_contexts":0, "strict_families_qualified":0,
        "counts":{"fixed_contexts":26,"native_base_sources_constructed":25,
            "syntax_negative_source_observations":1,"candidate_check_roots":35,
            "imported_methods_reached":57,"external_symbols":183,"dependencies_by_category":counts},
        "source_contexts":scopes, "dependencies":dependencies,
        "required_check_roots":[{"id":c["id"],"subject":c["subject"],
            "constructed_base_source_context_candidates":sorted(s["id"] for s in scopes if c["id"] in s["candidate_checks"]),
            "independent_source_context_candidates":c["source_context_candidates"],
            "independent_provider_context_candidates":c["provider_context_candidates"],
            "validator_registration_and_semantic_branch_applicability":"not_verified"} for c in checks],
        "execution_boundary":"This complete dependency plan authorizes no completion flag, model publication, mutation, family closure or terminal-context certificate. Imported, native-consumed and semantically verified remain separate."}

def audit(check=False,native_prefix="value-result-lifecycle"):
    paths = {"bundle":BASE/"value-result-obligation-bundle.json",
        "inventory":BASE/"value-result-dependency-inventory.json",
        "effective":META/"ecore-effective.extract.json",
        "validators":META/"validators.inventory.extract.json",
        "native_scope":EVIDENCE/(native_prefix+"-source-inspection.jsonl")}
    run = read(EVIDENCE/(native_prefix+"-source-run.json"))
    require(run["inputs_unchanged"] and run["qualification_certificate"] is False
        and run["output_sha256"] == digest(paths["native_scope"]), "Native source scope execution is stale")
    require(digest(ROOT/"target/release/audit_release_compile.exe") == run["binary_sha256"], "Source scope executable changed")
    for path, expected in {**run["consumer_sha256"], **run["source_sha256"]}.items():
        require(digest(ROOT/path) == expected, "Native source scope input changed: " + path)
    native = [json.loads(line) for line in paths["native_scope"].read_text(encoding="utf-8").splitlines()]
    parse_run_path = EVIDENCE / (native_prefix+"-parse-negative-run.json")
    parse_output = EVIDENCE / (native_prefix+"-parse-negative.jsonl")
    parse_run = read(parse_run_path)
    require(parse_run["qualification_certificate"] is False and parse_run["exit_code"] != 0
        and parse_run["inputs_unchanged"] and parse_run["binary_sha256"] == run["binary_sha256"]
        and parse_run["consumer_sha256"] == run["consumer_sha256"]
        and parse_run["output_sha256"] == digest(parse_output), "Typed native parse-negative execution is stale")
    for path, expected in parse_run["source_sha256"].items():
        require(digest(path) == expected, "Typed parse-negative source changed")
    paths["parse_run"] = parse_run_path
    paths["parse_output"] = parse_output
    result = build_plan(read(paths["bundle"]),read(paths["inventory"]),read(paths["effective"]),read(paths["validators"]),native,read(parse_output))
    result["input_sha256"] = {str(p.relative_to(ROOT)).replace("\\","/"):digest(p) for p in paths.values()}
    for path in [Path(__file__),EVIDENCE/(native_prefix+"-source-run.json"),EVIDENCE/"value-result-lifecycle-batch-scope.json"]:
        result["input_sha256"][str(path.relative_to(ROOT)).replace("\\","/")] = digest(path)
    output=OUTPUT if native_prefix=="value-result-lifecycle" else EVIDENCE/(native_prefix+"-dependency-plan.json")
    encoded = json.dumps(result,indent=2)+"\n"
    if check:
        require(output.read_text(encoding="utf-8") == encoded, "Lifecycle dependency plan is stale")
    else:
        output.write_text(encoded,encoding="utf-8")
    print(json.dumps(result["counts"],indent=2))
    return result

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check",action="store_true")
    parser.add_argument("--native-prefix",choices=["value-result-lifecycle","value-result-lifecycle-shared-services","value-result-fresh-structure"],default="value-result-lifecycle")
    args=parser.parse_args()
    audit(args.check,args.native_prefix)
