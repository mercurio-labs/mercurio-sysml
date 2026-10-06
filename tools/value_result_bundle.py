"""Audit the frozen V01 dependency bundle from resolved pinned definitions.

This inventory does not qualify native support. The source matrix, reference
witnesses, candidate delegates and every resolved validator call stay separate.
"""
import argparse, gzip, hashlib, json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs/conformance/2026-08-support"
META = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
OUTPUT = BASE / "value-result-dependency-inventory.json"

def load(path):
    data = Path(path).read_bytes()
    return json.loads(gzip.decompress(data) if str(path).endswith(".gz") else data)

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def validate_reference(bundle, reference):
    if bundle.get("qualification_certificate") is not False or reference.get("native_qualification") != "not_assessed":
        raise ValueError("Reference observations cannot award native qualification")
    rows = reference["controls"]
    contexts = {c["id"]: c for c in bundle["contexts"]}
    if any(set(c["required_stages"]) != {"parse", "construct", "link", "transform", "validate", "publish", "persist", "compare"} for c in contexts.values()):
        raise ValueError("Every frozen verification stage is required")
    if len(contexts) != 26 or len(rows) != 26 or {r["id"] for r in rows} != set(contexts):
        raise ValueError("Every frozen V01 context is required")
    query_count = witness_count = library_count = 0
    for row in rows:
        context = contexts[row["id"]]
        if row["validation_status"] != context["independent_outcome"]:
            raise ValueError("Independent outcome changed: " + row["id"])
        if row["validation_status"] != "accepted":
            if row.get("result_observations") or row.get("model"):
                raise ValueError("Rejected context cannot publish observations")
            continue
        observation = row["result_observations"]
        witnesses = {w["id"]: w for w in observation["stored_witnesses"]}
        if len(witnesses) != len(observation["stored_witnesses"]):
            raise ValueError("Duplicate canonical witness")
        query_count += len(observation["queries"])
        witness_count += len(witnesses)
        library_count += sum(w["resource"].startswith("sysml.library/") for w in witnesses.values())
        for query in observation["queries"]:
            target = query["result"]
            if target is None or target["id"] not in witnesses:
                raise ValueError("Resolved result requires a canonical stored witness")
            result = witnesses[target["id"]]
            parents = result["reference_sequences"]["owningRelationship"]
            if len(parents) != 1 or parents[0]["id"] not in witnesses:
                raise ValueError("Result membership witness absent")
            membership = witnesses[parents[0]["id"]]
            if membership["kind"] != "ReturnParameterMembership":
                raise ValueError("Result is not a return parameter")
            children = membership["reference_sequences"]["ownedRelatedElement"]
            if [x["id"] for x in children] != [target["id"]]:
                raise ValueError("Result reciprocal ownership mismatch")
            owners = membership["reference_sequences"]["owningRelatedElement"]
            if len(owners) != 1 or owners[0]["id"] not in witnesses:
                raise ValueError("Return owner witness absent")
            owner = witnesses[owners[0]["id"]]
            if membership["id"] not in [x["id"] for x in owner["reference_sequences"]["ownedRelationship"]]:
                raise ValueError("Return owner reciprocal ownership mismatch")
        if context["role"] == "original_parsed_control":
            functions = [q for q in observation["queries"] if q["kind"] == "Function" and (q.get("qualified_name") or "").endswith("::f")]
            if len(functions) != 1:
                raise ValueError("Original source requires its actual function")
            function = functions[0]
            owned = context["original_result_branch"] == "owned"
            if bool(function["owned_return_memberships"]) != owned or bool(function["owned_result"]) != owned:
                raise ValueError("Owned/inherited original result branch changed")
            if owned and function["owned_result"]["id"] != function["result"]["id"]:
                raise ValueError("Owned result identity mismatch")
            if not owned and function["result"]["resource"].startswith("valuation-"):
                raise ValueError("Inherited result cannot be fabricated locally")
    return {"contexts_observed": len(rows), "direct_result_queries": query_count,
            "canonical_stored_witnesses": witness_count, "library_stored_witnesses": library_count,
            "native_contexts_verified": 0}

