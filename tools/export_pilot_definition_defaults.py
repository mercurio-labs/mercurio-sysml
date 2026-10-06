"""Compile bounded Definition default policies from resolved javac trees, then verify runtime controls."""
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
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotDefinitionDefaultExporter.java"
OUTPUT = PROFILE / "definition-defaults.extract.json"
GENERATED = ROOT / "crates/mercurio-sysml/src/language_frontend/lowering/emit/definition_defaults_generated.rs"
PREFIX = "org.omg.sysml.adapter."
BASE = PREFIX + "TypeAdapter#addDefaultGeneralType"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def receiver_target(node):
    require(node["kind"] == "METHOD_INVOCATION" and node["symbol"].endswith("#getTarget")
            and node["symbol"].startswith(PREFIX) and not node["arguments"] and not node["parameters"]
            and node["type"].startswith("org.omg.sysml.lang.sysml.") and "receiver" not in node,
            "Unassessed target receiver")


def expr(node, binding, used):
    kind = node["kind"]
    if kind == "PARENTHESIZED":
        return expr(node["expression"], binding, used)
    if kind == "CONDITIONAL_EXPRESSION":
        require(node["type"] == "java.lang.String", "Changed selector type")
        return "if " + expr(node["condition"], binding, used) + " { " + expr(node["true"], binding, used) + " } else { " + expr(node["false"], binding, used) + " }"
    if kind == "NOT_EQUAL_TO":
        require(node["type"] == "boolean", "Changed comparison type")
        return "(" + expr(node["left"], binding, used) + " != " + expr(node["right"], binding, used) + ")"
    if kind == "INT_LITERAL":
        require(node["type"] == "int" and isinstance(node["value"], int) and node["value"] >= 0, "Unsupported integer")
        return str(node["value"])
    require(kind == "METHOD_INVOCATION", "Unsupported expression: " + kind)
    symbol = node["symbol"]
    if symbol == PREFIX + "TypeAdapter#getDefaultSupertype":
        require(node["parameters"] == ["java.lang.String"] and node["type"] == "java.lang.String" and "receiver" not in node,
                "Unassessed default-name call")
        require(len(node["arguments"]) == 1 and node["arguments"][0]["kind"] == "STRING_LITERAL", "Nonliteral default key")
        return json.dumps(binding["labels"][node["arguments"][0]["value"]])
    if symbol == "org.omg.sysml.lang.sysml.OccurrenceDefinition#isIndividual":
        require(node["type"] == "boolean" and not node["parameters"] and not node["arguments"], "Changed individual getter")
        receiver_target(node["receiver"]); used.add("individual")
        return "individual"
    if symbol == "java.util.List#size":
        require(node["type"] == "int" and not node["parameters"] and not node["arguments"], "Changed count operation")
        receiver = node["receiver"]
        require(receiver["kind"] == "METHOD_INVOCATION" and receiver["symbol"] == "org.omg.sysml.lang.sysml.Type#getOwnedEndFeature"
                and not receiver["parameters"] and not receiver["arguments"] and receiver["type"] == "org.eclipse.emf.common.util.EList<org.omg.sysml.lang.sysml.Feature>",
                "Unassessed collection dependency")
        receiver_target(receiver["receiver"]); used.add("owned_ends")
        return "owned_end_count"
    raise ValueError("Unsupported resolved call: " + symbol)


def statements(node, binding, methods, used):
    kind = node["kind"]
    if kind == "BLOCK":
        return "\n".join(statements(s, binding, methods, used) for s in node["statements"])
    if kind == "RETURN":
        return "names.push(" + expr(node["expression"], binding, used) + ");"
    if kind == "IF":
        return "if " + expr(node["condition"], binding, used) + " {\n" + statements(node["then"], binding, methods, used) + "\n}"
    require(kind == "EXPRESSION_STATEMENT", "Unsupported statement " + kind)
    call = node["expression"]
    require(call["kind"] == "METHOD_INVOCATION" and call["symbol"] == BASE and call["type"] == "void", "Unassessed effect")
    if not call["parameters"]:
        require(not call["arguments"] and call["receiver"]["kind"] == "IDENTIFIER" and call["receiver"]["name"] == "super", "Changed base dependency")
        return statements(methods[binding["selection"]], binding, methods, used)
    require(call["parameters"] == ["java.lang.String"] and "receiver" not in call and len(call["arguments"]) == 1, "Changed append dependency")
    argument = call["arguments"][0]
    require(argument["kind"] == "STRING_LITERAL", "Nonliteral additional default")
    return "names.push(" + json.dumps(binding["labels"][argument["value"]]) + ");"


