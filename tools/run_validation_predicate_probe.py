"""Run real pinned Pilot validator predicates on controlled getters, without model loading.

This is a translation oracle for eleven predicate bodies, not an EMF graph-validity,
derived-getter, validator-dispatch, source-diagnostic-span, or conformance test.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PILOT_COMMIT = "692170b71867353b8f90341e61556f49a5beb0e5"
JAR_SHA256 = "b1ad9d64b1f0c75730facf25a5e2856bc9df2bb4bd39476df2fdf5ae68cd9350"
VALIDATOR_SOURCE = "org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend"
SYSML_SOURCE = "org.omg.sysml.xtext/src/org/omg/sysml/xtext/validation/SysMLValidator.xtend"
SYSML_CLASS = "org/omg/sysml/xtext/validation/SysMLValidator.class"
VALIDATOR_CLASS = "org/omg/kerml/xtext/validation/KerMLValidator.class"
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotValidationPredicateProbe.java"
DEFAULT_OUTPUT = ROOT / "docs/conformance/2026-08-support/translated-validator-pilot-controls.json"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path)
    parser.add_argument("--pilot", type=Path, default=ROOT.parent / "target/upstream/SysML-v2-Pilot-Implementation")
    parser.add_argument("--jar", type=Path, default=ROOT.parent / "target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--check", action="store_true", help="Compare with saved evidence without rewriting it")
    args = parser.parse_args()
    actual_commit = subprocess.check_output(["git", "-C", str(args.pilot), "rev-parse", "HEAD"], text=True).strip()
    if actual_commit != PILOT_COMMIT:
        raise ValueError("Unexpected Pilot checkout commit: " + actual_commit)
    if digest(args.jar) != JAR_SHA256:
        raise ValueError("Pilot executable JAR does not match the pinned release")
    sources = {name: args.pilot / name for name in (VALIDATOR_SOURCE, SYSML_SOURCE)}
    for name, source in sources.items():
        pinned_source = subprocess.check_output(["git", "-C", str(args.pilot), "show", PILOT_COMMIT + ":" + name])
        if source.read_bytes().replace(b"\r\n", b"\n") != pinned_source.replace(b"\r\n", b"\n"):
            raise ValueError("Pilot validator source differs from its pinned commit")
    with zipfile.ZipFile(args.jar) as jar:
        validator_class_hashes = {name: hashlib.sha256(jar.read(name)).hexdigest() for name in (VALIDATOR_CLASS, SYSML_CLASS)}
    def fingerprints():
        return {"validators": {name: digest(path) for name, path in sources.items()}, "jar": digest(args.jar), "helper": digest(HELPER), "runner": digest(Path(__file__))}
    frozen = fingerprints()
    suffix = ".exe" if os.name == "nt" else ""
    def executable(name):
        return str(args.java_bin / (name + suffix)) if args.java_bin else name
    support = ROOT.parent / "target/support-2026-08"
    support.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="validation-predicate-probe-", dir=support) as temporary:
        temporary = Path(temporary)
        classes = temporary / "classes"
        classes.mkdir()
        result_path = temporary / "result.json"
        subprocess.run([executable("javac"), "-encoding", "UTF-8", "-cp", str(args.jar), "-d", str(classes), str(HELPER)], check=True, timeout=60)
        subprocess.run([executable("java"), "-cp", str(classes) + os.pathsep + str(args.jar),
            "dev.mercurio.pilot.PilotValidationPredicateProbe", str(result_path)], check=True, timeout=60)
        result = json.loads(result_path.read_text(encoding="utf-8"))
    if frozen != fingerprints():
        raise ValueError("Probe inputs changed during execution")
    cases = result["cases"]
    if len(cases) != 52 or len({case["id"] for case in cases}) != 52:
        raise ValueError("Expected 12 import combinations, eight annotation masks four scalar controls and 28 type-family controls")
    evidence = {
        "schema_version": 1,
        "scope": "Actual unmodified compiled KerMLValidator.checkImport/checkAnnotation and SysML scalar and selected checkOneType caller/helper predicates with controlled getter/type-list values; the type-family cases use real model classes, a controlled FeatureAdapter.getAllTypes, and explicit Xtext currentObject. No library bootstrap or source/model loading. This does not assess getter implementations, validity of EMF model combinations, @Check dispatch, source spans, or whole-model conformance.",
        "diagnostic_representation": {
            "ordering": "Preserve the order of ValidationMessageAcceptor calls within each direct method invocation",
            "severity": "error, warning, or info from the corresponding acceptor callback",
            "subject": "Stable controlled-object identifier",
            "feature": "ContainingClass::featureName when present; otherwise null",
            "index": "Raw Pilot index, including -1 (INSIGNIFICANT_INDEX)",
            "data": "Raw varargs array, preserving null versus empty",
            "unsupported_callbacks": "Offset/length diagnostic overloads fail the probe rather than lose location information",
        },
        "model_defaults": {"EnumerationDefinition.isVariation": True},
        "annotation_mask_bits": {"0": "owned_annotating_element_present", "1": "owning_annotating_element_present", "2": "owning_annotated_element_present"},
        "provenance": {
            "pilot_commit": PILOT_COMMIT,
            "source_sha256": frozen["validators"],
            "jar_sha256": frozen["jar"],
            "validator_class_sha256": validator_class_hashes,
            "helper_sha256": frozen["helper"],
            "runner_sha256": frozen["runner"],
        },
        "cases": cases,
    }
    content = json.dumps(evidence, indent=2, ensure_ascii=False) + "\n"
    if args.check:
        if not args.output.exists() or args.output.read_text(encoding="utf-8") != content:
            raise ValueError("Saved predicate controls differ from this deterministic run")
        print("Verified 52 deterministic Pilot predicate controls against saved evidence")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(content, encoding="utf-8", newline="\n")
        print("Recorded 52 Pilot predicate controls: " + str(args.output))


if __name__ == "__main__":
    main()
