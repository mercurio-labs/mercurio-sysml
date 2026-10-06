"""Qualify and export the fixed definition-driven core preview, without Java.

This is a separate bounded acceptance boundary, never a release certificate.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "docs/conformance/2026-08-support"
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
PIN = "692170b71867353b8f90341e61556f49a5beb0e5"


def sha(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify_contract(contract, reference):
    require(contract["schema"] == "dev.mercurio.definition-core-preview-contract.v1", "wrong preview contract")
    require(contract["pilot_commit"] == reference["provenance"]["pilot_revision"] == PIN, "changed Pilot pin")
    controls = reference["controls"]
    require(len(controls) == len(contract["cases"]) == 98, "changed supported subset")
    require(sum(c["accepted"] for c in controls) == contract["positive_controls"] == 90, "changed positives")
    require(contract["syntax_negative_controls"] == 8, "changed negatives")
    require(contract["full_release_qualification"] is False and contract["candidate_promoted"] is False, "preview scope overclaim")
    for i, (case, expected) in enumerate(zip(contract["cases"], controls)):
        require(case == {"case_id": f"core-{i:03}", "language": expected["language"],
                         "source_sha256": hashlib.sha256(expected["source"].encode()).hexdigest(),
                         "expected_stage": "core_constructed" if expected["accepted"] else "syntax"}, "changed case identity or scope")


class Contracts:
    """Structural-only lookup through the resolved imported Ecore inheritance."""
    def __init__(self, effective):
        self.classes = {c["id"].split("#//")[-1]: c for c in effective["classes"]}
        self.fields = {}
        for feature in effective["features"]:
            field = re.sub(r"(?<!^)(?=[A-Z])", "_", feature["name"]).lower()
            self.fields[(feature["owner"].split("#//")[-1], field)] = feature

    def feature(self, kind, field):
        pending, visited = [kind.rsplit("::", 1)[-1]], set()
        while pending:
            current = pending.pop(0)
            if current in visited:
                continue
            visited.add(current)
            if (current, field) in self.fields:
                return self.fields[(current, field)]
            require(current in self.classes, "unknown canonical metaclass " + current)
            pending.extend(p.split("#//")[-1] for p in self.classes[current]["super_types"])
        return None


def refs(value):
    if value is None:
        return []
    if isinstance(value, str):
        return [value]
    require(isinstance(value, list) and all(isinstance(v, str) for v in value), "invalid reference value")
    return value


def stored(contract):
    return contract is not None and not any(contract[f] for f in ("derived", "volatile", "transient"))


def compare_structure(document, expected, contracts, reference_views=None):
    elements = document["elements"]
    index = {element["id"]: element for element in elements}
    require(len(index) == len(elements), "duplicate canonical identity")
    children = set()
    for element in elements:
        for field, value in element["properties"].items():
            contract = contracts.feature(element["kind"], field)
            if stored(contract) and contract["containment"]:
                children.update(refs(value))
    roots = [e for e in elements if e["id"] not in children]
    require(len(roots) == 1 and roots[0]["kind"].rsplit("::", 1)[-1] == "Namespace", "wrong canonical root")
    paths, by_path, pending = {}, {}, [(roots[0], "$")]
    while pending:
        element, path = pending.pop()
        require(element["id"] not in paths, "cyclic or duplicate containment")
        paths[element["id"]], by_path[path] = path, element
        for field, value in element["properties"].items():
            contract = contracts.feature(element["kind"], field)
            if stored(contract) and contract["containment"]:
                for ordinal, identity in enumerate(refs(value)):
                    require(identity in index, "unclosed containment")
                    pending.append((index[identity], f"{path}/{field}/{ordinal}"))
    expected_nodes = {n["path"]: n for n in expected["nodes"]}
    require(len(paths) == len(index) and set(by_path) == set(expected_nodes), "missing, disconnected or extra nodes")
    expected_links = {(link["owner_path"], link["field"]): link["target_paths"] for link in expected["links"]}
    actual_links = {}
    for path, element in by_path.items():
        oracle, properties = expected_nodes[path], element["properties"]
        require(element["kind"].rsplit("::", 1)[-1] == oracle["kind"], "metaclass mismatch at " + path)
        actual_children = {}
        for field, value in properties.items():
            contract = contracts.feature(element["kind"], field)
            if not stored(contract):
                continue
            if contract["kind"] == "reference":
                identities = refs(value)
                require(all(identity in paths for identity in identities), "unclosed stored endpoint")
                if contract["containment"]:
                    actual_children[field] = [paths[identity] for identity in identities]
                elif not contract["container"] and identities:
                    actual_links[(path, field)] = [paths[identity] for identity in identities]
        require(actual_children == oracle["children"], "containment order mismatch at " + path)
        fields = set(oracle["attributes"])
        fields.update(field for field, value in properties.items()
                      if value is not None and (c := contracts.feature(element["kind"], field))
                      and stored(c) and c["kind"] == "attribute" and field != "element_id")
        # Overridden owning-membership name getters are independently compared by
        # the existing exact native Pilot/roundtrip test. This export is storage,
        # not a Python reimplementation of their semantic name algorithm.
        names = {"member_name", "member_short_name"}
        actual_attributes = {}
        for field in fields - names:
            contract = contracts.feature(element["kind"], field)
            require(stored(contract) and contract["kind"] == "attribute", "unknown stored attribute")
            value = properties.get(field, [] if contract["upper_bound"] != 1 else contract["default_value"])
            if value is not None:
                actual_attributes[field] = value
        require(actual_attributes == {k: v for k, v in oracle["attributes"].items() if k not in names},
                "stored attribute mismatch at " + path)
    require(all(expected_links.get(key) == value for key, value in actual_links.items()), "extra or incorrect raw stored reference")
    if reference_views is not None:
        native_views = {}
        for view in reference_views:
            key = (view["owner_path"], view["field"])
            require(key not in native_views and view["owner_path"] in by_path, "duplicate or unknown native reference view")
            require(all(identity in paths for identity in view["target_ids"]), "unclosed native reference view")
            native_views[key] = [paths[identity] for identity in view["target_ids"]]
        require(native_views == expected_links, "native reference views differ from independent endpoints/order")
    else:
        require(actual_links == expected_links, "stored reference endpoints/order mismatch")
    return len(index), len(expected_links)


def verify_rows(rows, contract, reference, effective):
    require(len(rows) == len(contract["cases"]), "missing or extra native results")
    by_id = {row["case_id"]: row for row in rows}
    require(len(by_id) == len(rows), "duplicate native case result")
    contracts = Contracts(effective)
    positive = negative = nodes = links = 0
    for case, expected in zip(contract["cases"], reference["controls"]):
        require(case["case_id"] in by_id, "missing original native case")
        row = by_id[case["case_id"]]
        require(row["schema"] == "dev.mercurio.definition-core-preview-model.v1", "wrong native export schema")
        require(row["language"] == case["language"] and row["definition_profile"] == contract["profile"], "wrong native context")
        require(row["qualification_certificate"] is False and row["candidate_promoted"] is False,
                "native export claims release qualification")
        require(row["semantic_validation"] == row["transformation_completion"] == "not_assessed", "semantic stage overclaim")
        if expected["accepted"]:
            require(row["status"] == "core_constructed" and row["kir_roundtrip"] == "exact", "failed supported source")
            require(row["stage"] == "stored_structure_and_supported_linking", "wrong positive stage")
            require("abstract_syntax" in row, "missing abstract-syntax export")
            count, slots = compare_structure(row["kir_document"], expected, contracts, row["reference_views"])
            require(row["element_count"] == count, "false node count")
            require(row["kir_document"]["metadata"]["semantic_validation"] == "not_assessed", "KIR semantic overclaim")
            require(compare_structure(row["roundtrip_kir_document"], expected, contracts, row["roundtrip_reference_views"]) == (count, slots), "fresh import changed structure")
            nodes += count
            links += slots
            positive += 1
        else:
            require(row["status"] == "blocked" and row["stage"] == "syntax", "unsupported execution cannot satisfy syntax rejection")
            require(not any(k in row for k in ("kir_document", "abstract_syntax", "roundtrip_kir_document")), "negative exported partial model")
            negative += 1
    require((positive, negative, nodes, links) == (90, 8, contract["expected_native_nodes"], contract["expected_reference_slots"]), "changed native matrix")
    return dict(supported_documents=positive, syntax_negatives=negative, canonical_nodes=nodes, stored_reference_slots=links)


def verify_preservation():
    manifest = read(BASE / "definition-core-preview-preservation.json")
    allowed = set(manifest["allowed_preview_edits"])
    for path, digest in manifest["retained_sha256"].items():
        if path not in allowed:
            require(sha(ROOT / path) == digest, "deferred semantic evidence changed: " + path)
    for path, row in manifest["archived_witnesses"].items():
        require(sha(ROOT / row["archive"]) == row["sha256"], "lost semantic witness: " + path)
    require(sha(ROOT / manifest["deferred_plan"]) == manifest["deferred_plan_sha256"], "changed full-release acceptance")
    return dict(retained_files=len(manifest["retained_sha256"]), archived_witnesses=len(manifest["archived_witnesses"]),
                full_release_criteria_preserved=True, candidate_promoted=False)


def verify_sources(reference):
    pilot = ROOT.parent / "target/support-2026-08/pilot-pinned-2026-08"
    grammar, effective, semantics = [read(PROFILE / name) for name in
        ("grammar.structure.extract.json", "ecore-effective.extract.json", "ecore-semantics.extract.json")]
    require(grammar["provenance"]["pilot_revision"] == effective["provenance"]["pilot_commit"] == semantics["source"]["pilot_commit"] == PIN, "mixed definition pins")
    require(grammar["coverage"]["unresolved_references"] == grammar["coverage"]["unsupported_nodes"] == 0, "unresolved imported definition")
    for path, digest in grammar["provenance"]["sources_sha256"].items():
        require(sha(pilot / path) == digest, "pinned grammar/Ecore source changed: " + path)
    require(reference["provenance"]["grammar_sha256"] == sha(PROFILE / "grammar.structure.extract.json"), "reference grammar drift")
    for path, digest in reference["provenance"]["source_sha256"].items():
        normalized = (pilot / path).read_bytes().replace(b"\r\n", b"\n")
        require(hashlib.sha256(normalized).hexdigest() == digest, "cached independent reference source changed: " + path)
    for name in ("xtext-terminal-programs.json", "xtext-assignment-contracts.json", "xtext-fragment-programs.json", "xtext-finite-programs.experimental.json"):
        for path, digest in read(PROFILE / name).get("source_sha256", {}).items():
            require(sha(PROFILE / path) == digest, "native definition input drift: " + path)
    return effective


def run(command, folder, ordinal, expected_count=None, env=None):
    log = folder / f"check-{ordinal:02}.log"
    with log.open("w", encoding="utf-8") as output:
        code = subprocess.run(command, cwd=ROOT, stdout=output, stderr=subprocess.STDOUT, env=env).returncode
    text = log.read_text(encoding="utf-8", errors="replace")
    require(code == 0, "check failed: " + str(command) + "\n" + text[-6000:])
    if expected_count is not None:
        require(re.search(rf"test result: ok\. {expected_count} passed; 0 failed;", text) is not None, "missing required native test count")
    print("passed:", " ".join(map(str, command[:6])), flush=True)
    return dict(command=list(map(str, command)), exit_code=code, log=str(log.relative_to(ROOT)), log_sha256=sha(log), test_count=expected_count)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "target/definition-core-preview" / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ"))
    args = parser.parse_args()
    folder = args.out.resolve()
    folder.mkdir(parents=True, exist_ok=False)
    result = dict(schema="dev.mercurio.definition-core-preview-acceptance.v1", status="running", preview_complete=False,
                  qualification_certificate=False, full_release_qualified=False, candidate_promoted=False, checks=[])
    save(folder / "acceptance.json", result)
    try:
        contract = read(BASE / "definition-core-preview-contract.json")
        require(sha(ROOT / contract["reference_path"]) == contract["reference_sha256"], "changed frozen independent reference")
        reference = read(ROOT / contract["reference_path"])
        verify_contract(contract, reference)
        effective = verify_sources(reference)
        result["preservation"] = verify_preservation()
        result["input_sha256"] = {str(p.relative_to(ROOT)).replace("\\", "/"): sha(p) for p in
                                  sorted([*PROFILE.rglob("*.json"), * (ROOT / "crates/mercurio-sysml/src").rglob("*.rs"),
                                          ROOT / "crates/mercurio-tools/src/bin/audit_release_compile.rs",
                                          ROOT / "crates/mercurio-tools/src/bin/audit_release_compile/core_preview.rs",
                                          Path(__file__), ROOT / "tools/test_definition_core_preview.py",
                                          BASE / "definition-core-preview-contract.json"])}
        commands = [[sys.executable, "-B", "tools/" + script, *extra] for script, extra in [
            ("generate_ecore_model.py", ["--check"]),
            ("generate_xtext_terminal_programs.py", ["--check"]),
            ("generate_xtext_assignment_contracts.py", ["--check"]),
            ("generate_xtext_fragment_programs.py", ["--check"]),
            ("generate_xtext_partial_programs.py", ["--finite-graph", "--check"])]]
        commands.append([sys.executable, "-B", "-m", "unittest", "discover", "-s", "tools", "-p", "test_definition_core_preview.py"])
        for command in commands:
            result["checks"].append(run(command, folder, len(result["checks"])))
        native_test = ROOT / "target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe"
        require(native_test.is_file(), "build native release library tests before qualifying the preview")
        baseline = read(BASE / "definition-pipeline-evidence/first-library-featuring-checks-run.json")
        require(sha(native_test) == baseline["binary_sha256"], "native test binary lacks retained source/executable witness")
        for selector, count in [
            ("definition_document::pilot_tests::definition_documents_match_independent_pilot_models_links_and_roundtrips", 1),
            ("language_frontend::lowering::ecore_model::tests::", 19),
            ("xtext_terminal::tests::", 11),
            ("definition_document::definition_fresh_structure_tests::", 5),
            ("definition_document::projection_tests::", 5)]:
            result["checks"].append(run([str(native_test), selector, "--test-threads=1"], folder, len(result["checks"]), count))
        result["native_test_binary_sha256"] = sha(native_test)
        for command, count in [
            (["cargo", "test", "-p", "mercurio-tools", "--bin", "audit_release_compile", "--release", "core_preview::tests::", "--", "--test-threads=1"], 2),
            (["cargo", "test", "-p", "mercurio-tools", "--bin", "audit_release_compile", "--release", "cached_definition_", "--", "--test-threads=1"], 13),
            (["cargo", "build", "-p", "mercurio-tools", "--bin", "audit_release_compile", "--release"], None)]:
            result["checks"].append(run(command, folder, len(result["checks"]), count))
        spec = {"cases": [dict(case_id=case["case_id"], language=case["language"], source=expected["source"], reference_queries=[dict(owner_path=link["owner_path"], field=link["field"]) for link in expected["links"]])
                          for case, expected in zip(contract["cases"], reference["controls"])]}
        save(folder / "sources.json", spec)
        binary = ROOT / "target/release/audit_release_compile.exe"
        env = dict(os.environ)
        env["JAVA_HOME"] = str(folder / "java-not-installed")
        env["PATH"] = str(binary.parent) + os.pathsep + str(Path(env.get("SystemRoot", "C:/Windows")) / "System32")
        result["checks"].append(run([str(binary), "--definition-core-preview", str(folder / "sources.json"),
                                      str(folder / "models.jsonl")], folder, len(result["checks"]), env=env))
        rows = [json.loads(line) for line in (folder / "models.jsonl").read_text(encoding="utf-8").splitlines()]
        result["matrix"] = verify_rows(rows, contract, reference, effective)
        result["native_execution_without_java"] = True
        bundle = folder / "bundle"
        definitions = bundle / "definitions"
        definitions.mkdir(parents=True)
        for path in PROFILE.rglob("*.json"):
            destination = definitions / path.relative_to(PROFILE)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, destination)
        for name in ("sources.json", "models.jsonl"):
            shutil.copy2(folder / name, bundle / name)
        for name in ("definition-core-preview-contract.json", "definition-core-preview-preservation.json"):
            shutil.copy2(BASE / name, bundle / name)
        require(all((ROOT / "crates/mercurio-sysml/src" / p).is_file() for p in ["xtext_fragment.rs", "xtext_terminal.rs", "xtext_assignment_contract.rs", "language_frontend/lowering/ecore_model.rs", "language_frontend/lowering/emit/operand_construction.rs"]), "missing native consumer")
        checklist = (BASE / "remaining-gap-checklist.md").read_text(encoding="utf-8")
        require(all(f"P{i} —" in checklist for i in range(1, 7)), "preview checklist lacks fixed acceptance items")
        require((BASE / "definition-core-preview.md").is_file(), "missing preview handoff")
        shutil.copy2(BASE / "definition-core-preview.md", bundle / "README.md")
        entrypoints = dict(profile=contract["profile"], standalone_crate=False, runtime_java=False,
            native_library="mercurio-sysml", required_workspace_dependencies=["mercurio-foundation"],
            APIs=["definition_document::parse_and_link", "definition_document::parse_and_link_sources",
                  "definition_document::inspect_source_structure", "definition_document::query_constructed_references",
                  "abstract_syntax_json::export_sysml_abstract_syntax_value", "abstract_syntax_json::import_sysml_abstract_syntax_value"],
            imported_definition_consumers=["xtext_fragment.rs", "xtext_terminal.rs", "xtext_assignment_contract.rs",
                "language_frontend/lowering/ecore_model.rs", "language_frontend/lowering/emit/operand_construction.rs"],
            handwritten_dependencies=["Xtext execution machine", "grammar-to-canonical-storage lowering and parser postprocessing",
                "namespace scope, visibility/import traversal and supported reference linking", "effective naming and bounded inherited/default scope",
                "stored Ecore integrity checks and abstract-syntax persistence"],
            unsupported_semantics=contract["deferred"])
        save(bundle / "native-entrypoints.json", entrypoints)
        result["input_files_unchanged"] = all(sha(ROOT / path) == digest for path, digest in result["input_sha256"].items())
        require(result["input_files_unchanged"], "preview implementation changed during acceptance")
        result["preservation"] = verify_preservation()
        result.update(status="passed", preview_complete=True, acceptance_items_passed=6, acceptance_items_required=6,
                      native_cli_sha256=sha(binary), contract_sha256=sha(BASE / "definition-core-preview-contract.json"),
                      deferred_full_qualification=dict(contexts=0, obligations=0, families=0, release_gates=0),
                      bundle_sha256={str(path.relative_to(bundle)).replace("\\", "/"): sha(path) for path in sorted(bundle.rglob("*")) if path.is_file()})
        save(folder / "acceptance.json", result)
        save(BASE / "definition-core-preview-acceptance.json", result)
        print("CORE PREVIEW PASSED: 6/6 acceptance items; 90 supported documents; 8 syntax negatives; full release deferred.")
    except Exception as error:
        result.update(status="failed", error=str(error), preview_complete=False)
        save(folder / "acceptance.json", result)
        raise


if __name__ == "__main__":
    main()