def render(doc):
    members = doc["methods"][PREFIX + "NamespaceAdapter#addAdditionalMembers"]
    require(members["kind"] == "BLOCK" and members["statements"] == [], "Additional members require a native algorithm")
    lines = ["// Generated from resolved javac policy trees by tools/export_pilot_definition_defaults.py.",
             "// Handwritten dependencies supply individual, owned-end count, metadata and library lookup.",
             "pub(super) fn inputs(kind: &str) -> Option<(bool, bool)> {", "    match kind {"]
    programs = []
    for binding in doc["bindings"]:
        used = set()
        body = statements(doc["methods"][binding["selection"] if binding["additions"] == BASE else binding["additions"]], binding, doc["methods"], used)
        lines.append("        " + json.dumps(binding["kind"]) + " => Some((" + str("individual" in used).lower() + ", " + str("owned_ends" in used).lower() + ")), ")
        programs.append((binding["kind"], body))
    lines.extend(["        _ => None,", "    }", "}", "#[allow(unused_parens)]", "pub(super) fn names(kind: &str, individual: bool, owned_end_count: usize) -> Option<Vec<&'static str>> {", "    let mut names = Vec::new();", "    match kind {"])
    for kind, body in programs:
        lines.append("        " + json.dumps(kind) + " => {\n" + "\n".join("            " + s for s in body.splitlines()) + "\n        },")
    lines.extend(["        _ => return None,", "    }", "    Some(names)", "}"])
    return "\n".join(line.rstrip() for line in lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    require(subprocess.check_output(["git", "-C", str(PILOT), "rev-parse", "HEAD"], text=True).strip() == PIN, "Changed Pilot revision")
    runtime = PILOT / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    ref = json.loads((PROFILE / "xtext-prediction-nfa.experimental.json").read_text(encoding="utf-8"))
    require(digest(runtime.read_bytes()) == ref["provenance"]["runtime_sha256"], "Changed runtime")
    prefix = "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/"
    sources = [prefix + name + ".java" for name in ["TypeAdapter", "OccurrenceDefinitionAdapter", "ConnectionDefinitionAdapter", "FlowDefinitionAdapter", "NamespaceAdapter"]]
    suffix = ".exe" if os.name == "nt" else ""
    with tempfile.TemporaryDirectory(prefix="definition-defaults-") as temp:
        raw = Path(temp) / "raw.json"
        subprocess.run([str(args.java_bin / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", str(runtime), "-d", temp, str(HELPER)], check=True, timeout=60)
        subprocess.run([str(args.java_bin / ("java" + suffix)), "-cp", temp + os.pathsep + str(runtime), "dev.mercurio.pilot.PilotDefinitionDefaultExporter", str(runtime), str(raw), *[str(PILOT / s) for s in sources]], check=True, timeout=90)
        doc = json.loads(raw.read_text(encoding="utf-8"))
    paths = set(sources) | {"org.omg.sysml/model/SysML.ecore", "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/ImplicitGeneralizationMap.java", "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Type_ownedEndFeature_SettingDelegate.java"}
    paths.add("org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/setting/Type_ownedFeature_SettingDelegate.java")
    paths.update(prefix + row["adapter"].rsplit(".", 1)[-1] + ".java" for row in doc["bindings"])
    paths.update(prefix + name + ".java" for name in ["NamespaceAdapter", "ClassifierAdapter"])
    hashes = {}
    for path in sorted(paths):
        content = (PILOT / path).read_bytes().replace(b"\r\n", b"\n")
        require(content == subprocess.check_output(["git", "-C", str(PILOT), "show", PIN + ":" + path]).replace(b"\r\n", b"\n"), "Changed source " + path)
        hashes[path] = digest(content)
    doc = {"schema_version": 1, "scope": "Bounded resolved default-selector and conditional-addition bodies for concrete Definition adapters. Actual runtime controls supply library bindings only. Canonical owned-end derivation, semantic metadata, general-type filtering, added members and library lookup are separate native dependencies; extraction alone is not support.", **doc,
           "provenance": {"pilot_revision": PIN, "runtime_sha256": digest(runtime.read_bytes()), "source_sha256": hashes, "helper_sha256": digest(HELPER.read_bytes()), "driver_sha256": digest(Path(__file__).read_bytes())}}
    require(len(doc["bindings"]) == 26 and len(doc["controls"]) == 196, "Changed bounded Definition inventory")
    generated = render(doc)
    rendered = json.dumps(doc, indent=2, sort_keys=True) + "\n"
    if args.check:
        require(OUTPUT.read_text(encoding="utf-8") == rendered and GENERATED.read_text(encoding="utf-8") == generated, "Stale Definition default artifact")
    else:
        OUTPUT.write_text(rendered, encoding="utf-8", newline="\n")
        GENERATED.write_text(generated, encoding="utf-8", newline="\n")
    print(len(doc["bindings"]), "resolved Definition dispatch rows;", len(doc["controls"]), "independent runtime controls")


if __name__ == "__main__":
    main()
