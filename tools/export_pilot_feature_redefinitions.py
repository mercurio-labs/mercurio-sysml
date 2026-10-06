"""Generate bounded relevant-feature selection and dispatch; handwritten Rust supplies semantics."""
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
HELPER = ROOT / "tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotFeatureRedefinitionExporter.java"
OUTPUT = PROFILE / "feature-redefinitions.extract.json"
GENERATED = ROOT / "crates/mercurio-sysml/src/language_frontend/lowering/emit/feature_redefinitions_generated.rs"
PREFIX = "org.omg.sysml.adapter."


def require(value, message):
    if not value: raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def identifier(node, name):
    return node["kind"] == "IDENTIFIER" and node["name"] == name


def call(node, symbol, parameters):
    return node["kind"] == "METHOD_INVOCATION" and node["symbol"] == symbol and node["parameters"] == parameters


def owning_type(node):
    return call(node, "org.omg.sysml.lang.sysml.Feature#getOwningType", []) and not node["arguments"] and identifier(node["receiver"], "target")


def selector(node):
    kind = node["kind"]
    if kind == "PARENTHESIZED": return selector(node["expression"])
    if kind == "CONDITIONAL_EXPRESSION":
        return "if " + selector(node["condition"]) + " { " + selector(node["true"]) + " } else { " + selector(node["false"]) + " }"
    if kind == "EQUAL_TO":
        require(node["type"] == "boolean" and identifier(node["left"], "type") and node["right"]["kind"] == "NULL_LITERAL", "Unassessed null condition")
        return "!has_owner"
    require(kind == "METHOD_INVOCATION", "Unsupported selector node " + kind)
    if call(node, "java.util.Collections#emptyList", []):
        require(not node["arguments"] and identifier(node["receiver"], "Collections"), "Changed empty-list call")
        return "Relevant::None"
    if call(node, "org.omg.sysml.lang.sysml.Feature#isEnd", []):
        require(node["type"] == "boolean" and not node["arguments"] and identifier(node["receiver"], "target"), "Changed end predicate")
        return "is_end"
    if call(node, "org.omg.sysml.util.ExpressionUtil#isConstructorResult", ["org.omg.sysml.lang.sysml.Type"]):
        require(node["type"] == "boolean" and identifier(node["receiver"], "ExpressionUtil") and len(node["arguments"]) == 1 and owning_type(node["arguments"][0]), "Changed constructor predicate")
        return "constructor_result"
    if call(node, "org.omg.sysml.util.FeatureUtil#isParameter", ["org.omg.sysml.lang.sysml.Feature"]):
        require(node["type"] == "boolean" and identifier(node["receiver"], "FeatureUtil") and len(node["arguments"]) == 1 and identifier(node["arguments"][0], "target"), "Changed parameter predicate")
        return "is_parameter"
    for method, category in [("getEndRelevantFeatures", "Ends"), ("getConstructorRelevantFeatures", "Constructor"), ("getParameterRelevantFeatures", "Parameters")]:
        if call(node, PREFIX + "FeatureAdapter#" + method, ["org.omg.sysml.lang.sysml.Type"]):
            require("receiver" not in node and len(node["arguments"]) == 1 and identifier(node["arguments"][0], "type"), "Changed relevant-feature dependency")
            return "Relevant::" + category
    raise ValueError("Unsupported resolved selector call " + node["symbol"])


def body(node):
    require(node["kind"] == "BLOCK", "Expected selector block")
    statements = list(node["statements"])
    if statements and statements[0]["kind"] == "VARIABLE":
        variable = statements.pop(0)
        require(variable["name"] == "target" and variable["type"] == "org.omg.sysml.lang.sysml.Feature", "Changed local target")
        init = variable["initializer"]
        require(call(init, PREFIX + "FeatureAdapter#getTarget", []) and not init["arguments"] and "receiver" not in init, "Changed target source")
    require(len(statements) == 1 and statements[0]["kind"] == "RETURN", "Unassessed selector statements")
    return selector(statements[0]["expression"])


