"""Export bounded independent Pilot whole-document parse, construction and links."""
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
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotDefinitionDocumentProbe.java"
OUTPUT = PROFILE / "definition-document-pilot-controls.json"


def digest(content):
    return hashlib.sha256(content).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    revision = subprocess.check_output(["git", "-C", str(PILOT), "rev-parse", "HEAD"], text=True).strip()
    if revision != PIN:
        raise ValueError("Changed Pilot revision")
    reference = json.loads((PROFILE / "xtext-prediction-nfa.experimental.json").read_text(encoding="utf-8"))
    runtime = PILOT / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    if digest(runtime.read_bytes()) != reference["provenance"]["runtime_sha256"]:
        raise ValueError("Changed pinned Pilot runtime")
    grammar = json.loads((PROFILE / "grammar.structure.extract.json").read_text(encoding="utf-8"))
    if grammar["provenance"]["pilot_revision"] != PIN:
        raise ValueError("Changed grammar revision")
    paths = set(reference["provenance"]["scope_sources_sha256"])
    paths.update(path for path in grammar["provenance"]["sources_sha256"] if path.endswith(".xtext"))
    paths.add("org.omg.sysml/model/SysML.ecore")
    paths.update([
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/UsageAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Usage_mayTimeVary_SettingDelegate.java",
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/naming/SysMLQualifiedNameConverter.xtend",
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/postprocessing/PortConjugationParserPostProcessor.java",
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/postprocessing/ConjugationParserPostProcessor.java",
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/postprocessing/SpecializationParserPostProcessor.java",
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/postprocessing/UsageParserPostProcessor.java",
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/postprocessing/PortUsageParserPostProcessor.java",
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/postprocessing/OccurrenceUsageParserPostProcessor.java",
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/postprocessing/DefinitionParserPostProcessor.java",
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/postprocessing/FeatureParserPostProcessor.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/TypeUtil.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/ConnectorUtil.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Element_qualifiedName_SettingDelegate.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/ElementUtil.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Feature_ownedTypeFeaturing_SettingDelegate.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Element_owner_SettingDelegate.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Feature_chainingFeature_SettingDelegate.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Feature_featureTarget_SettingDelegate.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/TypeAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/SysMLLibraryUtil.java",
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/library/KerMLLibraryProvider.xtend",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/UsageUtil.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Usage_isReference_SettingDelegate.java",
    ])
    definition_defaults = json.loads((PROFILE / "definition-defaults.extract.json").read_text(encoding="utf-8"))
    paths.update(definition_defaults["provenance"]["source_sha256"])
    feature_redefinitions = json.loads((PROFILE / "feature-redefinitions.extract.json").read_text(encoding="utf-8"))
    paths.update(feature_redefinitions["provenance"]["source_sha256"])
    hashes = {}
    for path in sorted(paths):
        content = (PILOT / path).read_bytes().replace(b"\r\n", b"\n")
        pinned = subprocess.check_output(["git", "-C", str(PILOT), "show", PIN + ":" + path]).replace(b"\r\n", b"\n")
        if content != pinned:
            raise ValueError("Changed pinned frontend source: " + path)
        hashes[path] = digest(content)
    suffix = ".exe" if os.name == "nt" else ""
    with tempfile.TemporaryDirectory(prefix="definition-document-") as temp:
        raw = Path(temp) / "raw.json"
        subprocess.run([str(args.java_bin / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", str(runtime), "-d", temp, str(HELPER)], check=True, timeout=60)
        subprocess.run([str(args.java_bin / ("java" + suffix)), "-Xmx512m", "-cp", temp + os.pathsep + str(runtime), "dev.mercurio.pilot.PilotDefinitionDocumentProbe", str(raw)], check=True, timeout=60)
        observed = json.loads(raw.read_text(encoding="utf-8"))
    if len(observed["conformance_controls"]) != 4:
        raise ValueError("Changed conformance control inventory")
    if len(observed["variability_controls"]) != 32:
        raise ValueError("Changed variability inventory")
    if len(observed["variation_controls"]) != 8:
        raise ValueError("Changed variation inventory")
    from export_pilot_feature_redefinitions import featuring_supported, usage_featuring_supported
    featuring_kinds = {row["kind"] for row in feature_redefinitions["bindings"] if featuring_supported(row)}
    if len(featuring_kinds) != 32 or {(c["kind"], c["variable"]) for c in observed["featuring_dispatch_controls"]} != {(kind, variable) for kind in featuring_kinds for variable in (False, True)} or len(observed["featuring_dispatch_controls"]) != 64:
        raise ValueError("Changed resolved featuring coverage")
    usage_kinds = {row["kind"] for row in feature_redefinitions["bindings"] if usage_featuring_supported(row)}
    if len(usage_kinds) != 47 or len(observed["usage_featuring_controls"]) != 94 or {(c["kind"], c["portion"]) for c in observed["usage_featuring_controls"]} != {(kind, portion) for kind in usage_kinds for portion in (False, True)}:
        raise ValueError("Changed Usage featuring inventory")
    if len(observed["variable_featuring_controls"]) != 5:
        raise ValueError("Changed variable featuring inventory")
    if len(observed["owning_type_featuring_controls"]) != 8:
        raise ValueError("Changed owning-type featuring inventory")
    if len(observed["binding_construction_controls"]) != 15:
        raise ValueError("Changed binding construction inventory")
    if len(observed["chain_construction_controls"]) != 5:
        raise ValueError("Changed chain construction inventory")
    if len(observed["participant_controls"]) != 6:
        raise ValueError("Changed participant inventory")
    if len(observed["ordinary_strategy_source_controls"]) != 25:
        raise ValueError("Changed ordinary source control inventory")
    controls = observed["controls"]
    if len(controls) != 98 or sum(case["accepted"] for case in controls) != 90:
        raise ValueError("Changed whole-document control admission: " + str(len(controls)) + " controls, rejected=" + repr([c["source"] for c in controls if not c["accepted"]]))
    if len(observed["library_lookup_disagreements"]) != 2:
        raise ValueError("Changed independent library-provider disagreements")
    if len(observed["unsupported_native_dependencies"]) != 1 or not all(c["accepted"] for c in observed["unsupported_native_dependencies"]):
        raise ValueError("Changed recorded unsupported native dependencies")
    doc = {"schema_version": 1,
           "scope": "Bounded full RootNamespace parse and source-constructed containment, non-derived EAttribute getter/default projection (including getter overrides such as Membership.memberName, excluding generated elementId), and all stored reference endpoints including parser-postprocessed defaults and actual Xtext lazy reference resolution for 90 self-contained positive documents plus eight malformed documents. No injected resolver. Ten controls exercise plain-Type inherited/default scope; eleven more exercise Definition defaults, including binary-end branches, through explicitly declared standard-library roots. Nine additional matching controls exercise ordinary Feature filtering, absent names, diamonds and qualified inherited members. Six further documents exercise positional end naming, inherited ends, diamonds and visibility in KerML and SysML. Eight further action/calculation documents exercise parameter/result positions, inherited results, diamonds, visibility and result exclusion. Individual Multiplicity naming now links, but its model remains separately unqualified due to Usage.mayTimeVary. No materialized implicit relationships, general derived-semantic qualification, full validator, external resources, or complete-language qualification.",
           **observed,
           "provenance": {"pilot_revision": PIN, "runtime_sha256": digest(runtime.read_bytes()),
                          "source_sha256": hashes, "grammar_sha256": digest((PROFILE / "grammar.structure.extract.json").read_bytes()),
                          "helper_sha256": digest(HELPER.read_bytes()), "driver_sha256": digest(Path(__file__).read_bytes())}}
    rendered = json.dumps(doc, indent=2, sort_keys=True) + "\n"
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != rendered:
            raise ValueError("Stale whole-document Pilot controls")
    else:
        OUTPUT.write_text(rendered, encoding="utf-8", newline="\n")
    print(len(controls), "whole-document controls;", sum(c["accepted"] for c in controls), "accepted;",
          sum(len(c["nodes"]) for c in controls), "canonical nodes;", sum(len(c["links"]) for c in controls), "resolved reference slots")


if __name__ == "__main__":
    main()
