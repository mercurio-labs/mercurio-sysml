"""Reuse only unchanged Pilot semantic exports after verifying source and oracle provenance."""
import argparse
import gzip
import hashlib
import json
import shutil
from pathlib import Path


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--dest", type=Path, required=True)
    args = parser.parse_args()
    source, dest = args.source, args.dest
    old, new = read(source / "source-lock.json"), read(dest / "source-lock.json")
    for key in ("pilot_commit", "jar_sha256", "manifest_sha256", "sources", "library_index_sha256"):
        if old[key] != new[key]:
            raise ValueError("Pilot input mismatch: " + key)
    if not old["pilot_clean"] or not new["pilot_clean"]:
        raise ValueError("unclean Pilot input")
    helper = Path("tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java")
    if digest(helper) != read(source / "build-provenance.json")["java_source_sha256"]:
        raise ValueError("oracle Java source changed")
    classes = {}
    for original in (source / "classes").rglob("*.class"):
        relative = original.relative_to(source / "classes")
        target = dest / "classes" / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(original, target)
        if digest(target) != digest(original):
            raise ValueError("class copy mismatch: " + str(relative))
        classes[relative.as_posix()] = digest(target)
    if classes.get("dev/mercurio/pilot/PilotModelExporter.class") != old["oracle_sha256"]:
        raise ValueError("oracle class fingerprint mismatch")
    groups = []
    for group in new["groups"]:
        previous, current = source / "groups" / group["id"], dest / "groups" / group["id"]
        if read(previous / "spec.json") != read(current / "spec.json"):
            raise ValueError("source-set specification mismatch: " + group["id"])
        archive = read(previous / "export-archive.json")
        data = gzip.decompress((previous / "pilot-export.json.gz").read_bytes())
        export_hash = hashlib.sha256(data).hexdigest()
        if export_hash != archive["uncompressed_sha256"]:
            raise ValueError("export archive mismatch: " + group["id"])
        output = current / "pilot-export.json"
        if output.exists() and digest(output) != export_hash:
            raise ValueError("refusing to overwrite a different oracle export")
        output.write_bytes(data)
        shutil.copyfile(previous / "pilot-semantic-timings.json", current / "pilot-semantic-timings.json")
        groups.append({"id": group["id"], "export_sha256": export_hash, "spec_sha256": digest(current / "spec.json"), "semantic_timings_sha256": digest(current / "pilot-semantic-timings.json")})
    provenance = {"source": str(source.resolve()), "dest": str(dest.resolve()), "java_source_sha256": digest(helper), "class_sha256": classes, "pilot_commit": new["pilot_commit"], "jar_sha256": new["jar_sha256"], "source_stdlib_sha256": old["stdlib_sha256"], "candidate_stdlib_sha256": new["stdlib_sha256"], "groups": groups, "scope": "Unchanged Pilot semantic exports only. Native compilation/comparison and both engines' timing trials must run freshly."}
    (dest / "oracle-reuse-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8")
    print(f"Verified and reused {len(groups)} Pilot source groups and {len(classes)} helper classes")


if __name__ == "__main__":
    main()