def end_source(node):
    kind = node["kind"]
    if kind == "CONDITIONAL_EXPRESSION":
        return "if " + end_source(node["condition"]) + " { " + end_source(node["true"]) + " } else { " + end_source(node["false"]) + " }"
    if kind == "EQUAL_TO":
        left = node["left"]
        require(call(left, "org.omg.sysml.lang.sysml.Feature#getOwningType", []) and not left["arguments"], "Changed positional owner")
        target = left["receiver"]
        require(call(target, PREFIX + "FeatureAdapter#getTarget", []) and not target["arguments"] and "receiver" not in target and identifier(node["right"], "type"), "Changed positional owner equality")
        return "same_owner"
    if call(node, "org.omg.sysml.lang.sysml.Type#getOwnedEndFeature", []):
        require(identifier(node["receiver"], "type") and not node["arguments"], "Changed owned-end input")
        return "EndSource::Owned"
    if call(node, "org.omg.sysml.util.TypeUtil#getEndFeatureOf", ["org.omg.sysml.lang.sysml.Type"]):
        require(identifier(node["receiver"], "TypeUtil") and len(node["arguments"]) == 1 and identifier(node["arguments"][0], "type"), "Changed effective-end input")
        return "EndSource::Effective"
    raise ValueError("Unsupported resolved end selector " + kind)


def target_call(node):
    return call(node, PREFIX + "FeatureAdapter#getTarget", []) and not node["arguments"] and "receiver" not in node


def parameter_expression(node):
    kind = node["kind"]
    if kind == "PARENTHESIZED": return parameter_expression(node["expression"])
    if kind == "NOT_EQUAL_TO":
        require(node["right"]["kind"] == "NULL_LITERAL", "Changed parameter null guard")
        name = node["left"].get("name")
        require(identifier(node["left"], name) and name in ["type", "resultParameter"], "Changed parameter null input")
        return "has_type" if name == "type" else "result_parameter"
    if call(node, "org.omg.sysml.util.FeatureUtil#isResultParameter", ["org.omg.sysml.lang.sysml.Feature"]):
        require(identifier(node["receiver"], "FeatureUtil") and len(node["arguments"]) == 1 and target_call(node["arguments"][0]), "Changed result predicate")
        return "is_result"
    if call(node, "org.omg.sysml.util.TypeUtil#getResultParameterOf", ["org.omg.sysml.lang.sysml.Type"]):
        require(identifier(node["receiver"], "TypeUtil") and len(node["arguments"]) == 1 and identifier(node["arguments"][0], "type"), "Changed result query")
        return "has_result"
    if call(node, "java.util.Collections#singletonList", ["T"]):
        require(identifier(node["receiver"], "Collections") and len(node["arguments"]) == 1 and identifier(node["arguments"][0], "resultParameter"), "Changed singleton result")
        return "ParameterSelection::Result"
    if call(node, PREFIX + "FeatureAdapter#getRelevantParameters", ["org.omg.sysml.lang.sysml.Type"]):
        require("receiver" not in node and len(node["arguments"]) == 1 and identifier(node["arguments"][0], "type"), "Changed positional parameters")
        return "ParameterSelection::Parameters"
    if call(node, "java.util.Collections#emptyList", []):
        require(identifier(node["receiver"], "Collections") and not node["arguments"], "Changed empty parameter result")
        return "ParameterSelection::Empty"
    raise ValueError("Unsupported resolved parameter expression " + str(node))


def parameter_statements(node):
    kind = node["kind"]
    if kind == "BLOCK": return "\n".join(parameter_statements(s) for s in node["statements"])
    if kind == "IF":
        code = "if " + parameter_expression(node["condition"]) + " {\n" + parameter_statements(node["then"]) + "\n}"
        if "else" in node: code += " else {\n" + parameter_statements(node["else"]) + "\n}"
        return code
    if kind == "VARIABLE":
        require(node["name"] == "resultParameter" and node["type"] == "org.omg.sysml.lang.sysml.Feature", "Changed parameter local")
        return "let result_parameter = " + parameter_expression(node["initializer"]) + ";"
    if kind == "RETURN": return "return " + parameter_expression(node["expression"]) + ";"
    raise ValueError("Unsupported parameter statement " + kind)


