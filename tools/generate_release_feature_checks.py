"""Extract two bounded KerML feature predicates and constant postprocessing.

This selects branches of checkFeature; it does not claim the whole method.
Unsupported selected predicate shapes fail closed on upstream drift.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

VALIDATOR = "org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend"
PROCESSOR = "org.omg.kerml.xtext/src/org/omg/kerml/xtext/postprocessing/FeatureParserPostProcessor.java"


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
    compact = lambda value: re.sub(r"\s+", "", value)
    text = (args.pilot_root / VALIDATOR).read_text(encoding="utf-8")
    constants = dict(re.findall(r'public static val (\w+)\s*=\s*"([^"\n]*)"', text))
    selected = []
    shapes = {
        "validateFeatureIsVariable": ('f.isVariable&&(f.owningType===null||!TypeUtil.specializes(f.owningType,SysMLLibraryUtil.getLibraryType(f,"TYPE")))', "variable_occurrence_owner"),
        "validatePortionNotVariable": ('f.isPortion&&f.isVariable', "portion_not_variable"),
    }
    for marker, (shape, operation) in shapes.items():
        block = re.search(r"// " + marker + r"\s+if \((.*?)\)\s*\{\s*error\((\w+), f, null, (\w+)\)\s*\}", text, re.S)
        if block is None:
            raise ValueError("missing selected feature branch: " + marker)
        predicate, message, issue = block.groups()
        anchors = re.findall(r'getLibraryType\(f, "([^"\n]+)"\)', predicate)
        if compact(re.sub(r'"[^"\n]+"', '"TYPE"', predicate)) != shape:
            raise ValueError("unsupported feature predicate: " + marker)
        row = {"operation": operation, "context": "Feature", "method": "checkFeature", "issue": constants[issue], "message": constants[message], "source_file": VALIDATOR, "line": text[:block.start()].count("\n") + 1}
        if operation == "variable_occurrence_owner":
            if len(anchors) != 1: raise ValueError("expected one occurrence anchor")
            row["owner_type"] = anchors[0]
        selected.append(row)
    post = (args.pilot_root / PROCESSOR).read_text(encoding="utf-8")
    method = re.search(r"protected void setIsVariableIfConstant\(\)\s*\{", post)
    if method is None:
        raise ValueError("missing constant-feature postprocessor")
    end, depth = method.end(), 1
    while depth and end < len(post):
        depth += (post[end] == "{") - (post[end] == "}")
        end += 1
    if depth or compact(post[method.end():end-1]) != 'Featuretarget=getTarget();if(target.isConstant()){target.setIsVariable(true);}':
        raise ValueError("unsupported constant-feature postprocessor")
    result = {"schema_version": 1, "source": {"pilot_commit": git("rev-parse", "HEAD"), "pilot_dirty": False, "files": {name: hashlib.sha256((args.pilot_root / name).read_bytes()).hexdigest() for name in [VALIDATOR, PROCESSOR]}, "extractor": "generate_release_feature_checks.py"}, "constant_implies_variable": True, "checks": selected}
    if args.check:
        if result != json.loads(args.out.read_text(encoding="utf-8")):
            raise ValueError("feature-check artifact drift")
    else:
        args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
