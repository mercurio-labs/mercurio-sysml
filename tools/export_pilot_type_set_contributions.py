"""Cache bounded pinned Pilot type-set getter observations; no qualification."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PIN = "692170b71867353b8f90341e61556f49a5beb0e5"
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotTypeSetContributionProbe.java"
OUTPUT = ROOT / "docs/conformance/2026-08-support/type-set-contribution-pilot-controls.json"


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def validate(cases, boundaries):
    expected = {(r, k, m, o, c, p, t) for r in ("Disjoining", "Unioning", "Intersecting", "Differencing")
                for k in ("Feature", "Connector", "BindingConnector") for m in (0, 1, 2, 4)
                for o in ("Package", "Class", "Structure") for c in (False, True) for p in (False, True)
                for t in ("Class", "Structure", "DataType", "Feature")}
    actual = {(r["relation"], r["receiver_kind"], r["typing_mask"], r["owner_kind"], r["composite"], r["portion"], r["target_kind"])
              for r in cases}
    if len(cases) != 2304 or actual != expected:
        raise ValueError("Incomplete type-set contribution matrix")
    for row in cases:
        baseline, observation = row["baseline"], row["observation"]
        for field in ("feature_property_types", "adapter_all_types", "general_types", "default_supertype"):
            if baseline[field] != observation[field]:
                raise ValueError("Type-set contribution changes bounded getter: " + str(row))
        if not (observation["source_is_receiver"] and observation["endpoint_present"]
                and observation["endpoint_kind"] == row["target_kind"] and observation["owned_subtree_count"] == 0
                and baseline["receiver_completed"] and observation["receiver_completed"]):
            raise ValueError("Unexpected bounded endpoint/lifecycle fixture")
    expected_boundaries = {(r, b) for r in ("Disjoining", "Unioning", "Intersecting", "Differencing")
                           for b in ("missing", "wrong_kind")}
    if len(boundaries) != 8 or {(r["relation"], r["boundary"]) for r in boundaries} != expected_boundaries:
        raise ValueError("Incomplete boundary inventory")
    for row in boundaries:
        observation = row["observation"]
        if row["boundary"] == "missing":
            if observation["endpoint_present"] or not observation["source_is_receiver"]:
                raise ValueError("Missing endpoint was silently supplied")
        elif observation["endpoint_assignment"] == "accepted":
            raise ValueError("Wrong endpoint kind was accepted")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--pilot", type=Path, default=ROOT.parent / "target/support-2026-08/pilot-pinned-2026-08")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="type-set-probe-") as temporary:
        temporary = Path(temporary)
        config = temporary / "gitconfig"
        config.write_text("[safe]\n\tdirectory = " + args.pilot.resolve().as_posix() + "\n", encoding="utf-8")
        env = dict(os.environ, GIT_CONFIG_GLOBAL=str(config))
        def git(*command):
            return subprocess.check_output(["git", "-C", str(args.pilot), *command], env=env)
        if git("rev-parse", "HEAD").decode().strip() != PIN:
            raise ValueError("Unexpected Pilot revision")
        sources = {}
        for name in ("adapter/FeatureAdapter.java", "adapter/TypeAdapter.java", "util/TypeUtil.java",
                     "delegate/invocation/Feature_typingFeatures_InvocationDelegate.java", "util/FeatureUtil.java"):
            relative = "org.omg.sysml.logic/src/main/java/org/omg/sysml/" + name
            data = (args.pilot / relative).read_bytes().replace(b"\r\n", b"\n")
            if data != git("show", PIN + ":" + relative).replace(b"\r\n", b"\n"):
                raise ValueError("Changed pinned source: " + relative)
            sources[relative] = hashlib.sha256(data).hexdigest()
        connector_impl = "org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/ConnectorImpl.java"
        impl_data = (args.pilot / connector_impl).read_bytes().replace(b"\r\n", b"\n")
        if impl_data != git("show", PIN + ":" + connector_impl).replace(b"\r\n", b"\n"):
            raise ValueError("Changed pinned Connector property implementation")
        sources[connector_impl] = hashlib.sha256(impl_data).hexdigest()
        jar = args.pilot / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
        imported = json.loads((ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/feature-defaults.extract.json").read_text(encoding="utf-8"))
        if imported["provenance"]["runtime_sha256"] != digest(jar):
            raise ValueError("Reference runtime differs from the imported Feature program runtime")
        initial = {str(p): digest(p) for p in (jar, HELPER, Path(__file__))}
        suffix = ".exe" if os.name == "nt" else ""
        version = subprocess.run([str(args.java_bin / ("java" + suffix)), "-version"], capture_output=True, text=True, check=True)
        subprocess.run([str(args.java_bin / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", str(jar),
                        "-d", str(temporary), str(HELPER)], check=True, timeout=60)
        result = temporary / "result.json"
        subprocess.run([str(args.java_bin / ("java" + suffix)), "-cp", str(temporary) + os.pathsep + str(jar),
                        "dev.mercurio.pilot.PilotTypeSetContributionProbe", str(result)], check=True, timeout=60)
        payload = json.loads(result.read_text(encoding="utf-8"))
        validate(payload["cases"], payload["boundaries"])
        if any(digest(p) != sha for p, sha in initial.items()):
            raise ValueError("Probe inputs changed")
    output = {"schema": "dev.mercurio.type-set-contribution-reference.v1", "qualification_certificate": False,
              "scope": "2304 bounded supplied-completed Ecore getter contexts and eight endpoint boundary observations. "
                       "Independent direct typing/general/default effects and reciprocal derived endpoints only. "
                       "Cold lifecycle, inheritance contexts, full source environments and constraint validation unqualified.",
              "provenance": {"pilot_commit": PIN, "source_sha256": sources, "jar_sha256": digest(jar),
                             "helper_sha256": digest(HELPER), "driver_sha256": digest(Path(__file__)),
                             "java_version": version.stdout + version.stderr},
              **payload}
    text = json.dumps(output, indent=2, sort_keys=True) + "\n"
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != text:
            raise ValueError("Stale type-set contribution observations")
    else:
        OUTPUT.write_text(text, encoding="utf-8", newline="\n")
    print("Pinned type-set contribution reference: 2304 getter contexts, eight endpoint boundaries; no qualification.")


if __name__ == "__main__":
    main()