def parameter_collection(node):
    kind = node["kind"]
    if kind == "CONDITIONAL_EXPRESSION":
        return "if " + parameter_collection(node["condition"]) + " { " + parameter_collection(node["true"]) + " } else { " + parameter_collection(node["false"]) + " }"
    if kind == "EQUAL_TO":
        require(identifier(node["left"], "type") and identifier(node["right"], "owningType"), "Changed parameter owner comparison")
        return "same_owner"
    for method, selection in [("getOwnedParametersOf", "Owned"), ("getAllParametersOf", "Effective")]:
        if call(node, "org.omg.sysml.util.TypeUtil#" + method, ["org.omg.sysml.lang.sysml.Type"]):
            require(identifier(node["receiver"], "TypeUtil") and len(node["arguments"]) == 1 and identifier(node["arguments"][0], "type"), "Changed parameter collection")
            return "ParameterCollection::" + selection
    raise ValueError("Unsupported parameter collection " + kind)


def parameter_programs(methods):
    selection = parameter_statements(methods[PREFIX + "FeatureAdapter#getParameterRelevantFeatures"])
    body = methods[PREFIX + "FeatureAdapter#getRelevantParameters"]
    require(body["kind"] == "BLOCK" and len(body["statements"]) == 2, "Changed parameter collection statements")
    variable, ret = body["statements"]
    require(variable["kind"] == "VARIABLE" and variable["name"] == "owningType" and variable["type"] == "org.omg.sysml.lang.sysml.Type", "Changed owning Type local")
    init = variable["initializer"]
    require(call(init, "org.omg.sysml.lang.sysml.Feature#getOwningType", []) and not init["arguments"] and target_call(init["receiver"]), "Changed parameter ownership source")
    require(ret["kind"] == "RETURN", "Changed parameter collection return")
    filtered = ret["expression"]
    require(call(filtered, PREFIX + "FeatureAdapter#filterIgnoredParameters", ["java.util.List<org.omg.sysml.lang.sysml.Feature>"]) and "receiver" not in filtered and len(filtered["arguments"]) == 1, "Changed parameter filtering dependency")
    collection = parameter_collection(filtered["arguments"][0])
    # Validate the resolved stream filter; the Rust collection traversal is a
    # named handwritten dependency, while the exclusion predicate is generated.
    block = methods[PREFIX + "FeatureAdapter#filterIgnoredParameters"]
    require(block["kind"] == "BLOCK" and len(block["statements"]) == 1 and block["statements"][0]["kind"] == "RETURN", "Changed parameter filter statements")
    collect = block["statements"][0]["expression"]
    require(call(collect, "java.util.stream.Stream#collect", ["java.util.stream.Collector<? super T,A,R>"]) and len(collect["arguments"]) == 1, "Changed parameter collector")
    to_list = collect["arguments"][0]
    require(call(to_list, "java.util.stream.Collectors#toList", []) and identifier(to_list["receiver"], "Collectors") and not to_list["arguments"], "Changed ordered list collector")
    filtered_stream = collect["receiver"]
    require(call(filtered_stream, "java.util.stream.Stream#filter", ["java.util.function.Predicate<? super T>"]) and len(filtered_stream["arguments"]) == 1, "Changed parameter filter")
    stream = filtered_stream["receiver"]
    require(call(stream, "java.util.Collection#stream", []) and identifier(stream["receiver"], "parameters") and not stream["arguments"], "Changed parameter stream")
    predicate = filtered_stream["arguments"][0]
    require(predicate["kind"] == "LAMBDA_EXPRESSION" and len(predicate["parameters"]) == 1 and predicate["parameters"][0]["name"] == "p", "Changed filter lambda")
    negated = predicate["body"]
    require(negated["kind"] == "LOGICAL_COMPLEMENT", "Changed parameter exclusion")
    ignored = negated["expression"]
    require(call(ignored, "org.omg.sysml.util.FeatureUtil#isIgnoredParameter", ["org.omg.sysml.lang.sysml.Feature"]) and identifier(ignored["receiver"], "FeatureUtil") and len(ignored["arguments"]) == 1 and identifier(ignored["arguments"][0], "p"), "Changed ignored-parameter dependency")
    ignored_body = methods[PREFIX + "FeatureAdapter#isIgnoredParameter"]
    require(ignored_body["kind"] == "BLOCK" and len(ignored_body["statements"]) == 1 and ignored_body["statements"][0]["kind"] == "RETURN", "Changed ignored parameter body")
    ignored_expression = parameter_expression(ignored_body["statements"][0]["expression"])
    return ["#[derive(Clone, Copy, Debug, PartialEq, Eq)]", "pub(super) enum ParameterSelection { Empty, Result, Parameters }",
            "pub(super) fn parameter_selection(has_type: bool, is_result: bool, has_result: bool) -> ParameterSelection {", selection, "}",
            "#[derive(Clone, Copy, Debug, PartialEq, Eq)]", "pub(super) enum ParameterCollection { Owned, Effective }",
            "pub(super) fn parameter_collection(same_owner: bool) -> ParameterCollection { " + collection + " }",
            "pub(super) fn ignored_parameter(is_result: bool) -> bool { " + ignored_expression + " }"]


