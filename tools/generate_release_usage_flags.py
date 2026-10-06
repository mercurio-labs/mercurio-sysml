"""Extract Pilot variability anchors and non-composite usage families."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("pilot-root", "metamodel", "out"):
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    def git(*command):
        return subprocess.check_output(["git", "-C", str(args.pilot_root), *command], text=True).strip()
    if git("status", "--porcelain"):
        raise ValueError("Pilot checkout must be clean")
    adapter = "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/UsageAdapter.java"
    processor = "org.omg.sysml.xtext/src/org/omg/sysml/xtext/postprocessing/UsageParserPostProcessor.java"
    ports = "org.omg.sysml.xtext/src/org/omg/sysml/xtext/postprocessing/PortUsageParserPostProcessor.java"
    delegate = "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Usage_mayTimeVary_SettingDelegate.java"
    delegate_text = (args.pilot_root / delegate).read_text(encoding="utf-8")
    match = re.search(r"return !\((.*?)\) &&\s*UsageUtil.mayTimeVary\(\(Usage\)owner\);", delegate_text, re.S)
    if match is None:
        raise ValueError("unsupported Usage variability setting delegate")
    excluded_contexts = re.findall(r"owner instanceof (\w+)", match[1])
    if re.sub(r"\s+", "", match[1]) != "||".join("ownerinstanceof" + name for name in excluded_contexts):
        raise ValueError("unsupported variability exclusion predicate")
    text = (args.pilot_root / adapter).read_text(encoding="utf-8")
    formula = re.search(r"mayTimeVary = owningType != null &&(.*?);", text, re.S)
    if formula is None:
        raise ValueError("missing variability formula")
    anchors = re.findall(r'getLibraryElement\(target, "([^"\n]+)"\)', formula[1])
    shape = re.sub(r'"[^"\n]+"', '"TYPE"', formula[1])
    shape = re.sub(r"\s+", "", shape)
    expected = 'TypeUtil.specializes(owningType,(Type)SysMLLibraryUtil.getLibraryElement(target,"TYPE"))&&!(target.isPortion()||TypeUtil.specializes(target,(Type)SysMLLibraryUtil.getLibraryElement(target,"TYPE"))||TypeUtil.specializes(target,(Type)SysMLLibraryUtil.getLibraryElement(target,"TYPE"))||target.isComposite()&&TypeUtil.specializes(target,(Type)SysMLLibraryUtil.getLibraryElement(target,"TYPE")))'
    if shape != expected or len(anchors) != 4:
        raise ValueError("unsupported variability formula")
    post = (args.pilot_root / processor).read_text(encoding="utf-8")
    match = re.search(r"NON_COMPOSITE_USAGE_TYPES = List.of\((.*?)\);", post, re.S)
    if match is None:
        raise ValueError("missing non-composite usage families")
    literals = re.findall(r"SysMLPackage.Literals.(\w+)", match[1])
    model = json.loads(args.metamodel.read_text(encoding="utf-8"))
    names = {re.sub(r"(?<!^)(?=[A-Z])", "_", c["name"]).upper(): c["name"] for c in model["metaclasses"]}
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    result = {"schema_version": 1, "source": {"pilot_commit": git("rev-parse", "HEAD"), "pilot_dirty": False, "files": {path: digest(args.pilot_root / path) for path in (adapter, processor, ports, delegate)}, "metamodel_sha256": digest(args.metamodel), "extractor": "generate_release_usage_flags.py"}, "non_variable_contexts": excluded_contexts, "owner_type": anchors[0], "excluded_types": anchors[1:3], "composite_excluded_type": anchors[3], "non_composite_contexts": [names[name] for name in literals]}
    if args.check:
        if result != json.loads(args.out.read_text(encoding="utf-8")):
            raise ValueError("usage flag artifact drift")
    else:
        args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
