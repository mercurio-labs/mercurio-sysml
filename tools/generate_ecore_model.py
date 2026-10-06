"""Generate native structural-feature contracts from the pinned effective Ecore."""

import argparse
import hashlib
import json
import re

from export_pilot_ecore_effective import PROFILE, ROOT, cross_check


INPUT = PROFILE / "ecore-effective.extract.json"
SEMANTICS = PROFILE / "ecore-semantics.extract.json"
OUTPUT = ROOT / "crates/mercurio-sysml/src/language_frontend/lowering/ecore_model_generated.rs"
SYSML = "https://www.omg.org/spec/SysML/20250201#//"
ECORE = "http://www.eclipse.org/emf/2002/Ecore#//"


def local(ref):
    if ref is None:
        return None
    if ref.startswith(SYSML):
        return ref[len(SYSML):]
    if ref.startswith(ECORE):
        return "Ecore::" + ref[len(ECORE):]
    raise ValueError("Unresolved Ecore reference: " + ref)


def snake(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def generate(doc, semantics):
    enums = {r["id"]: [] for r in semantics["elements"] if r["kind"] == "EEnum"}
    for row in semantics["elements"]:
        if row["kind"] == "EEnumLiteral":
            if row["owner"] not in enums:
                raise ValueError("Unresolved enum literal owner: " + row["id"])
            literal = row["attributes"].get("literal", row["name"])
            if literal in enums[row["owner"]]:
                raise ValueError("Ambiguous enum literal: " + row["id"])
            enums[row["owner"]].append(literal)
    if any(not literals for literals in enums.values()):
        raise ValueError("Empty Ecore enumeration")
    primitives = {ECORE + name for name in ("EBoolean", "EString", "EInt", "EDouble")}
    classes = {c["id"]: c for c in doc["classes"]}
    features = {f["id"]: f for f in doc["features"]}
    if len(classes) != len(doc["classes"]) or len(features) != len(doc["features"]):
        raise ValueError("Duplicate effective Ecore identity")
    for class_id, row in classes.items():
        local(class_id)
        for parent in row["super_types"]:
            if parent not in classes:
                raise ValueError("Unresolved superclass: " + parent)

    settings = {}
    for binding in semantics["delegate_bindings"]:
        if binding["kind"] != "setting":
            continue
        identity = binding["element"]
        if identity not in features or identity in settings:
            raise ValueError("Unknown or duplicate setting delegate: " + identity)
        settings[identity] = binding

    inherited = {}
    active = set()

    def ancestors(class_id):
        if class_id in inherited:
            return inherited[class_id]
        if class_id in active:
            raise ValueError("Cyclic Ecore inheritance: " + class_id)
        active.add(class_id)
        result = {class_id}
        for parent in classes[class_id]["super_types"]:
            result.update(ancestors(parent))
        active.remove(class_id)
        inherited[class_id] = result
        return result

    for class_id in classes:
        ancestors(class_id)
    for row in features.values():
        if row["owner"] not in classes:
            raise ValueError("Unresolved feature owner: " + row["id"])
        target = row["type"]
        if row["kind"] == "reference" and target not in classes:
            raise ValueError("Unresolved reference type: " + row["id"])
        if row["kind"] == "attribute" and target not in primitives | enums.keys():
            raise ValueError("Unsupported attribute type: " + row["id"])
        if target in enums and row["default_literal"] is not None and row["default_literal"] not in enums[target]:
            raise ValueError("Invalid enum default: " + row["id"])
        if row["kind"] not in ("reference", "attribute"):
            raise ValueError("Unsupported feature kind: " + row["id"])
        local(target)
        if row["opposite"] is not None and row["opposite"] not in features:
            raise ValueError("Unresolved opposite: " + row["id"])
        if row["lower_bound"] < 0 or (row["upper_bound"] != -1 and row["upper_bound"] < row["lower_bound"]):
            raise ValueError("Invalid Ecore bounds: " + row["id"])
    # An inherited name collision would require explicit Ecore feature override
    # semantics; fail generation until that case is deliberately represented.
    for class_id, parents in inherited.items():
        names = set()
        for row in features.values():
            if row["owner"] in parents:
                key = snake(row["name"])
                if key in names:
                    raise ValueError("Ambiguous inherited Ecore feature: " + class_id + "/" + key)
                names.add(key)

    # Resolve semantic property redefinitions from the pinned annotations. These
    # drive dispatch only; derived target properties still require their delegates.
    redefinitions = {}
    for element in semantics["elements"]:
        for annotation in element.get("annotations", []):
            attrs = annotation.get("attributes", {})
            if attrs.get("source") != "redefines":
                continue
            redefining = element["id"]
            if redefining not in features:
                raise ValueError("Unknown redefining feature: " + redefining)
            refs = attrs.get("references", "").split()
            if not refs:
                raise ValueError("Empty property redefinition: " + redefining)
            for ref in refs:
                base = SYSML + ref[3:] if ref.startswith("#//") else ref
                if base not in features:
                    raise ValueError("Unresolved redefined feature: " + ref)
                child, parent = features[redefining], features[base]
                if parent["owner"] not in inherited[child["owner"]] or base == redefining:
                    raise ValueError("Invalid redefinition ancestry: " + redefining)
                if child["kind"] != parent["kind"]:
                    raise ValueError("Invalid redefinition kind: " + redefining)
                if child["kind"] == "reference" and parent["type"] not in inherited[child["type"]]:
                    raise ValueError("Invalid redefinition target type: " + redefining)
                redefinitions.setdefault(base, set()).add(redefining)
    dispatch = []
    ambiguous = []
    for class_id, parents in sorted(inherited.items()):
        def terminal(identity, visiting):
            if identity in visiting:
                raise ValueError("Cyclic property redefinition: " + identity)
            children = [f for f in redefinitions.get(identity, ()) if features[f]["owner"] in parents]
            if not children:
                return {identity}
            result = set()
            for child in children:
                result.update(terminal(child, visiting | {identity}))
            return result
        for identity, row in sorted(features.items()):
            if row["owner"] not in parents:
                continue
            targets = terminal(identity, set())
            if len(targets) != 1:
                ambiguous.append((local(class_id), snake(row["name"])))
                continue
            target = targets.pop()
            if target != identity:
                dispatch.append((local(class_id), snake(row["name"]), snake(features[target]["name"])))

    # Resolve subset declarations separately from redefinition dispatch. Their
    # transitive closure checks explicit snapshots; it does not derive values.
    subsets = {}
    ordered_subset_sources = {}
    for element in semantics["elements"]:
        for annotation in element.get("annotations", []):
            attrs = annotation.get("attributes", {})
            if attrs.get("source") != "subsets":
                continue
            identity = element["id"]
            if identity not in features:
                raise ValueError("Unknown subsetting property: " + identity)
            refs = attrs.get("references", "").split()
            if not refs:
                raise ValueError("Empty property subset: " + identity)
            for ref in refs:
                base = SYSML + ref[3:] if ref.startswith("#//") else ref
                if base not in features:
                    raise ValueError("Unresolved subset property: " + ref)
                child, parent = features[identity], features[base]
                if parent["owner"] not in inherited[child["owner"]] or child["kind"] != parent["kind"]:
                    raise ValueError("Invalid subset property ancestry or kind: " + identity)
                if child["kind"] == "reference" and parent["type"] not in inherited[child["type"]]:
                    raise ValueError("Invalid subset target type: " + identity)
                ordered_subset_sources.setdefault(identity, []).append(base)
                subsets.setdefault(identity, set()).add(base)
    def supersets(identity, visiting):
        if identity in visiting:
            raise ValueError("Cyclic property subset: " + identity)
        result = set(subsets.get(identity, ()))
        for parent in subsets.get(identity, ()):
            result.update(supersets(parent, visiting | {identity}))
        return result
    subset_contracts = []
    for identity in sorted(subsets):
        for parent in sorted(supersets(identity, set())):
            subset_contracts.append((local(features[identity]["owner"]), snake(features[identity]["name"]), snake(features[parent]["name"])))

    unions = []
    for element in semantics["elements"]:
        if not any(a.get("attributes", {}).get("source") == "union" for a in element.get("annotations", [])):
            continue
        identity = element["id"]
        if identity not in features or not features[identity]["derived"]:
            raise ValueError("Union annotation requires a derived structural feature: " + identity)
        contributors = sorted(child for child, parents in subsets.items() if identity in parents)
        if not contributors:
            raise ValueError("Union requires declared subset inputs: " + identity)
        unions.append((features[identity], [features[child] for child in contributors]))

    digest = hashlib.sha256(json.dumps(doc, sort_keys=True).encode()).hexdigest()
    lines = [
        "// Generated by tools/generate_ecore_model.py; do not edit.",
        "// Resolved EMF input SHA256: " + digest,
        "// Ecore semantic declarations SHA256: " + hashlib.sha256(json.dumps(semantics, sort_keys=True).encode()).hexdigest(),
        "use super::{FeatureContract, FeatureKind, SettingDelegateContract};",
        "pub(super) const FEATURES: &[FeatureContract] = &[",
    ]
    for row in sorted(features.values(), key=lambda f: f["id"]):
        def quoted(value):
            return "None" if value is None else "Some(" + json.dumps(value) + ")"

        lines.extend([
            "    FeatureContract {",
            f'        id: {json.dumps(row["id"])}, owner: {json.dumps(local(row["owner"]))}, field: {json.dumps(snake(row["name"]))},',
            f'        name: {json.dumps(row["name"])}, enum_literals: ' + ("Some(&[" + ", ".join(json.dumps(v) for v in enums[row["type"]]) + "])" if row["type"] in enums else "None") + ",",
            f'        kind: FeatureKind::{"Reference" if row["kind"] == "reference" else "Attribute"}, target: {json.dumps(local(row["type"]))},',
            f'        lower: {row["lower_bound"]}, upper: {row["upper_bound"]},',
            f'        ordered: {str(row["ordered"]).lower()}, unique: {str(row["unique"]).lower()},',
            f'        containment: {str(row["containment"]).lower()}, container: {str(row["container"]).lower()},',
            f'        derived: {str(row["derived"]).lower()}, transient: {str(row["transient"]).lower()}, volatile: {str(row["volatile"]).lower()},',
            f'        changeable: {str(row["changeable"]).lower()}, unsettable: {str(row["unsettable"]).lower()}, resolve_proxies: ' + ("None" if row["resolve_proxies"] is None else "Some(" + str(row["resolve_proxies"]).lower() + ")") + ",",
            f'        opposite: {quoted(local(row["opposite"]))}, default_literal: {quoted(row["default_literal"])},',
            '        emf_default_json: ' + json.dumps(json.dumps(row["default_value"], ensure_ascii=False), ensure_ascii=False) + ',',
            '        subset_sources: &[' + ', '.join(json.dumps(v) for v in ordered_subset_sources.get(row['id'], [])) + '],',
            '        setting_delegate: ' + (
                'Some(SettingDelegateContract { uri: ' + json.dumps(settings[row["id"]]["delegate_uri"]) +
                ', status: ' + json.dumps(settings[row["id"]]["binding_status"]) +
                ', candidates: &[' + ', '.join(json.dumps(v) for v in settings[row["id"]]["candidate_source_classes"]) + '], fallback: ' + quoted(settings[row["id"]].get("fallback_class")) + ' })'
                if row["id"] in settings else 'None') + ',',
            "    },",
        ])
    lines.extend(["];", "pub(super) const REDEFINED_FIELDS: &[(&str, &str, &str)] = &["])
    lines.extend("    (" + ", ".join(json.dumps(v) for v in entry) + ")," for entry in dispatch)
    lines.extend(["];", "pub(super) const AMBIGUOUS_REDEFINED_FIELDS: &[(&str, &str)] = &["])
    lines.extend("    (" + ", ".join(json.dumps(v) for v in entry) + ")," for entry in ambiguous)
    lines.extend(["];", "pub(super) const SUBSET_FIELDS: &[(&str, &str, &str)] = &["])
    lines.extend("    (" + ", ".join(json.dumps(v) for v in entry) + ")," for entry in subset_contracts)
    lines.extend(["];", "pub(super) const UNION_FIELDS: &[(&str, &str, &[(&str, &str)])] = &["])
    for union, contributors in unions:
        inputs = ", ".join("(" + json.dumps(local(c["owner"])) + ", " + json.dumps(snake(c["name"])) + ")" for c in contributors)
        lines.append("    (" + json.dumps(local(union["owner"])) + ", " + json.dumps(snake(union["name"])) + ", &[" + inputs + "]),")
    lines.extend(["];", ""])
    # This bounded consumer requires the resolved operation and dynamic selector,
    # not a guessed namespace rule or a signature counted as implementation.
    operations = [o for o in doc["operations"] if o["name"] == "libraryNamespace"]
    bindings = [b for b in semantics["delegate_bindings"] if b["element"].endswith("#//Element/operation:libraryNamespace()")]
    if len(operations) != 1 or len(bindings) != 1:
        raise ValueError("Ambiguous or missing libraryNamespace operation")
    operation, binding = operations[0], bindings[0]
    if local(operation["owner"]) != "Element" or local(operation["type"]) != "Namespace" or operation["parameters"] or operation["exceptions"] or operation["ordered"] or not operation["unique"]:
        raise ValueError("Changed libraryNamespace signature")
    branches = []
    for candidate in binding["candidate_source_classes"]:
        owner, suffix = candidate.rsplit(".", 1)[-1].split("_", 1)
        if suffix != "libraryNamespace_InvocationDelegate" or SYSML + owner not in classes:
            raise ValueError("Unresolved libraryNamespace invocation branch")
        branches.append((owner, candidate))
    lines.extend([
        "pub(super) const LIBRARY_NAMESPACE: super::LibraryNamespaceContract = super::LibraryNamespaceContract {",
        "    owner: " + json.dumps(local(operation["owner"])) + ", target: " + json.dumps(local(operation["type"])) + ",",
        f"    lower: {operation['lower_bound']}, upper: {operation['upper_bound']},",
        "    delegate_uri: " + json.dumps(binding["delegate_uri"]) + ",",
        "    binding_status: " + json.dumps(binding["binding_status"]) + ",",
        "    branches: &[",
    ])
    lines.extend("        (" + json.dumps(owner) + ", " + json.dumps(candidate) + ")," for owner, candidate in branches)
    lines.extend(["    ],", "};", ""])
    operations = [o for o in doc["operations"] if o["name"] == "modelLevelEvaluable"]
    bindings = [b for b in semantics["delegate_bindings"] if "#//Expression/operation:modelLevelEvaluable(" in b["element"]]
    if len(operations) != 1 or len(bindings) != 1:
        raise ValueError("Ambiguous or missing modelLevelEvaluable operation")
    operation, binding = operations[0], bindings[0]
    expected_parameter = {"name": "visited", "type": SYSML + "Feature", "lower_bound": 0, "upper_bound": -1, "ordered": False, "unique": True}
    if local(operation["owner"]) != "Expression" or local(operation["type"]) != "Ecore::EBoolean" or operation["parameters"] != [expected_parameter] or operation["exceptions"] or operation["ordered"] or not operation["unique"] or operation["lower_bound"] != 1 or operation["upper_bound"] != 1:
        raise ValueError("Changed modelLevelEvaluable signature")
    if binding["delegate_uri"] != "http://www.omg.org/spec/SysML" or binding["binding_status"] != "dynamic_invocation_candidates_not_selected" or binding["dispatch_source_class"] != "org.omg.sysml.delegate.invocation.OperationInvocationDelegateSelector":
        raise ValueError("Changed modelLevelEvaluable invocation selector")
    branches = []
    for candidate in binding["candidate_source_classes"]:
        owner, suffix = candidate.rsplit(".", 1)[-1].split("_", 1)
        if suffix != "modelLevelEvaluable_InvocationDelegate" or SYSML + owner not in classes:
            raise ValueError("Unresolved modelLevelEvaluable invocation branch")
        branches.append((owner, candidate))
    if not branches or len(set(branches)) != len(branches):
        raise ValueError("Ambiguous modelLevelEvaluable invocation branches")
    lines.extend([
        "pub(super) const MODEL_LEVEL_EVALUABLE: super::InvocationContract = super::InvocationContract {",
        '    name: "modelLevelEvaluable", owner: "Expression", target: "Ecore::EBoolean", lower: 1, upper: 1, ordered: false, unique: true,',
        '    parameters: &[super::InvocationParameterContract { name: "visited", target: "Feature", lower: 0, upper: -1, ordered: false, unique: true }],',
        "    delegate_uri: " + json.dumps(binding["delegate_uri"]) + ",",
        "    binding_status: " + json.dumps(binding["binding_status"]) + ",",
        "    branches: &[",
    ])
    lines.extend("        (" + json.dumps(owner) + ", " + json.dumps(candidate) + ")," for owner, candidate in branches)
    lines.extend(["    ],", "};", ""])
    operations = [o for o in doc["operations"] if o["name"] == "directionOf"]
    bindings = [b for b in semantics["delegate_bindings"] if "#//Type/operation:directionOf(" in b["element"]]
    if len(operations) != 1 or len(bindings) != 1:
        raise ValueError("Ambiguous or missing directionOf operation")
    operation, binding = operations[0], bindings[0]
    expected_parameter = {"name": "feature", "type": SYSML + "Feature", "lower_bound": 1, "upper_bound": 1, "ordered": False, "unique": True}
    if local(operation["owner"]) != "Type" or local(operation["type"]) != "FeatureDirectionKind" or operation["parameters"] != [expected_parameter] or operation["exceptions"] or operation["ordered"] or not operation["unique"] or operation["lower_bound"] != 0 or operation["upper_bound"] != 1:
        raise ValueError("Changed directionOf signature")
    if binding["delegate_uri"] != "http://www.omg.org/spec/SysML" or binding["binding_status"] != "dynamic_invocation_candidates_not_selected" or binding["dispatch_source_class"] != "org.omg.sysml.delegate.invocation.OperationInvocationDelegateSelector":
        raise ValueError("Changed directionOf invocation selector")
    candidates = ["org.omg.sysml.delegate.invocation.Type_directionOf_InvocationDelegate"]
    if binding["candidate_source_classes"] != candidates:
        raise ValueError("Unresolved directionOf invocation branch")
    lines.extend([
        "pub(super) const DIRECTION_OF: super::InvocationContract = super::InvocationContract {",
        '    name: "directionOf", owner: "Type", target: "FeatureDirectionKind", lower: 0, upper: 1, ordered: false, unique: true,',
        '    parameters: &[super::InvocationParameterContract { name: "feature", target: "Feature", lower: 1, upper: 1, ordered: false, unique: true }],',
        "    delegate_uri: " + json.dumps(binding["delegate_uri"]) + ",",
        "    binding_status: " + json.dumps(binding["binding_status"]) + ",",
        '    branches: &[("Type", "org.omg.sysml.delegate.invocation.Type_directionOf_InvocationDelegate")],',
        "};", ""])
    operations = [o for o in doc["operations"] if o["name"] == "parameterDirection"]
    bindings = [b for b in semantics["delegate_bindings"] if b["element"].endswith("#//ParameterMembership/operation:parameterDirection()")]
    if len(operations) != 1 or len(bindings) != 1:
        raise ValueError("Ambiguous or missing parameterDirection operation")
    operation, binding = operations[0], bindings[0]
    if (local(operation["owner"]) != "ParameterMembership" or local(operation["type"]) != "FeatureDirectionKind"
            or operation["parameters"] or operation["exceptions"] or operation["ordered"] or not operation["unique"]
            or operation["lower_bound"] != 1 or operation["upper_bound"] != 1):
        raise ValueError("Changed parameterDirection signature")
    if (binding["delegate_uri"] != "http://www.omg.org/spec/SysML"
            or binding["binding_status"] != "dynamic_invocation_candidates_not_selected"
            or binding["dispatch_source_class"] != "org.omg.sysml.delegate.invocation.OperationInvocationDelegateSelector"):
        raise ValueError("Changed parameterDirection invocation selector")
    branches = []
    for candidate in binding["candidate_source_classes"]:
        owner, suffix = candidate.rsplit(".", 1)[-1].split("_", 1)
        if (suffix != "parameterDirection_InvocationDelegate" or SYSML + owner not in classes
                or SYSML + "ParameterMembership" not in inherited[SYSML + owner]):
            raise ValueError("Unresolved parameterDirection invocation branch")
        branches.append((owner, candidate))
    if not branches or len({owner for owner, _ in branches}) != len(branches):
        raise ValueError("Missing or duplicate parameterDirection invocation branch")
    lines.extend([
        "pub(super) const PARAMETER_DIRECTION: super::InvocationContract = super::InvocationContract {",
        '    name: "parameterDirection", owner: "ParameterMembership", target: "FeatureDirectionKind", lower: 1, upper: 1, ordered: false, unique: true,',
        '    parameters: &[],',
        "    delegate_uri: " + json.dumps(binding["delegate_uri"]) + ",",
        "    binding_status: " + json.dumps(binding["binding_status"]) + ",",
        "    branches: &[",
    ])
    lines.extend("        (" + json.dumps(owner) + ", " + json.dumps(candidate) + ")," for owner, candidate in branches)
    lines.extend(["    ],", "};", ""])

    operations = [o for o in doc["operations"] if o["name"] == "sourceTargetFeature"]
    bindings = [b for b in semantics["delegate_bindings"] if b["element"].endswith("#//FeatureChainExpression/operation:sourceTargetFeature()")]
    if len(operations) != 1 or len(bindings) != 1:
        raise ValueError("Ambiguous or missing sourceTargetFeature operation")
    operation, binding = operations[0], bindings[0]
    if (local(operation["owner"]) != "FeatureChainExpression" or local(operation["type"]) != "Feature"
            or operation["parameters"] or operation["exceptions"] or operation["ordered"] or not operation["unique"]
            or operation["lower_bound"] != 0 or operation["upper_bound"] != 1):
        raise ValueError("Changed sourceTargetFeature signature")
    if (binding["delegate_uri"] != "http://www.omg.org/spec/SysML"
            or binding["binding_status"] != "dynamic_invocation_candidates_not_selected"
            or binding["dispatch_source_class"] != "org.omg.sysml.delegate.invocation.OperationInvocationDelegateSelector"):
        raise ValueError("Changed sourceTargetFeature invocation selector")
    candidates = binding["candidate_source_classes"]
    if candidates != ["org.omg.sysml.delegate.invocation.FeatureChainExpression_sourceTargetFeature_InvocationDelegate"]:
        raise ValueError("Unreviewed sourceTargetFeature invocation branch")
    lines.extend([
        "pub(super) const SOURCE_TARGET_FEATURE: super::InvocationContract = super::InvocationContract {",
        '    name: "sourceTargetFeature", owner: "FeatureChainExpression", target: "Feature", lower: 0, upper: 1, ordered: false, unique: true,',
        '    parameters: &[],',
        "    delegate_uri: " + json.dumps(binding["delegate_uri"]) + ",",
        "    binding_status: " + json.dumps(binding["binding_status"]) + ",",
        '    branches: &[("FeatureChainExpression", ' + json.dumps(candidates[0]) + ')],',
        "};", ""
    ])
    return "\n".join(lines).encode()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    doc = json.loads(INPUT.read_text(encoding="utf-8"))
    cross_check(doc, json.loads((PROFILE / "metamodel.extract.json").read_text(encoding="utf-8")))
    output = generate(doc, json.loads(SEMANTICS.read_text(encoding="utf-8")))
    if args.check:
        if OUTPUT.read_bytes() != output:
            raise ValueError("Stale native Ecore feature contracts")
    else:
        OUTPUT.write_bytes(output)
    print("Native Ecore feature contracts current:", len(doc["features"]), "features")


if __name__ == "__main__":
    main()
