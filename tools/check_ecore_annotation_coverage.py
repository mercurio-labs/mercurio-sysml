"""Classify every pinned annotation; distinguish metadata from executable obligations."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
INPUT = PROFILE / "ecore-semantics.extract.json"
OUTPUT = ROOT / "docs/conformance/2026-08-support/ecore-annotation-coverage.json"
# Reviewed dialects and exact payload shapes, not an interpreter for arbitrary annotation text.
DIALECTS = {
    "http://www.eclipse.org/emf/2002/GenModel": ("documentation", {"documentation"}),
    "http://www.eclipse.org/emf/2002/Ecore": ("delegate_registration", {"settingDelegates", "invocationDelegates"}),
    "http://www.omg.org/spec/SysML": ("delegate_binding", set()),
    "http://schema.omg.org/spec/MOF/2.0/emof.xml#Property.oppositeRoleName": ("opposite_role_name", {"body"}),
    "subsets": ("subsets", set()),
    "redefines": ("redefines", set()),
    "union": ("union", set()),
}


def classify(model):
    elements = {e["id"]: e for e in model["elements"]}
    if len(elements) != len(model["elements"]):
        raise ValueError("Duplicate Ecore identity")
    bindings = {b["element"] for b in model["delegate_bindings"]}
    records = []
    for element in model["elements"]:
        for index, annotation in enumerate(element["annotations"]):
            attrs = annotation["attributes"]
            source = attrs.get("source")
            if source not in DIALECTS:
                raise ValueError("Unreviewed annotation dialect: " + str(source))
            category, keys = DIALECTS[source]
            expected_attrs = {"source", "references"} if category in {"subsets", "redefines"} else {"source"}
            if set(attrs) != expected_attrs or annotation["tag"] != "eAnnotations":
                raise ValueError("Unreviewed annotation attributes: " + source)
            details = {}
            for child in annotation.get("children", []):
                if child["tag"] != "details" or set(child["attributes"]) != {"key", "value"} or child.get("children"):
                    raise ValueError("Unreviewed annotation payload: " + source)
                key = child["attributes"]["key"]
                if key in details: raise ValueError("Duplicate annotation detail: " + key)
                details[key] = child["attributes"]["value"]
            if set(details) != keys: raise ValueError("Unreviewed annotation detail keys: " + source)
            references = []
            if category in {"subsets", "redefines"}:
                for reference in attrs["references"].split():
                    target = element["id"].split("#", 1)[0] + reference if reference.startswith("#") else reference
                    if target not in elements: raise ValueError("Unresolved annotation reference: " + reference)
                    if elements[target]["kind"] not in {"EReference", "EAttribute"} or element["kind"] not in {"EReference", "EAttribute"}:
                        raise ValueError("Property annotation requires structural features")
                    references.append(target)
                if not references: raise ValueError("Empty property annotation")
            if category == "delegate_binding" and element["id"] not in bindings:
                raise ValueError("Delegate marker lacks resolved binding evidence")
            records.append({
                "id": element["id"] + "@annotation/" + str(index),
                "element": element["id"], "category": category,
                "annotation": annotation, "resolved_reference_ids": references,
                "support": "bounded_native_dispatch" if category == "redefines" else "metadata_only" if category == "documentation" else "semantic_dependency_open",
            })
    counts = Counter(r["category"] for r in records)
    return {
        "schema": "dev.mercurio.ecore-annotation-coverage.v1",
        "meaning": "Classification and reference resolution do not execute semantics. Documentation formulas are reference text, not executable constraints. Opposite role names do not establish Ecore opposite maintenance.",
        "annotation_count": len(records), "category_counts": dict(sorted(counts.items())),
        "records": records,
        "remaining_dependencies": {
            "subsets": "Native subset derivation/consistency and update semantics, including derived or externally resolved values, are unqualified.",
            "union": "Namespace.membership union derivation must compose the declared subsets; storing the marker is not execution.",
            "opposite_role_name": "Determine each role annotation's semantic consequence separately from actual eOpposite contracts; no inverse algorithm is inferred from a label.",
            "delegate_registration": "Registration selects possible delegates; each required selected algorithm still needs native implementation and verification.",
            "delegate_binding": "Delegate source provenance is imported; algorithm coverage remains in the delegate coverage row.",
            "redefines": "Existing generated dispatch handles resolved property redefinitions; general value/update semantics and all ambiguity cases remain unqualified.",
            "documentation": "Preserve exact documentation text; formulas require separately identified normative/executable semantics before implementation credit.",
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result = classify(json.loads(INPUT.read_text(encoding="utf-8")))
    result["input_sha256"] = hashlib.sha256(INPUT.read_bytes()).hexdigest()
    data = json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != data: raise ValueError("Stale annotation coverage")
    else: OUTPUT.write_text(data, encoding="utf-8")
    print("Annotation inventory:", result["annotation_count"], result["category_counts"])


if __name__ == "__main__": main()
