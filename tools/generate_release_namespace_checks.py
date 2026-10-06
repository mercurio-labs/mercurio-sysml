"""Extract complete pinned checkImport/checkAnnotation predicates; reject source drift."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

VALIDATOR = "org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend"

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pilot-root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    def git(*command):
        return subprocess.check_output(["git", "-C", str(args.pilot_root), *command], text=True).strip()
    if git("status", "--porcelain"):
        raise ValueError("Pilot checkout must be clean")
    text = (args.pilot_root / VALIDATOR).read_text(encoding="utf-8")
    constants = dict(re.findall(r'public static val (\w+)\s*=\s*"([^"\n]*)"', text))
    match = re.search(r"def checkImport\(Import import_\)\s*\{(.*?)\n\s*\}\s*\n\s*\}", text, re.S)
    if match is None:
        raise ValueError("missing checkImport")
    body = re.sub(r"//[^\n]*", "", match[1])
    body = re.sub(r"\s+", "", body)
    expected = "if(import_.importOwningNamespace!==null&&import_.importOwningNamespace.owner===null&&import_.visibility!==VisibilityKind.PRIVATE){error(INVALID_IMPORT_TOP_LEVEL_VISIBILITY_MSG,import_,null,INVALID_IMPORT_TOP_LEVEL_VISIBILITY)"
    if body != expected:
        raise ValueError("unsupported checkImport predicate shape")
    result = {"schema_version": 1, "source": {"pilot_commit": git("rev-parse", "HEAD"), "file": VALIDATOR, "sha256": hashlib.sha256((args.pilot_root / VALIDATOR).read_bytes()).hexdigest(), "extractor": Path(__file__).name}, "import_top_level": {"method": "checkImport", "required_visibility": "private", "issue": constants["INVALID_IMPORT_TOP_LEVEL_VISIBILITY"], "message": constants["INVALID_IMPORT_TOP_LEVEL_VISIBILITY_MSG"]}}
    annotation = re.search(r"def checkAnnotation\(Annotation ann\)\s*\{(.*?)\n\s*\}\s*\n\s*@Check", text, re.S)
    if annotation is None: raise ValueError("missing checkAnnotation")
    body = re.sub(r"\s+", "", re.sub(r"//[^\n]*", "", annotation[1]))
    expected = """
        val ownedAnnotatingElement = ann.ownedAnnotatingElement
        val owningAnnotatingElement = ann.owningAnnotatingElement
        if (ownedAnnotatingElement === null && owningAnnotatingElement === null ||
            ownedAnnotatingElement !== null && owningAnnotatingElement !== null) {
            error(INVALID_ANNOTATION_ANNOTATING_ELEMENT_MSG, ann, null, INVALID_ANNOTATION_ANNOTATING_ELEMENT)
        }
        val owningAnnotatedElement = ann.owningAnnotatedElement
        if (owningAnnotatedElement !== null && ownedAnnotatingElement === null) {
            error(INVALID_ANNOTATION_ANNOTATED_ELEMNT_OWNERSHIP_MSG_1, ann, null, INVALID_ANNOTATION_ANNOTATED_ELEMNT_OWNERSHIP)
        }
        if (owningAnnotatedElement === null && ownedAnnotatingElement !== null) {
            error(INVALID_ANNOTATION_ANNOTATED_ELEMNT_OWNERSHIP_MSG_2, ann, null, INVALID_ANNOTATION_ANNOTATED_ELEMNT_OWNERSHIP)
        }
    """
    if body != re.sub(r"\s+", "", expected): raise ValueError("unsupported checkAnnotation predicate shape")
    rows = []
    for mask in range(8):
        owned, owning, annotated = [bool(mask & bit) for bit in (1, 2, 4)]
        failures = []
        if owned == owning: failures.append(("INVALID_ANNOTATION_ANNOTATING_ELEMENT", "INVALID_ANNOTATION_ANNOTATING_ELEMENT_MSG"))
        if annotated and not owned: failures.append(("INVALID_ANNOTATION_ANNOTATED_ELEMNT_OWNERSHIP", "INVALID_ANNOTATION_ANNOTATED_ELEMNT_OWNERSHIP_MSG_1"))
        if not annotated and owned: failures.append(("INVALID_ANNOTATION_ANNOTATED_ELEMNT_OWNERSHIP", "INVALID_ANNOTATION_ANNOTATED_ELEMNT_OWNERSHIP_MSG_2"))
        rows.append([{"issue": constants[issue], "message": constants[message]} for issue, message in failures])
    result["annotation"] = {"method": "checkAnnotation", "presence_bits": ["owned_annotating_element", "owning_annotating_element", "owning_annotated_element"], "failures_by_mask": rows}
    if args.check:
        if result != json.loads(args.out.read_text(encoding="utf-8")):
            raise ValueError("namespace-check artifact drift")
    else:
        args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")

if __name__ == "__main__":
    main()
