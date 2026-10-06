"""Pinned full-model/validation source oracle for one value-service batch.

Each case has its own live source resource and real library environment.
Only export timestamp metadata is omitted; semantic observations are unchanged.
This tool never awards native support or family qualification.
"""
import argparse, copy, gzip, hashlib, json, os, subprocess, tempfile, uuid
from pathlib import Path
from export_library_reference_inventory import ROOT, PIN, read_bytes

BASE = ROOT / "docs/conformance/2026-08-support"
MANIFEST = BASE / "value-service-reference-manifest.json"
OUTPUT = BASE / "value-service-reference-controls.json.gz"
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotValueServiceReference.java"
EXPORTER = HELPER.with_name("PilotModelExporter.java")
JAR = ROOT.parent / "target/support-2026-08/pilot-pinned-2026-08/org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
RELEASE = ROOT.parent / "target/upstream/SysML-v2-Release"

def digest(data):
    return hashlib.sha256(data).hexdigest()

def validate(doc, manifest):
    expected = {c["id"] for c in manifest["cases"]}
    rows = doc["controls"]
    if len(rows) != len(expected) or {r["id"] for r in rows} != expected:
        raise ValueError("Exact complete reference case inventory required")
    for row in rows:
        status = row["validation_status"]
        if status not in ("accepted", "rejected", "infrastructure_error"):
            raise ValueError("Unknown reference outcome")
        if status == "accepted":
            if row.get("model_status") != "exported" or not row.get("model", {}).get("elements"):
                raise ValueError("Accepted context requires full model export")
        elif row.get("model"):
            raise ValueError("Rejected/failed contexts cannot claim a successful model")
        if status == "accepted":
            model = row["model"]
            names = [e["qualified_name"] for e in model["elements"]]
            if len(names) != len(set(names)) or model["metadata"]["element_count"] != len(names):
                raise ValueError("Canonical full model identities/counts required")
            identities = set(names)
            if any(r["source"] not in identities or r["target"] not in identities for r in model["relationships"]):
                raise ValueError("Full model has missing relationship endpoints")
            if model["metadata"]["relationship_count"] != len(model["relationships"]):
                raise ValueError("Full relationship count mismatch")
            if any(i["severity"] == "ERROR" for i in row["diagnostics"]):
                raise ValueError("Accepted context cannot contain validation errors")
    if doc.get("native_qualification") != "not_assessed":
        raise ValueError("Reference export cannot qualify native behavior")