def bounded(row):
    dispatch = row["methods"]
    return all(dispatch[name] == PREFIX + "FeatureAdapter#" + name for name in ["addRedefinitions", "addFeatureWriteTypes", "addComputedRedefinitions", "isComputeRedefinitions", "getRedefinedFeaturesWithComputed"]) and dispatch["getRelevantFeatures"] in [PREFIX + "FeatureAdapter#getRelevantFeatures", PREFIX + "MultiplicityAdapter#getRelevantFeatures"]


def featuring_supported(row):
    methods = row["methods"]
    return (methods.get("computeFeaturingType") == PREFIX + "FeatureAdapter#computeFeaturingType"
            and methods.get("isVariableGetter") == "org.omg.sysml.lang.sysml.impl.FeatureImpl#isVariable"
            and methods.get("owningTypeGetter") == "org.omg.sysml.lang.sysml.impl.FeatureImpl#getOwningType")


def usage_featuring_supported(row):
    methods = row["methods"]
    return (methods.get("computeFeaturingType") == PREFIX + "FeatureAdapter#computeFeaturingType"
            and methods.get("isVariableGetter") == "org.omg.sysml.lang.sysml.impl.UsageImpl#isVariable"
            and methods.get("mayTimeVaryGetter") == "org.omg.sysml.lang.sysml.impl.UsageImpl#isMayTimeVary"
            and methods.get("owningTypeGetter") == "org.omg.sysml.lang.sysml.impl.FeatureImpl#getOwningType")