def getter_candidates(getter, declaring, features, classes):
    # Resolved model declarations plus pinned Ecore ancestry, not a source-body
    # recognizer. Boolean isX features expose isX in this pinned EMF model.
    if not declaring.startswith("org.omg.sysml.lang.sysml."):
        return []
    short = declaring.rsplit(".", 1)[-1]
    if short not in classes:
        return []
    ancestry = set()
    def add(kind):
        if kind in ancestry: return
        ancestry.add(kind)
        for parent in classes[kind]["super_types"]: add(parent.split("#//")[-1].split("/")[0])
    add(short)
    candidates = []
    for feature in features:
        if feature["owner"].split("#//")[-1].split("/")[0] not in ancestry:
            continue
        name = feature["name"]
        getters = {"get" + name[0].upper() + name[1:]}
        if feature["type"].endswith("#//EBoolean"):
            getters.add("is" + name[0].upper() + name[1:])
            if name.startswith("is") and len(name) > 2 and name[2].isupper():
                getters.add(name)
        if getter in getters:
            candidates.append(feature["id"])
    return candidates

def validator_method_closure(start_id, validators):
    """Follow resolved JVM identities through the already imported Xtend methods.

    Java/util/delegate bodies outside this inventory are explicit external
    boundaries. This is dependency analysis, never execution or qualification.
    """
    methods = {m["id"]: m for m in validators["methods"]}
    if len(methods) != len(validators["methods"]) or start_id not in methods:
        raise ValueError("Missing or duplicate validator method identity")
    symbols = {}
    for method in methods.values():
        for symbol in method.get("jvm_ids", []):
            if symbol in symbols and symbols[symbol] != method["id"]:
                raise ValueError("Ambiguous resolved validator JVM identity: " + symbol)
            symbols[symbol] = method["id"]
    todo, reached, edges, external, issues = [start_id], set(), set(), set(), []
    while todo:
        identity = todo.pop()
        if identity in reached:
            continue
        reached.add(identity)
        method = methods[identity]
        if method.get("resolution_status") != "resolved" or method.get("resolution_issues"):
            issues.append({"method": identity, "resolution_status": method.get("resolution_status"),
                           "issues": method.get("resolution_issues", [])})
        for call in method.get("calls", []):
            target = call.get("target") or {}
            if target.get("kind") != "JvmOperation":
                if call.get("resolved") is False:
                    issues.append({"method": identity, "unresolved_call_span": call.get("span")})
                continue
            symbol = target.get("id")
            if not symbol:
                issues.append({"method": identity, "missing_operation_identity": call.get("span")})
                continue
            edges.add((identity, symbol, symbols.get(symbol)))
            if symbol in symbols:
                todo.append(symbols[symbol])
            else:
                external.add(symbol)
    return {
        "status": "resolved_imported_methods_external_bodies_open" if not issues else "resolution_issues_open",
        "methods": sorted(reached),
        "edges": [{"caller_method": caller, "resolved_jvm_symbol": symbol, "imported_callee": callee}
                  for caller, symbol, callee in sorted(edges)],
        "external_symbols": sorted(external),
        "resolution_issues": sorted(issues, key=lambda row: json.dumps(row, sort_keys=True)),
        "native_verified": False,
        "boundary": "External JVM symbols require their actual algorithm/delegate/native consumer assessment. A finite imported method closure is not a complete semantic dependency closure.",
    }

