"""Extract Pilot connection-definition base/binary defaults and their selection rule."""
import argparse, hashlib, json, re, subprocess
from pathlib import Path

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--pilot-root", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    def git(*args):
        return subprocess.check_output(["git", "-C", str(a.pilot_root), *args], text=True).strip()
    if git("status", "--porcelain"):
        raise ValueError("Pilot checkout must be clean")
    root = "org.omg.sysml.logic/src/main/java/org/omg/sysml/"
    paths = [root + "util/ImplicitGeneralizationMap.java", root + "adapter/ConnectionDefinitionAdapter.java", root + "adapter/FlowDefinitionAdapter.java"]
    paths.extend([root + "adapter/AssociationAdapter.java", root + "adapter/AssociationStructureAdapter.java"])
    table, adapter, flow_adapter, association, structure = [(a.pilot_root / p).read_text(encoding="utf-8") for p in paths]
    shape = re.sub(r"\s+", "", adapter)
    expected = 'returngetTarget().getOwnedEndFeature().size()!=2?getDefaultSupertype("base"):getDefaultSupertype("binary");'
    if expected not in shape or expected not in re.sub(r"\s+", "", flow_adapter):
        raise ValueError("Unsupported connection default selection rule")
    if expected not in re.sub(r"\s+", "", association) or "extends AssociationAdapter" not in structure:
        raise ValueError("Unsupported association default rule")
    defaults = {}
    for kind in ["ConnectionDefinition", "InterfaceDefinition", "AllocationDefinition", "FlowDefinition", "Association", "AssociationStructure"]:
        defaults[kind] = {}
        for selector in ["base", "binary"]:
            values = re.findall(r'put\(' + kind + r'Impl.class,\s*"' + selector + r'",\s*"([^"\n]+)"\);', table)
            if len(values) != 1:
                raise ValueError(f"Expected one {kind}.{selector} mapping")
            defaults[kind][selector] = values[0]
    sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
    result = {"schema_version": 1, "source": {"pilot_commit": git("rev-parse", "HEAD"), "pilot_dirty": False, "files": {p: sha(a.pilot_root / p) for p in paths}, "extractor": Path(__file__).name}, "definitions": defaults}
    if a.check:
        if result != json.loads(a.out.read_text(encoding="utf-8")):
            raise ValueError("Connection default extraction drift")
    else:
        a.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"{len(defaults)} conditional connection defaults verified")

if __name__ == "__main__":
    main()
