"""Generate additive candidate KIR field contracts from selected Ecore features."""
import argparse
import hashlib
import json
from pathlib import Path
import re


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("metamodel", "selection", "out"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    model = json.loads(args.metamodel.read_text(encoding="utf-8"))
    selection = json.loads(args.selection.read_text(encoding="utf-8"))
    snake = lambda value: re.sub(r"(?<!^)(?=[A-Z])", "_", value).lower()
    fields = []
    for name in selection["fields"]:
        matches = [f for f in model["structural_features"] if snake(f["name"]) == name]
        kinds = {"Scalar" if f["kind"] == "attribute" else "Reference" if f["upper_bound"] == 1 else "ReferenceList" for f in matches}
        if len(kinds) != 1:
            raise ValueError(f"Missing or ambiguous structural field {name}: {kinds}")
        fields.append({"field": name, "kind": kinds.pop(), "source": {"classification": "pilot-feature", "pilot_features": sorted(f["qualified_name"] for f in matches)}})
    fields.extend(selection["extensions"])
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    result = {"schema": "dev.mercurio.field-specs.v1", "source": {"metamodel_sha256": digest(args.metamodel), "selection_sha256": digest(args.selection), "extractor": "generate_release_fields.py"}, "fields": sorted(fields, key=lambda f: f["field"])}
    if args.check:
        if result != json.loads(args.out.read_text(encoding="utf-8")):
            raise ValueError("candidate field contract drift")
    else:
        args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"{len(fields)} candidate fields verified")


if __name__ == "__main__":
    main()
