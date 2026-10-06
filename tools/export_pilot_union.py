"""Observe actual pinned ordered union getter with supplied subset inputs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
PILOT = ROOT.parent / "target/support-2026-08/pilot-pinned-2026-08"
PIN = "692170b71867353b8f90341e61556f49a5beb0e5"
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotUnionProbe.java"
OUTPUT = PROFILE / "union-pilot-controls.json"

def digest(data):
    return hashlib.sha256(data).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    revision = subprocess.check_output(["git", "-C", str(PILOT), "rev-parse", "HEAD"], text=True).strip()
    if revision != PIN:
        raise ValueError("Changed Pilot revision")
    runtime = PILOT / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    reference = json.loads((PROFILE / "xtext-prediction-nfa.experimental.json").read_text(encoding="utf-8"))
    if digest(runtime.read_bytes()) != reference["provenance"]["runtime_sha256"]:
        raise ValueError("Changed pinned runtime")
    suffix = ".exe" if os.name == "nt" else ""
    with tempfile.TemporaryDirectory(prefix="union-") as temp:
        raw = Path(temp) / "raw.json"
        subprocess.run([str(args.java_bin / ("javac"+suffix)), "-encoding", "UTF-8", "-cp", str(runtime), "-d", temp, str(HELPER)], check=True, timeout=60)
        subprocess.run([str(args.java_bin / ("java"+suffix)), "-cp", temp+os.pathsep+str(runtime), "dev.mercurio.pilot.PilotUnionProbe", str(raw)], check=True, timeout=60)
        doc = json.loads(raw.read_text(encoding="utf-8"))
    paths = {'org.omg.sysml/model/SysML.ecore', 'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/NamespaceImpl.java', 'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/TypeImpl.java', 'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/util/DerivedUnionEObjectEList.java', 'org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/util/DerivedEObjectEList.java'}
    paths.update({
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/TypeAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/FeatureAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/FeatureUtil.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/TypeUtil.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/NamespaceAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/NamespaceImportAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/MembershipImportAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Namespace_ownedMembership_SettingDelegate.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Namespace_importedMembership_SettingDelegate.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/invocation/Namespace_importedMemberships_InvocationDelegate.java",
    })
    paths.update({
        "org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/FeatureImpl.java",
        "org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/FeatureChainingImpl.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/invocation/Feature_supertypes_InvocationDelegate.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/FeatureChaining_featureChained_SettingDelegate.java",
    })
    hashes = {}
    for path in sorted(paths):
        content = (PILOT/path).read_bytes().replace(b"\r\n",b"\n")
        pinned = subprocess.check_output(["git","-C",str(PILOT),"show",PIN+":"+path]).replace(b"\r\n",b"\n")
        if content != pinned: raise ValueError("Changed source: "+path)
        hashes[path] = digest(content)
    doc = {"schema_version":1, "scope":"Actual generated Namespace/Type union getters over supplied subset lists and their matching EMF eIsSet state. Composition controls supply dependencies. Separate canonical_controls use actual factory-created ownership/import graphs and runtime-derived package unions. Typed-import controls explicitly supply Pilot-derived inheritedMembership snapshots; Seven recursive typed controls derive non-Feature inheritance from stored specializations/conjugation without inherited snapshots. A separate implied-specialization disagreement records Pilot behavior versus the normative exclusion rule. Nine further controls provide complete materialized implied relationships and verify full inheritance, Feature redefinition filtering and complete unnamed/derived-name Features without inherited snapshots; fixture completeness is checked against Pilot computed redefinitions and implicit generals. A separate shared-Feature-alias disagreement compares the normative membership-based rejection predicate with Pilot target-based filtering. Nine further canonical Feature-chaining controls exercise ordered final-target inheritance, explicit parents, recursive imports, cycles, duplicate parents, visibility, PartUsage dispatch and conjugation. Computing missing implicit relationships, chain legality and filtered/external scopes remain unqualified.", "chaining_controls":doc["chaining"], "controls":doc["composition"], "canonical_controls":doc["canonical"], "implied_disagreement_control":doc["implied_disagreement"], "redefinition_alias_disagreement_control":doc["redefinition_alias_disagreement"],
           "provenance":{"pilot_revision":PIN,"runtime_sha256":digest(runtime.read_bytes()),"ecore_sha256":digest((PROFILE/"ecore-effective.extract.json").read_bytes()),"source_sha256":hashes,"helper_sha256":digest(HELPER.read_bytes()),"driver_sha256":digest(Path(__file__).read_bytes())}}
    rendered = json.dumps(doc,indent=2,sort_keys=True)+"\n"
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != rendered: raise ValueError("Stale union controls")
    else: OUTPUT.write_text(rendered,encoding="utf-8",newline="\n")
    print(len(doc["controls"]),"composition controls and",len(doc["canonical_controls"]),"canonical graph controls and",len(doc["chaining_controls"]),"chaining controls observed")

if __name__ == "__main__": main()
