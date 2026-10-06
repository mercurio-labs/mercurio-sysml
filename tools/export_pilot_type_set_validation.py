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
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotTypeSetValidationProbe.java"
OUTPUT = ROOT / "docs/conformance/2026-08-support/type-set-validation-pilot-controls.json"


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def validate(cases,boundaries):
    expected={(k,r,c,p) for k in ("Unioning","Intersecting","Differencing") for r in ("Feature","Connector","BindingConnector") for c in range(4) for p in ("distinct","duplicate","self")}
    if len(cases)!=108 or {(r["relation"],r["receiver_kind"],r["count"],r["pattern"]) for r in cases}!=expected:
        raise ValueError("Incomplete type-set validation/projection matrix")
    for row in cases:
        kind,count,pattern=row["relation"],row["count"],row["pattern"]
        expected_codes=[]
        if count==1:expected_codes.append("validateOwned"+kind+"NotOne")
        if count>0 and pattern=="self":expected_codes.append("validateType"+kind+"TypesNotSelf")
        actual=[d["code"] for d in row["diagnostics"]]
        if sorted(actual)!=sorted(expected_codes):raise ValueError("Unexpected independent constraint result: "+str(row))
        if row["owned"]!=["relation"+str(i) for i in range(count)]:raise ValueError("Changed owned projection")
        targets=["receiver" if pattern=="self" and i+1==count else "target0" if pattern=="duplicate" else "target"+str(i) for i in range(count)]
        # Preserve the independently observed ordered uniqueness, with relationship
        # cardinality separate from endpoint deduplication.
        if row["endpoints"]!=list(dict.fromkeys(targets)):raise ValueError("Changed endpoint projection")
    if len(boundaries)!=3 or {r["mode"] for r in boundaries}!={"canonical","foreign_source","package_owner"}:
        raise ValueError("Incomplete Disjoining source/owner controls")
    for row in boundaries:
        mode=row["mode"]
        if row["owned_count"]!=int(mode=="canonical") or row["owning_is_receiver"]!=(mode!="package_owner") or row["owning_present"]!=(mode!="package_owner") or row["stored_source_is_receiver"]!=(mode!="foreign_source"):
            raise ValueError("Changed Disjoining role projection")


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
                     "delegate/invocation/Feature_typingFeatures_InvocationDelegate.java", "util/FeatureUtil.java",
                     "delegate/setting/DefaultDerivedPropertySettingDelegate.java",
                     "delegate/setting/Type_ownedDisjoining_SettingDelegate.java",
                     "delegate/setting/Disjoining_owningType_SettingDelegate.java",
                     "delegate/setting/Type_unioningType_SettingDelegate.java",
                     "delegate/setting/Type_intersectingType_SettingDelegate.java",
                     "delegate/setting/Type_differencingType_SettingDelegate.java"):
            relative = "org.omg.sysml.logic/src/main/java/org/omg/sysml/" + name
            data = (args.pilot / relative).read_bytes().replace(b"\r\n", b"\n")
            if data != git("show", PIN + ":" + relative).replace(b"\r\n", b"\n"):
                raise ValueError("Changed pinned source: " + relative)
            sources[relative] = hashlib.sha256(data).hexdigest()
        validator_source="org.omg.kerml.xtext/src/org/omg/kerml/xtext/validation/KerMLValidator.xtend"
        validator_data=(args.pilot/validator_source).read_bytes().replace(b"\r\n",b"\n")
        if validator_data!=git("show",PIN+":"+validator_source).replace(b"\r\n",b"\n"):
            raise ValueError("Changed pinned Type validator source")
        sources[validator_source]=hashlib.sha256(validator_data).hexdigest()
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
                        "dev.mercurio.pilot.PilotTypeSetValidationProbe", str(result)], check=True, timeout=60)
        payload = json.loads(result.read_text(encoding="utf-8"))
        validate(payload["cases"], payload["boundaries"])
        if any(digest(p) != sha for p, sha in initial.items()):
            raise ValueError("Probe inputs changed")
    output = {"schema": "dev.mercurio.type-set-validation-reference.v1", "qualification_certificate": False,
              "scope": "108 supplied-completed predicate/projection contexts and three Disjoining source/owner controls; no cold lifecycle or whole-model qualification.",
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
    print("Pinned Type-set validation reference: 108 predicate/projection contexts, three Disjoining boundaries; no qualification.")


if __name__ == "__main__":
    main()