def render(doc):
    require(isinstance(doc["participant_default"], str) and "::" in doc["participant_default"], "Missing participant default binding")
    methods = doc["methods"]
    hook = methods[PREFIX + "NamespaceAdapter#addAdditionalMembers"]
    require(hook["kind"] == "BLOCK" and not hook["statements"], "Additional member algorithm required")
    programs = {key: body(node) for key, node in methods.items() if key.endswith("#getRelevantFeatures")}
    lines = ["// Generated from pinned resolved selector trees and actual adapter dispatch.",
             "pub(super) const PARTICIPANT_DEFAULT: &str = " + json.dumps(doc["participant_default"]) + ";",
             "// Ecore ownership, contextual predicates and redefinition algorithms are handwritten dependencies.",
             "#[derive(Clone, Copy, Debug, PartialEq, Eq)]", "pub(super) enum Relevant { None, Ends, Constructor, Parameters }",
             "pub(super) fn relevant(kind: &str, has_owner: bool, is_end: bool, constructor_result: bool, is_parameter: bool) -> Option<Relevant> {",
             "    Some(match kind {"]
    for row in doc["bindings"]:
        if bounded(row): lines.append("        " + json.dumps(row["kind"]) + " => " + programs[row["methods"]["getRelevantFeatures"]] + ",")
    lines.extend(["        _ => return None,", "    })", "}", "pub(super) fn no_additional_members(kind: &str) -> bool {", "    matches!(kind,"])
    names = [json.dumps(kind) for kind, method in sorted(doc["additional_members_dispatch"].items()) if method == PREFIX + "NamespaceAdapter#addAdditionalMembers"]
    lines.append("        " + " | ".join(names))
    lines.extend(["    )", "}"])
    end = methods[PREFIX + "FeatureAdapter#getEndRelevantFeatures"]
    require(end["kind"] == "BLOCK" and len(end["statements"]) == 1 and end["statements"][0]["kind"] == "RETURN", "Unassessed end selector statements")
    lines.extend(["#[derive(Clone, Copy, Debug, PartialEq, Eq)]", "pub(super) enum EndSource { Owned, Effective }",
                  "pub(super) fn end_source(same_owner: bool) -> EndSource { " + end_source(end["statements"][0]["expression"]) + " }",
                  "pub(super) fn end_strategy_supported(kind: &str) -> bool {", "    matches!(kind,"])
    names = [json.dumps(row["kind"]) for row in doc["bindings"] if bounded(row) and all(row["methods"][name] == PREFIX + "FeatureAdapter#" + name for name in ["getGeneralTypes", "getEndRelevantFeatures"])]
    lines.extend(["        " + " | ".join(names), "    )", "}"])
    lines.extend(parameter_programs(methods))
    lines.extend(["pub(super) fn parameter_strategy_supported(kind: &str) -> bool {", "    matches!(kind,"])
    names = [json.dumps(row["kind"]) for row in doc["bindings"] if bounded(row) and all(row["methods"][name] == PREFIX + "FeatureAdapter#" + name for name in ["getGeneralTypes", "getParameterRelevantFeatures", "getRelevantParameters", "filterIgnoredParameters"])]
    lines.extend(["        " + " | ".join(names), "    )", "}", "pub(super) fn ignored_parameter_supported(kind: &str) -> bool {", "    matches!(kind,"])
    names = [json.dumps(row["kind"]) for row in doc["bindings"] if row["methods"]["isIgnoredParameter"] == PREFIX + "FeatureAdapter#isIgnoredParameter"]
    lines.extend(["        " + " | ".join(names), "    )", "}"])
    names = [json.dumps(row["kind"]) for row in doc["bindings"] if featuring_supported(row)]
    require(bool(names), "No qualified featuring dispatch")
    lines.extend(["pub(super) fn owning_type_featuring_supported(kind: &str) -> bool {",
                  "    matches!(kind, " + " | ".join(names) + ")", "}"])
    names = [json.dumps(row["kind"]) for row in doc["bindings"] if usage_featuring_supported(row)]
    require(bool(names), "No resolved Usage featuring dispatch")
    lines.extend(["pub(super) fn owning_type_featuring_uses_variability(kind: &str) -> bool {",
                  "    matches!(kind, " + " | ".join(names) + ")", "}"])
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    require(subprocess.check_output(["git", "-C", str(PILOT), "rev-parse", "HEAD"], text=True).strip() == PIN, "Changed Pilot revision")
    runtime = PILOT / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    ref = json.loads((PROFILE / "xtext-prediction-nfa.experimental.json").read_text(encoding="utf-8"))
    require(digest(runtime.read_bytes()) == ref["provenance"]["runtime_sha256"], "Changed runtime")
    prefix = "org.omg.sysml.logic/src/main/java/org/omg/sysml/"
    sources = [prefix + "adapter/" + name + ".java" for name in ["FeatureAdapter", "MultiplicityAdapter", "NamespaceAdapter"]]
    suffix = ".exe" if os.name == "nt" else ""
    with tempfile.TemporaryDirectory(prefix="feature-redefinitions-") as temp:
        raw = Path(temp) / "raw.json"
        subprocess.run([str(args.java_bin / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", str(runtime), "-d", temp, str(HELPER)], check=True, timeout=60)
        subprocess.run([str(args.java_bin / ("java" + suffix)), "-cp", temp + os.pathsep + str(runtime), "dev.mercurio.pilot.PilotFeatureRedefinitionExporter", str(runtime), str(raw), *[str(PILOT / p) for p in sources]], check=True, timeout=90)
        doc = json.loads(raw.read_text(encoding="utf-8"))
    paths = set(sources) | {"org.omg.sysml/model/SysML.ecore", prefix + "util/ImplicitGeneralizationMap.java", prefix + "delegate/setting/Feature_endOwningType_SettingDelegate.java"}
    paths.update(prefix + name for name in ["adapter/TypeAdapter.java", "util/FeatureUtil.java", "util/TypeUtil.java", "util/ExpressionUtil.java", "delegate/invocation/Feature_namingFeature_InvocationDelegate.java", "delegate/setting/Feature_owningType_SettingDelegate.java", "delegate/setting/Type_endFeature_SettingDelegate.java", "delegate/setting/Type_featureMembership_SettingDelegate.java", "delegate/setting/Feature_owningFeatureMembership_SettingDelegate.java", "delegate/setting/FeatureMembership_owningType_SettingDelegate.java"])
    paths.update(prefix + "adapter/" + row["adapter"].rsplit(".",1)[-1] + ".java" for row in doc["bindings"])
    paths.update(prefix + "adapter/" + method.split("#")[0].rsplit(".",1)[-1] + ".java" for method in doc["additional_members_dispatch"].values())
    paths.add("org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/FeatureImpl.java")
    paths.update("org.omg.sysml.model/src/main/java/" + row["methods"][key].split("#")[0].replace(".", "/") + ".java"
                 for row in doc["bindings"] for key in ("isVariableGetter", "owningTypeGetter"))
    paths.add("org.omg.kerml.xtext/src/org/omg/kerml/xtext/postprocessing/ParameterMembershipParserPostProcessor.java")
    paths.update(prefix + name for name in ["delegate/invocation/ReturnParameterMembership_parameterDirection_InvocationDelegate.java", "delegate/setting/Type_feature_SettingDelegate.java", "delegate/setting/Function_result_SettingDelegate.java", "delegate/setting/Expression_result_SettingDelegate.java"])
    hashes = {}
    for path in sorted(paths):
        content = (PILOT / path).read_bytes().replace(b"\r\n", b"\n")
        require(content == subprocess.check_output(["git", "-C", str(PILOT), "show", PIN + ":" + path]).replace(b"\r\n", b"\n"), "Changed source " + path)
        hashes[path] = digest(content)
    doc["provenance"] = {"pilot_revision": PIN, "runtime_sha256": digest(runtime.read_bytes()), "source_sha256": hashes, "helper_sha256": digest(HELPER.read_bytes()), "driver_sha256": digest(Path(__file__).read_bytes())}
    doc["scope"] = "Generated relevant-feature selectors and adapter dispatch. Native ordered redefinition algorithms, ownership, contextual predicates and missing positional dependencies are assessed separately. Controls exercise detached, Package-owned and Class-owned ordinary contexts and 990 positional end contexts over materialized Class ancestry. Includes 1440 Function-owned parameter/result contexts and 30 explicit InvocationExpression suppression controls. Native consumers and verification are recorded separately. Constructors, cyclic/conjugated feature ancestry, unsupported providers and recursive Feature chain construction remain separate dependencies."
    require(len(doc["bindings"]) == 79 and sum(bounded(row) for row in doc["bindings"]) == 30 and len(doc["controls"]) == 180 and len(doc["end_controls"]) == 990 and len(doc["parameter_controls"]) == 1440 and len(doc["invocation_controls"]) == 30, "Changed pinned strategy inventory")
    generated = render(doc)
    rendered = json.dumps(doc, indent=2, sort_keys=True) + "\n"
    if args.check:
        require(OUTPUT.read_text(encoding="utf-8") == rendered and GENERATED.read_text(encoding="utf-8") == generated, "Stale Feature strategy artifacts")
    else:
        OUTPUT.write_text(rendered, encoding="utf-8", newline="\n")
        GENERATED.write_text(generated, encoding="utf-8", newline="\n")
    print(len(doc["bindings"]), "resolved Feature dispatch rows;", sum(bounded(row) for row in doc["bindings"]), "bounded strategies;", len(doc["controls"]), "ordinary controls;", len(doc["end_controls"]), "end controls;", len(doc["parameter_controls"]), "parameter/result controls;", len(doc["invocation_controls"]), "invocation controls")


if __name__ == "__main__": main()
