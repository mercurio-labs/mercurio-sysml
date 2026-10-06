"""Inventory all SysML/KerML textual validators with the actual Xtend frontend.

Resolution failures remain explicit inventory rows. This does not translate
unselected methods or assert completeness of handwritten native semantics.
"""
from __future__ import annotations
import argparse
from collections import Counter
import json
import os
from pathlib import Path
import subprocess
from export_pilot_xtend import ROOT, PILOT_REVISION, JAR_SHA256, XTEND_VERSION, XTEND_SHA256, clean_revision, digest, encode

PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotValidatorInventory.java"
PROJECTS = {"org.omg.kerml.xtext", "org.omg.sysml.xtext", "org.omg.kerml.expressions.xtext"}


def sources(root):
    discovered = sorted(p.relative_to(root).as_posix() for p in root.rglob("*Validator.xtend"))
    selected = [p for p in discovered if p.split("/")[0] in PROJECTS]
    if {p.split("/")[0] for p in selected} != PROJECTS:
        raise ValueError("Incomplete SysML/KerML validator source projects")
    return selected, [{"path": p, "reason": "Outside the SysML/KerML textual frontend projects"} for p in discovered if p not in selected]


def validate(document):
    if document.get("schema_version") != 1 or document.get("source_format") != "resolved-xtend-validator-inventory" or document.get("parser") != "org.eclipse.xtend.core.XtendStandaloneSetup":
        raise ValueError("Unexpected upstream inventory schema/frontend")
    if document.get("span_encoding") != "utf-16-code-units":
        raise ValueError("Unexpected span encoding")
    methods = document.get("methods")
    files = document.get("files")
    if not isinstance(methods, list) or not methods or not isinstance(files, list):
        raise ValueError("Missing method/file inventory")
    names = [m["id"] for m in methods]
    if len(set(names)) != len(names):
        raise ValueError("Duplicated method identities")
    file_names = [f["source"] for f in files]
    if len(set(file_names)) != len(file_names):
        raise ValueError("Duplicated source inventory")
    for method in methods:
        if method["source"] not in file_names or type(method["is_check"]) is not bool:
            raise ValueError("Invalid method source/check classification")
        if method["is_check"] != ("org.eclipse.xtext.validation.Check" in method["annotations"]):
            raise ValueError("Check classification differs from resolved annotation")
        if method["resolution_status"] != ("incomplete" if method["resolution_issues"] else "resolved"):
            raise ValueError("Resolution status conceals issues")
        span = method["span"]
        if span["offset"] < 0 or span["length"] <= 0 or span["start_line"] < 1 or span["end_line"] < span["start_line"]:
            raise ValueError("Invalid method span")
        call_kinds = {"XFeatureCall", "XMemberFeatureCall", "XBinaryOperation", "XUnaryOperation", "XPostfixOperation", "XAssignment", "XConstructorCall"}
        if Counter(c["node_kind"] for c in method["calls"]) != Counter({k: v for k, v in method["body_node_kinds"].items() if k in call_kinds}):
            raise ValueError("Call inventory omits or duplicates body call nodes")
        for call in method["calls"]:
            if type(call.get("resolved")) is not bool or (call["resolved"] and "target" not in call):
                raise ValueError("Missing call resolution evidence")
            if not call["resolved"] and not method["resolution_issues"]:
                raise ValueError("Unresolved call concealed by method status")
    for file in files:
        subset = [m for m in methods if m["source"] == file["source"]]
        if len(subset) != file["methods"] or sum(m["is_check"] for m in subset) != file["checks"]:
            raise ValueError("Method/check counts do not cover source file")
    return {"files": len(files), "methods": len(methods), "checks": sum(m["is_check"] for m in methods),
            "check_resolution": dict(sorted(Counter(m["resolution_status"] for m in methods if m["is_check"]).items()))}


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--pilot-root", type=Path, default=ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation")
    p.add_argument("--jar", type=Path, default=ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar")
    p.add_argument("--xtend-jar", type=Path, default=Path.home() / f".m2/repository/org/eclipse/xtend/org.eclipse.xtend.core/{XTEND_VERSION}/org.eclipse.xtend.core-{XTEND_VERSION}.jar")
    p.add_argument("--java-bin", type=Path, required=True)
    p.add_argument("--work-dir", type=Path, default=ROOT.parent / "target/support-2026-08/validator-inventory")
    p.add_argument("--out", type=Path, default=PROFILE / "validators.inventory.extract.json")
    p.add_argument("--check", action="store_true")
    a = p.parse_args(argv)
    clean_revision(a.pilot_root)
    if digest(a.jar) != JAR_SHA256 or digest(a.xtend_jar) != XTEND_SHA256:
        raise ValueError("Wrong pinned Pilot or Xtend JAR")
    selected, excluded = sources(a.pilot_root)
    hashes = {s: digest(a.pilot_root / s) for s in selected}
    helpers = {HELPER.name: digest(HELPER), "export_validator_inventory.py": digest(Path(__file__)),
               "export_pilot_xtend.py": digest(ROOT / "tools/export_pilot_xtend.py")}
    a.work_dir.mkdir(parents=True, exist_ok=True)
    inputs = a.work_dir / "sources.json"
    inputs.write_bytes(encode(selected))
    classes = a.work_dir / "classes"
    classes.mkdir(exist_ok=True)
    cp = os.pathsep.join(map(str, [a.jar, a.xtend_jar]))
    suffix = ".exe" if os.name == "nt" else ""
    subprocess.run([str(a.java_bin / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", cp, "-d", str(classes), str(HELPER)], check=True)
    raw = a.work_dir / "inventory.raw.json"
    subprocess.run([str(a.java_bin / ("java" + suffix)), "-Xmx2g", "-cp", str(classes) + os.pathsep + cp,
                    "dev.mercurio.pilot.PilotValidatorInventory", str(a.pilot_root), str(inputs), str(raw)], check=True)
    document = json.loads(raw.read_text(encoding="utf-8"))
    if sorted(f["source"] for f in document["files"]) != selected:
        raise ValueError("Exporter omitted source files")
    document["summary"] = validate(document)
    clean_revision(a.pilot_root)
    if (sources(a.pilot_root) != (selected, excluded) or hashes != {s: digest(a.pilot_root / s) for s in selected}
        or helpers != {HELPER.name: digest(HELPER), "export_validator_inventory.py": digest(Path(__file__)), "export_pilot_xtend.py": digest(ROOT / "tools/export_pilot_xtend.py")}
        or digest(a.jar) != JAR_SHA256 or digest(a.xtend_jar) != XTEND_SHA256):
        raise ValueError("Inventory inputs changed during extraction")
    document["excluded_sources"] = excluded
    document["provenance"] = {"pilot_revision": PILOT_REVISION, "jar_sha256": JAR_SHA256,
                              "xtend_version": XTEND_VERSION, "xtend_jar_sha256": XTEND_SHA256,
                              "sources_sha256": hashes, "tools_sha256": helpers}
    output = encode(document)
    if a.check:
        if not a.out.is_file() or a.out.read_bytes() != output:
            raise ValueError("Stale resolved validator inventory")
    else:
        a.out.parent.mkdir(parents=True, exist_ok=True)
        a.out.write_bytes(output)
    print(json.dumps(document["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
