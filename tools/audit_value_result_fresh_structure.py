"""Audit one frozen native fresh-construction batch; never family qualification."""
import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "docs/conformance/2026-08-support/definition-pipeline-evidence"
PREFIX = "value-result-fresh-structure"

def require(value, message):
    if not value:
        raise ValueError(message)

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def read(name):
    return json.loads((EVIDENCE / name).read_text(encoding="utf-8"))

def compare_structural(spec, reference, native, identities):
    ports = spec["ports"]
    require(spec["fixed_contexts"] == 26 and spec["source_contexts_parsed"] == 25
        and not spec["mutations_applied"], "Fixed contexts changed")
    require(len(ports) == len(reference["observations"]) == 275, "All275structural ports are required")
    require(len({(p["owner_id"], p["field"]) for p in ports}) == 275, "Duplicate structural port")
    require(len(native["queries"]) == 4462 and native["status"] == "read_only_diagnostic"
        and native["publication"] == "not_attempted" and native["qualification_certificate"] is False,
        "Read matrix pruned or promoted")
    require(reference["initial_source_node_counts"] == reference["final_source_node_counts"]
        and sum(reference["initial_source_node_counts"].values()) == 388, "Reference construction side effect")
    answers = {(q["owner_id"], q["field"]): q for q in native["queries"]}
    require(len(answers)==4462, "Duplicate native query identity")
    mapping = identities["canonical_resource_fragment_to_native_id"]
    rows = []
    for port, observed in zip(ports, reference["observations"]):
        key = (port["owner_id"], port["field"])
        require(observed["root"] == port and observed["status"] == "reference_observed",
            "Independent structural port identity or outcome changed")
        owner = observed["owner"]
        mapped = mapping.get(owner["resource"].replace(chr(92), "/") + "#" + owner["emf_fragment"])
        require(mapped == key[0] and owner["kind"] == port["owner_kind"].split("::")[-1],
            "Independent owner or metaclass changed")
        resolved = observed["feature_declaring_kind"] + "/" + observed["feature"]
        require(any(f.endswith("#//" + resolved) for f in port["feature_candidates"])
            and not observed["derived"], "Observed feature is not the stored imported contract")
        require(not observed["provider_completion_before"] and not observed["provider_completion_after"]
            and observed["source_nodes_before"] == observed["source_nodes_after"]
            and observed["owner_relationships_before"] == observed["owner_relationships_after"],
            "Prepared reference or getter state change")
        require(observed["endpoints"] == [], "Independent stored port is not empty")
        answer = answers[key]
        require(answer["status"] == "query_evaluated" and answer["targets"] == [],
            "Native stored read still requires work or differs")
        rows.append({"owner_id": key[0], "field": key[1], "context": port["context"],
                     "comparison": "exact_empty_match", "complete_native_context": False})
    return rows

def compare_fresh_graph(old, new):
    require(len(old) == len(new) and [e["id"] for e in old] == [e["id"] for e in new],
        "Fresh constructor identities or order changed")
    fields = {"owned_relationship", "owned_related_element", "owning_relationship", "owning_related_element"}
    inserted = Counter()
    for before, after in zip(old, new):
        require({k:v for k,v in before.items() if k!="properties"} ==
                {k:v for k,v in after.items() if k!="properties"}, "Element identity/metaclass/layer changed")
        for field, value in before["properties"].items():
            require(after["properties"].get(field) == value and field in after["properties"],
                "Explicit existing field was overwritten: " + field)
        for field in after["properties"].keys() - before["properties"].keys():
            require(field in fields, "Unexpected constructor property: " + field)
            expected = [] if field.startswith("owned_") else None
            require(after["properties"][field] == expected, "Fresh slot has fabricated edges")
            inserted[field] += 1
        require("owning_relationship" in after["properties"] and "owned_relationship" in after["properties"],
            "Fresh Element structural slot absent")
        require(after["properties"].get("is_implied_included") is not True, "Native provider was prepared")
    return dict(sorted(inserted.items()))

