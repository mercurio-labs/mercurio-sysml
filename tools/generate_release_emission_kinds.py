"""Preserve actual Ecore metaclass identity in inherited KIR emission templates."""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("metamodel", "emission", "out"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    model = json.loads(args.metamodel.read_text(encoding="utf-8"))
    emission = json.loads(args.emission.read_text(encoding="utf-8"))
    classes = {entry["qualified_name"] for entry in model["metaclasses"]}
    metaclasses = {}
    for name, rule in emission["metaclasses"].items():
        if name in classes and rule["kir_kind"].rsplit("::", 1)[-1] != name.rsplit("::", 1)[-1]:
            metaclasses[name] = dict(rule, kir_kind=name)
    if not metaclasses:
        raise ValueError("no legacy kind erasures found")
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    result = {"source": {"authorship": {"mode": "generated", "reason": "Keep inherited IDs and property templates, replacing erased kinds only for actual extracted Ecore metaclasses."}, "metamodel_sha256": digest(args.metamodel), "emission_sha256": digest(args.emission), "extractor": "generate_release_emission_kinds.py"}, "metaclasses": metaclasses}
    if args.check:
        if result != json.loads(args.out.read_text(encoding="utf-8")):
            raise ValueError("emission-kind artifact drift")
    else:
        args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"{len(metaclasses)} Ecore-backed emission kinds verified")


if __name__ == "__main__":
    main()