def comparison_view(doc):
    """Alpha-rename generated identities and checked identity aliases only."""
    result = copy.deepcopy(doc)
    for row in result["controls"]:
        if row["validation_status"] != "accepted": continue
        model = row["model"]
        identities = {}
        for element in model["elements"]:
            identifier = element["properties"].get("element_id")
            if identifier is not None:
                uuid.UUID(identifier)
                if identifier in identities: raise ValueError("Generated model identity is not unique")
                identities[identifier] = element["qualified_name"]
        for element in model["elements"]:
            props = element["properties"]
            for field in ("element_id", "member_element_id", "owned_member_element_id"):
                identifier = props.get(field)
                if identifier not in identities: continue
                target = identities[identifier]
                if field != "element_id":
                    endpoints = [r["target"] for r in model["relationships"]
                                 if r["source"] == element["qualified_name"] and r["relation"] == "member_element"]
                    if endpoints != [target]:
                        raise ValueError("Identity alias disagrees with canonical member endpoint")
                props[field] = {"generated_identity": target}
    return result

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", required=True, type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    old = json.loads((BASE / "parsed-result-valuation-pilot-controls.json").read_text(encoding="utf-8"))
    if old["provenance"]["pilot_commit"] != PIN:
        raise ValueError("Changed Pilot pin")
    for name, expected in old["provenance"]["library_inputs"].items():
        if digest(read_bytes(RELEASE / name)) != expected:
            raise ValueError("Changed pinned library: " + name)
    for c in manifest["cases"]:
        if digest((ROOT / c["path"]).read_bytes()) != c["source_file_sha256"]:
            raise ValueError("Changed frozen source: " + c["id"])
    if digest(JAR.read_bytes()) != old["provenance"]["runtime_sha256"]:
        raise ValueError("Changed pinned runtime")
    suffix = ".exe" if os.name == "nt" else ""
    java, javac = [args.java_bin / (n + suffix) for n in ("java", "javac")]
    with tempfile.TemporaryDirectory(prefix="value-service-reference-") as folder:
        folder = Path(folder)
        spec = dict(manifest)
        spec["cases"] = [dict(c, path=str(ROOT / c["path"])) for c in manifest["cases"]]
        (folder / "spec.json").write_text(json.dumps(spec), encoding="utf-8")
        subprocess.run([str(javac), "-encoding", "UTF-8", "-cp", str(JAR), "-d", str(folder), str(EXPORTER), str(HELPER)], check=True, timeout=60)
        subprocess.run([str(java), "-Xmx4g", "-cp", str(folder) + os.pathsep + str(JAR),
                        "dev.mercurio.pilot.PilotValueServiceReference", str(RELEASE / "sysml.library"),
                        str(folder / "spec.json"), str(folder / "observations.json")], check=True, timeout=300)
        controls = json.loads((folder / "observations.json").read_text(encoding="utf-8"))
    inputs = {p.relative_to(ROOT).as_posix(): digest(p.read_bytes()) for p in (MANIFEST, HELPER, EXPORTER, Path(__file__), BASE / "parsed-result-valuation-pilot-controls.json")}
    doc = {"schema": "dev.mercurio.value-service-reference-controls.v1",
           "scope": "Isolated parsed source full CheckMode.ALL and model observations using the genuine pinned library. Two explicit Ecore mutation controls are separate from source-only controls.",
           "provenance": {"pilot_commit": PIN, "runtime_sha256": digest(JAR.read_bytes()),
                          "library_source_sha256": old["provenance"]["library_inputs"],
                          "java_sha256": digest(java.read_bytes()), "javac_sha256": digest(javac.read_bytes()),
                          "input_sha256": inputs},
           "omitted_volatile_metadata": ["model.metadata.exported_at_utc"],
           "comparison_identity_policy": "Raw Pilot-assigned Element.elementId UUIDs are retained. Fresh-parse comparison alpha-renames generated identities and their memberElementId/ownedMemberElementId aliases to canonical qualified_name, checking aliases against the exported member_element endpoint. No other attribute, reference, ordering or diagnostic is normalized.",
           "native_qualification": "not_assessed", "controls": controls}
    validate(doc, manifest)
    text = json.dumps(doc, indent=2, sort_keys=True, ensure_ascii=False) + "\n"
    if args.check:
        saved = json.loads(gzip.decompress(OUTPUT.read_bytes()))
        if comparison_view(saved) != comparison_view(doc):
            failure = ROOT / "target/value-service-reference-reproducibility-failure.json.gz"
            failure.parent.mkdir(exist_ok=True)
            failure.write_bytes(gzip.compress(text.encode("utf-8"), mtime=0))
            def differences(a, b, path="", remaining=None):
                if remaining is None: remaining = [12]
                if remaining[0] <= 0 or a == b: return
                if isinstance(a, dict) and isinstance(b, dict):
                    for key in sorted(set(a) | set(b)):
                        differences(a.get(key), b.get(key), path + "/" + key, remaining)
                elif isinstance(a, list) and isinstance(b, list) and len(a) == len(b):
                    for i, (left, right) in enumerate(zip(a, b)):
                        differences(left, right, path + "/" + str(i), remaining)
                else:
                    print("Reference difference:", path, repr(a)[:180], "=>", repr(b)[:180])
                    remaining[0] -= 1
            differences(comparison_view(saved), comparison_view(doc))
            raise ValueError("Changed value-service reference observations; raw failing observation retained at " + str(failure))
    else:
        OUTPUT.write_bytes(gzip.compress(text.encode("utf-8"), mtime=0))
    print("Value-service reference:", len(controls), "contexts;",
          sum(r["validation_status"] == "accepted" for r in controls), "accepted;",
          sum(r["validation_status"] == "rejected" for r in controls), "rejected;",
          sum(r["validation_status"] == "infrastructure_error" for r in controls), "infrastructure failures")

if __name__ == "__main__":
    main()