def audit(check=False):
    scope = read(PREFIX + "-scope.json")
    require(scope["baseline"]["unregistered_read_ports"] == 275 and len(scope["imported_contracts"]) == 4,
        "Frozen structural dependency scope changed")
    for path, digest in scope["frozen_sha256"].items():
        require(sha(ROOT/path) == digest, "Frozen input changed: " + path)
    runs = ["build", "native", "library", "pilot-actual", "pilot-controls", "focused", "native-regression"]
    for stage in runs:
        run = read(PREFIX + "-" + stage + "-run.json")
        require(run["exit_code"] == 0 and run["inputs_unchanged"], "Execution failed or inputs changed: " + stage)
        for group in ["input_sha256", "consumer_sha256", "source_sha256", "source_input_sha256"]:
            for path, digest in run.get(group, {}).items():
                require(sha(ROOT/path) == digest, "Execution input stale: " + path)
        if "log_sha256" in run:
            log_name = PREFIX + "-" + ("pilot-actual" if stage=="pilot-actual" else stage) + ".log"
            if stage == "pilot-controls": log_name = PREFIX + "-pilot-controls.log"
            require(sha(EVIDENCE/log_name) == run["log_sha256"], "Execution log changed: " + stage)
    focused=read(PREFIX+"-focused-run.json")
    require(sha(Path(focused["executable"]))==focused["executable_sha256"], "Focused test executable changed")
    focused_text=(EVIDENCE/(PREFIX+"-focused.log")).read_text(encoding="utf-8")
    require("test result: FAILED" not in focused_text
        and len(re.findall(r"^test .* \.\.\. ok$", focused_text, re.M))==25, "All25focused controls must pass")
    regression=read(PREFIX+"-native-regression-run.json")
    require(regression["binary_sha256"]==focused["executable_sha256"], "Regression executable differs")
    regression_text=(EVIDENCE/(PREFIX+"-native-regression.log")).read_text(encoding="utf-8")
    require("test result: ok. 478 passed; 0 failed;" in regression_text
        and len(re.findall(r"^test .* \.\.\. ok$", regression_text, re.M))==478,
        "All478native regressions must pass")
    build = read(PREFIX+"-build-run.json")
    require(sha(ROOT/"target/release/audit_release_compile.exe") == build["binary_sha256"], "Native executable stale")
    pilot = read(PREFIX+"-pilot-actual-run.json")
    require(sha(Path(pilot["runs"][0]["command"][4])) == pilot["jar_sha256"], "Pinned Pilot executable changed")
    for path,digest in pilot["pinned_library_source_sha256"].items():
        require(sha(ROOT.parent/"target/upstream/SysML-v2-Release"/path)==digest, "Pinned library changed")
    for name,expected in [(PREFIX+"-pilot-observations.json",pilot["output_sha256"]),
        (PREFIX+"-pilot-spec.json",pilot["spec_sha256"]),
        (PREFIX+"-native-queries.json",read(PREFIX+"-native-run.json")["output_sha256"])]:
        require(sha(EVIDENCE/name)==expected, "Current data changed: "+name)
    library_path = EVIDENCE/(PREFIX+"-library-inspection.jsonl")
    require(sha(library_path)==read(PREFIX+"-library-run.json")["output_sha256"], "Fresh library producer changed")
    old_library = read("value-result-parser-sharing-actual-inspection.jsonl")
    new_library = read(PREFIX+"-library-inspection.jsonl")
    require(new_library["element_count"]==93320 and new_library["pending_reference_count"]==18876
        and len(new_library["input_files"])==94 and new_library["publication"]=="not_attempted",
        "Complete fresh library diagnostic was pruned or promoted")
    require(old_library["inspection"]["pending_references"]==new_library["inspection"]["pending_references"],
        "Unfinished library reference registry changed")
    assembly=read("value-result-lifecycle-reference-read-assembly.json")
    for path,digest in assembly["assembly_input_sha256"].items():
        require(sha(ROOT/path)==digest, "Historical constructor evidence changed: "+path)
    source_run=read(PREFIX+"-source-run.json")
    source_path=EVIDENCE/(PREFIX+"-source-inspection.jsonl")
    require(source_run["inputs_unchanged"] and sha(source_path)==source_run["output_sha256"],
        "Current source constructor changed")
    old_sources=[json.loads(line) for line in (EVIDENCE/"value-result-lifecycle-source-inspection.jsonl").read_text(encoding="utf-8").splitlines()]
    new_sources=[json.loads(line) for line in source_path.read_text(encoding="utf-8").splitlines()]
    require(len(old_sources)==len(new_sources)==26
        and Counter(row["status"] for row in new_sources)=={"unlinked_inspection":25,"blocked":1},
        "Fixed source outcome denominator changed")
    new_source_slots=Counter()
    for before,after in zip(old_sources,new_sources):
        require(before["relative_path"]==after["relative_path"] and before["status"]==after["status"],
            "Source outcome or identity changed")
        if after["status"]=="unlinked_inspection":
            require(before["inspection"]["pending_references"]==after["inspection"]["pending_references"],
                "Source pending registry changed")
            new_source_slots.update(compare_fresh_graph(before["inspection"]["constructed_elements"],
                after["inspection"]["constructed_elements"]))
    negative=read(PREFIX+"-parse-negative.jsonl")
    require(negative["relative_path"]=="valuation-missing-value" and negative["stage"]=="syntax"
        and negative["status"]=="blocked", "Required syntax negative changed stage")
    inserted=compare_fresh_graph(old_library["inspection"]["constructed_elements"],
                                 new_library["inspection"]["constructed_elements"])
    controls=read(PREFIX+"-pilot-controls.json")
    require(len(controls["observations"])==167 and all(not row["completion_flag"]
        and all(value in [None,[]] for value in row["slots"].values()) for row in controls["observations"]),
        "Fresh factory reference incomplete or prepared")
    rows=compare_structural(read(PREFIX+"-pilot-spec.json"),read(PREFIX+"-pilot-observations.json"),
        read(PREFIX+"-native-queries.json"),read("value-result-lifecycle-shared-services-source-identities.json"))
    proof={"schema":"dev.mercurio.fresh-structure-batch-comparison.v1","qualification_certificate":False,
        "structural_ports_verified":len(rows),"concrete_factory_classes_verified":167,"focused_native_controls_passed":25,"native_regressions_passed":478,
        "fresh_library_nodes":93320,"unfinished_library_ports":18876,"new_library_slots":inserted,"new_source_slots":dict(sorted(new_source_slots.items())),
        "native_full_read_counts":read(PREFIX+"-native-run.json")["counts"],
        "complete_native_contexts":0,"strict_families_qualified":0,"candidate_promoted":False,
        "ports":rows,"boundary":"Fresh stored construction only. Real linking/producer prerequisites, algorithms, validation and complete lifecycles remain unqualified."}
    proof["input_sha256"]={str(path.relative_to(ROOT)).replace(chr(92), "/"):sha(path)
        for path in [Path(__file__),ROOT/"tools/test_audit_value_result_fresh_structure.py",
            *[EVIDENCE/(PREFIX+"-"+stage+"-run.json") for stage in runs],
            EVIDENCE/(PREFIX+"-native-queries.json"),EVIDENCE/(PREFIX+"-pilot-observations.json"),
            EVIDENCE/(PREFIX+"-pilot-spec.json"),EVIDENCE/(PREFIX+"-scope.json"),
            EVIDENCE/(PREFIX+"-normative-review.json"),EVIDENCE/(PREFIX+"-source-run.json"),
            EVIDENCE/(PREFIX+"-assembly.json")]}
    target=EVIDENCE/(PREFIX+"-comparison.json")
    rendered=json.dumps(proof,indent=2)+"\n"
    if check: require(target.read_text(encoding="utf-8")==rendered, "Structural comparison stale")
    else: target.write_text(rendered,encoding="utf-8",newline="\n")
    return proof

if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--check",action="store_true")
    result=audit(parser.parse_args().check)
    print(json.dumps({k:v for k,v in result.items() if k not in ["ports"]},indent=2))
