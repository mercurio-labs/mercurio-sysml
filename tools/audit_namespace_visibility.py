"""Verify the isolated wildcard visibility controls; no full comparison or benchmark."""
import argparse
import json
import os
import subprocess
from run_namespace_batch import ROOT, read, write, digest
from run_declaration_milestone import sources

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--suite', choices=['namespace-visibility', 'qualified-visibility', 'inherited-visibility', 'inherited-types', 'import-visibility', 'import-feature-visibility'], default='namespace-visibility')
    suite = parser.parse_args().suite
    before = sources()
    support = ROOT.parent / "target/support-2026-08"
    folder = support / (suite + "-controls")
    evidence = ROOT / "docs/conformance/2026-08-support"
    oracle = read(evidence / (suite + "-pilot-controls.json"))
    helper = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java"
    jar = ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar"
    if oracle["helper_sha256"] != digest(helper) or oracle["jar_sha256"] != digest(jar): raise ValueError("Pilot provenance drift")
    fixtures = ROOT / "crates/mercurio-tools/corpus/release-2026-08" / suite
    for name, expected in oracle["source_sha256"].items():
        if digest(fixtures / name) != expected: raise ValueError("Fixture drift")
    binary = ROOT / "target/debug/audit_release_compile.exe"
    library = support / "stdlib.inherited-types.kir.json"
    env = dict(os.environ,MERCURIO_STDLIB_PATH=str(library),MERCURIO_KERNEL_LIBRARY_PATH=str(library))
    output = folder / "native-controls.jsonl"
    with (folder / "native-controls.log").open("w",encoding="utf-8") as log:
        result = subprocess.run([str(binary),str(folder / "spec.json"),str(output)],env=env,stdout=log,stderr=subprocess.STDOUT)
    rows = [json.loads(line) for line in output.read_text(encoding="utf-8").splitlines()]
    expected = {row["relative_path"]:row["status"] for row in oracle["cases"]}
    if result.returncode != 1 or len(rows) != 8 or {row["relative_path"]:row["status"] for row in rows} != expected: raise ValueError("Visibility acceptance differs")
    for row in rows:
        if row["status"] == "error":
            name = ("Visible" if suite in ("import-visibility", "import-feature-visibility") and ("reexport" in row["relative_path"] or "qualified" in row["relative_path"] or "sibling" in row["relative_path"])
                    else "Secret" if suite == "namespace-visibility" and "private-alias" in row["relative_path"] else "Hidden")
            if name not in json.dumps(row): raise ValueError("Expected inaccessible name diagnostic")
    if sources() != before: raise ValueError("Source drift")
    write(evidence / ("namespace-wildcard-visibility-evidence.json" if suite == "namespace-visibility" else suite + "-evidence.json"),{"scope":("Eight imported-feature controls: private imports cannot leak through legacy feature-reference fallback; public aliases to private features, local private imports, import-all and public re-exports bind their actual targets. Library and full linking remain open." if suite == "import-feature-visibility" else "Eight local import controls: explicit visibility, import-all private access, alias target visibility, private/public re-export and qualified import access. Library membership visibility and full namespace linking remain open." if suite == "import-visibility" else "Eight inherited local-type controls: protected typing and specialization, inherited public aliases and private/qualified protected rejection. Library/import visibility and complete scope/linking remain open." if suite == "inherited-types" else "Eight inherited local-member controls: protected features/aliases, private member exclusion, public aliases to private features and qualified protected rejection. Library/import visibility and complete scope/linking remain open." if suite == "inherited-visibility" else "Eight qualified local-membership controls: private/protected types, private prefixes and aliases, lexical private access and public aliases. Inherited/imported/library member visibility and full scope/linking remain open." if suite == "qualified-visibility" else "Eight wildcard visibility controls: public aliases, import all, private aliases and recursive private namespaces; full scope/linking remains open."),"compiler_source_sha256":before,"source_sha256":oracle["source_sha256"],"helper_sha256":digest(helper),"jar_sha256":digest(jar),"binary_sha256":digest(binary),"library_sha256":digest(library),"controls":rows})
    print("Verified eight isolated " + suite + " controls")

if __name__ == "__main__": main()