def build(bundle, reference, effective, semantics, validators, bindings, rules):
    metrics = validate_reference(bundle, reference)
    classes = {c["name"]: c for c in effective["classes"]}
    name = lambda value: value.split("#//")[-1].split("/")[0]
    concrete = {e["kind"] for r in reference["controls"] for e in (r.get("model") or {}).get("elements", [])}
    concrete.update(w["kind"] for r in reference["controls"] for w in r.get("result_observations", {}).get("stored_witnesses", []))
    if concrete - set(classes):
        raise ValueError("Model kind is absent from pinned Ecore")
    closure = set()
    def ancestors(kind, active):
        if kind in active:
            raise ValueError("Cyclic pinned inheritance")
        if kind in closure:
            return
        closure.add(kind)
        for parent in classes[kind]["super_types"]:
            ancestors(name(parent), active | {kind})
    for kind in concrete:
        ancestors(kind, set())
    features = [dict(f, native_bundle_status="not_verified") for f in effective["features"] if name(f["owner"]) in closure]
    operations = [dict(o, native_bundle_status="not_verified") for o in effective["operations"] if name(o["owner"]) in closure]
    scoped_features = {f["id"]: f for f in features}
    scope_ids = set(scoped_features)
    scope_ids.update(b["element"] for b in semantics["delegate_bindings"] if name(b["element"]) in closure)
    delegates = [dict(b, native_bundle_status="not_verified") for b in semantics["delegate_bindings"] if b["element"] in scope_ids]
    # Keep validation scope separate from the export's library reference
    # closure. Pilot validates input resources; foreign provider behavior remains
    # required, but cannot silently expand or erase source check applicability.
    def kind_ancestors(kind):
        result, pending = set(), [kind]
        while pending:
            current = pending.pop()
            if current in result: continue
            if current not in classes: raise ValueError("Unknown context metaclass")
            result.add(current)
            pending.extend(name(p) for p in classes[current]["super_types"])
        return result
    source_subjects = {c["id"]: set().union(*(kind_ancestors(k) for k in c["observed_source_kinds"]))
                       for c in bundle["contexts"]}
    provider_subjects = {}
    for row in reference["controls"]:
        kinds = {e["kind"] for e in (row.get("model") or {}).get("elements", [])
                 if e.get("source", {}).get("file", "").startswith("sysml.library/")}
        kinds.update(w["kind"] for w in row.get("result_observations", {}).get("stored_witnesses", [])
                     if w["resource"].startswith("sysml.library/"))
        provider_subjects[row["id"]] = set().union(*(kind_ancestors(k) for k in kinds))
    bound = {b["symbol"]: b for b in bindings["bindings"]}
    translated = {r["id"]: r for r in rules["rules"]}
    checks, dependencies = [], {}
    for method in validators["methods"]:
        if not method["is_check"] or len(method["parameters"]) != 1:
            continue
        subject = method["parameters"][0]["type"].rsplit(".", 1)[-1]
        if subject not in closure:
            continue
        calls = []
        for call in method["calls"]:
            target = call.get("target") or {}
            if target.get("kind") != "JvmOperation":
                continue
            symbol = target.get("id")
            if not symbol:
                symbol = "<unresolved>:" + method["id"] + ":" + str(call.get("span"))
            declaring = target.get("declaring_type", "")
            getter = target.get("name", "")
            candidates = getter_candidates(getter, declaring, features, classes)
            if candidates:
                category = "ecore_setting_delegate" if any(scoped_features[c]["derived"] or scoped_features[c]["volatile"] for c in candidates) else "ecore_stored_access"
            elif declaring.startswith("org.omg.sysml.util."):
                category = "model_algorithm"
            elif declaring == "org.omg.sysml.lang.sysml.SysMLPackage":
                category = "ecore_contract_descriptor"
            elif declaring == "org.omg.sysml.lang.sysml.FeatureDirectionKind":
                category = "enum_scalar_dependency"
            elif declaring.startswith("org.omg.sysml.lang.sysml."):
                category = "model_operation_or_dispatch"
            elif declaring.startswith(("java.", "org.eclipse.")):
                category = "java_or_framework_dependency"
            else:
                category = "validator_helper_or_other_dependency"
            dependencies[symbol] = {"symbol": symbol, "resolved_target": target,
                "algorithm_category": category, "ecore_feature_candidates": candidates,
                "existing_native_binding": bound.get(symbol), "native_bundle_status": "not_verified"}
            calls.append(symbol)
        checks.append({"id": method["id"], "rule_id": method["rule_id"], "subject": subject,
            "source": method["source"], "span": method["span"], "resolution_status": method["resolution_status"],
            "resolution_issues": method["resolution_issues"], "dependencies": list(dict.fromkeys(calls)),
            "bounded_translation_present": method["rule_id"] in translated, "native_bundle_status": "not_verified",
            "source_context_candidates": sorted(identity for identity, kinds in source_subjects.items() if subject in kinds),
            "provider_context_candidates": sorted(identity for identity, kinds in provider_subjects.items() if subject in kinds),
            "applicability_boundary": "Metaclass candidacy only; rule branches, rejected-stage scope and required provider checks still need semantic review.",
            "imported_validator_method_closure": validator_method_closure(method["id"], validators)})
    return {"schema": "dev.mercurio.value-result-dependency-inventory.v1", "qualification_certificate": False,
        "meaning": "Conservative resolved dependency inventory for the frozen V01 bundle. Class applicability, an imported operation, a candidate delegate, or a native binding record cannot qualify behavior.",
        "obligation": "V01", "metrics": metrics, "concrete_model_kinds": sorted(concrete),
        "inherited_contract_kinds": sorted(closure), "ecore_features": features,
        "ecore_operations": operations, "delegate_bindings": delegates, "applicable_check_candidates": checks,
        "validator_applicability_boundary": {
            "policy": "Source and provider metaclass candidacy are separate. Empty observed kinds are unknown scope, not a proof that no check applies.",
            "unknown_source_contexts": [{"id": c["id"], "required_rejection_stage": c["required_rejection_stage"],
                                         "status": "source_check_applicability_not_assessed"}
                                        for c in bundle["contexts"] if not c["observed_source_kinds"]],
            "semantic_branch_applicability": "not_verified",
            "full_semantic_dependency_closure": "external_model_java_delegate_algorithms_open",
        },
        "validator_closure_metrics": {
            "check_roots": len(checks),
            "imported_methods_reached": len({identity for row in checks for identity in row["imported_validator_method_closure"]["methods"]}),
            "external_symbols": len({symbol for row in checks for symbol in row["imported_validator_method_closure"]["external_symbols"]}),
            "checks_with_resolution_issues": sum(bool(row["imported_validator_method_closure"]["resolution_issues"]) for row in checks),
            "native_checks_verified": 0,
        },
        "resolved_semantic_dependencies": sorted(dependencies.values(), key=lambda x: x["symbol"]),
        "context_stage_matrix": [{"id": c["id"], "required_rejection_stage": c["required_rejection_stage"],
            "required_stages": c["required_stages"], "native_complete": False} for c in bundle["contexts"]],
        "unresolved_closure": ["Confirm applicability and implement every required semantic/check dependency in the actual native source/provider closure.",
            "Resolve explicit/implicit/global native resource closure; the 15-resource reference envelope is not proof.",
            "Consume explicit read/construction transformation plans through the existing typed scheduler; bounded literal/binding read-wave implementation is tracked separately and does not close the provider lifecycle.",
            "Execute all 26 complete native context outcomes, with normative validation, publication and persistence.",
            "Review and independently verify every required specification/Pilot disagreement."]}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = [BASE/"value-result-obligation-bundle.json", BASE/"value-result-reference-controls.json.gz",
        META/"ecore-effective.extract.json", META/"ecore-semantics.extract.json",
        META/"validators.inventory.extract.json", META/"validation-bindings.json", META/"validation-rules.extract.json"]
    reference = load(files[1])
    for path, expected in reference["provenance"]["input_sha256"].items():
        if digest(ROOT/path) != expected:
            raise ValueError("Stale supplemental reference input: " + path)
    bundle = load(files[0])
    if reference["provenance"]["library_source_sha256"] != bundle["resource_environment"]["pinned_library_fingerprints"]:
        raise ValueError("Changed independent library environment")
    if reference["provenance"]["runtime_sha256"] != load(BASE/"parsed-result-valuation-pilot-controls.json")["provenance"]["runtime_sha256"]:
        raise ValueError("Changed independent runtime pin")
    doc = build(bundle, reference, *(load(p) for p in files[2:]))
    for path, expected in bundle["inputs"].items():
        if digest(ROOT/path) != expected:
            raise ValueError("Changed frozen bundle input: " + path)
    for context in bundle["contexts"]:
        if digest(ROOT/context["source"]) != context["source_sha256"]:
            raise ValueError("Changed frozen context")
    doc["input_sha256"] = {p.relative_to(ROOT).as_posix(): digest(p) for p in files + [Path(__file__)]}
    text = json.dumps(doc, indent=2, sort_keys=True) + "\n"
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != text: raise ValueError("Stale V01 dependency inventory")
    else: OUTPUT.write_text(text, encoding="utf-8")
    print("V01:", doc["metrics"], "Ecore features/operations/delegates:", len(doc["ecore_features"]),
          len(doc["ecore_operations"]), len(doc["delegate_bindings"]), "check candidates/dependencies:",
          len(doc["applicable_check_candidates"]), len(doc["resolved_semantic_dependencies"]), "native closure: open")

if __name__ == "__main__":
    main()
