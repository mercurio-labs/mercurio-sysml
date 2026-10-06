"""Audit attached Pilot default observations against an unpromoted native KIR import."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
ECORE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json"
PIN = "692170b71867353b8f90341e61556f49a5beb0e5"
URI = "https://www.omg.org/spec/SysML/20250201#//"


def snake(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def audit(ecore, export, kir):
    metadata = export.get("metadata", {})
    if metadata.get("observed_ecore_defaults_v1") is not True:
        raise ValueError("Pilot export lacks marked Ecore getter observations")
    if metadata.get("element_count") != len(export["elements"]):
        raise ValueError("Pilot export element count does not match its content")
    if metadata.get("relationship_count") != len(export["relationships"]):
        raise ValueError("Pilot export relationship count does not match its content")
    classes = {row["id"]: row for row in ecore["classes"]}
    defaults = [row for row in ecore["features"] if row["kind"] == "attribute" and row["default_literal"] is not None]
    if len(defaults) != 24:
        raise ValueError("Pinned explicit-default inventory changed")
    ancestry = {}

    def ancestors(identity):
        if identity not in ancestry:
            if identity not in classes:
                raise ValueError("Unknown pinned Ecore class " + identity)
            ancestry[identity] = {identity}
            for parent in classes[identity]["super_types"]:
                ancestry[identity].update(ancestors(parent))
        return ancestry[identity]

    imported = {element["id"]: element for element in kir["elements"]}
    if len(imported) != len(kir["elements"]):
        raise ValueError("Duplicate KIR element identity")
    counts = Counter()
    deviations = Counter()
    for source in export["elements"]:
        identity = source["qualified_name"]
        kind = URI + source["kind"]
        parents = ancestors(kind)
        target = imported.get(identity)
        if target is None or target["kind"].rsplit("::", 1)[-1] != source["kind"]:
            raise ValueError("Missing or mistyped native element " + identity)
        for feature in defaults:
            if feature["owner"] not in parents:
                continue
            field = snake(feature["name"])
            if field not in source["properties"]:
                raise ValueError("Missing attached Pilot observation " + identity + "." + field)
            observed = source["properties"][field]
            if target["properties"].get(field) != observed or target["properties"].get("metadata", {}).get(field) != observed:
                raise ValueError("Native observed value differs from Pilot " + identity + "." + field)
            counts[field] += 1
            literal = feature["default_literal"]
            declared = literal == "true" if literal in ("true", "false") else literal
            if observed != declared:
                deviations[field] += 1
    if kir["metadata"].get("pilot_commit") != PIN:
        raise ValueError("KIR input does not identify exact pinned Pilot commit")
    return {"schema": "dev.mercurio.ecore-library-observations.v1",
            "pilot_commit": PIN, "source_elements": len(export["elements"]),
            "source_relationships": len(export["relationships"]),
            "native_elements": len(kir["elements"]), "observations": sum(counts.values()),
            "by_field": dict(sorted(counts.items())),
            "different_from_ecore_literal": dict(sorted(deviations.items()))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--export", type=Path, required=True)
    parser.add_argument("--kir", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    ecore = json.loads(ECORE.read_text(encoding="utf-8"))
    export = json.loads(args.export.read_text(encoding="utf-8"))
    kir = json.loads(args.kir.read_text(encoding="utf-8"))
    result = audit(ecore, export, kir)
    result["inputs_sha256"] = {
        "ecore": hashlib.sha256(ECORE.read_bytes()).hexdigest(),
        "pilot_export": hashlib.sha256(args.export.read_bytes()).hexdigest(),
        "native_kir": hashlib.sha256(args.kir.read_bytes()).hexdigest(),
    }
    args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"Verified {result['observations']} observed Ecore attribute values across {result['source_elements']} Pilot elements")


if __name__ == "__main__":
    main()
