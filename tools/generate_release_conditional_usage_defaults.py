"""Extract bounded conditional usage defaults from pinned Pilot adapters and map."""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--pilot-root", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--check", action="store_true")
    a = p.parse_args()
    def git(*args):
        return subprocess.check_output(["git", "-C", str(a.pilot_root), *args], text=True).strip()
    if git("status", "--porcelain"):
        raise ValueError("Pilot checkout must be clean")
    prefix = "org.omg.sysml.logic/src/main/java/org/omg/sysml/"
    paths = [prefix + "util/ImplicitGeneralizationMap.java", prefix + "adapter/ConnectionUsageAdapter.java", prefix + "adapter/TransitionUsageAdapter.java"]
    paths.extend([prefix + "adapter/ConnectorAdapter.java", prefix + "adapter/FeatureAdapter.java"])
    model_prefix = "org.omg.sysml.model/src/main/java/org/omg/sysml/lang/sysml/impl/"
    paths.extend([model_prefix + kind + "Impl.java" for kind in ["BindingConnector", "Succession"]])
    paths.extend([prefix + "adapter/SuccessionAsUsageAdapter.java", prefix + "adapter/BindingConnectorAsUsageAdapter.java", prefix + "adapter/ConnectorAsUsageAdapter.java"])
    table, connection, transition, connector, feature, binding_impl, succession_impl, succession_usage, binding_usage, connector_usage = [(a.pilot_root / path).read_text(encoding="utf-8") for path in paths]
    compact = lambda value: re.sub(r"\s+", "", value)
    rules = {
        "ConnectionUsage": ['int numEnds = TypeUtil.getOwnedEndFeaturesOf(getTarget()).size();', 'return numEnds != 2? getDefaultSupertype("base"): getDefaultSupertype("binary");'],
        "TransitionUsage": ['return isStateTransition()? getDefaultSupertype("stateTransition"): isActionTransition()? getDefaultSupertype("actionTransition"): getDefaultSupertype("base");', 'return target.isComposite() && (owningType instanceof ActionDefinition || owningType instanceof ActionUsage) && !(target.getSource() instanceof StateUsage);', 'return target.isComposite() && (owningType instanceof StateDefinition || owningType instanceof StateUsage) && target.getSource() instanceof StateUsage;'],
    }
    for kind, code in [("ConnectionUsage", connection), ("TransitionUsage", transition)]:
        if any(compact(rule) not in compact(code) for rule in rules[kind]):
            raise ValueError(f"Unsupported {kind} default selection rule")
    guards = [(binding_usage, "class BindingConnectorAsUsageAdapter extends ConnectorAsUsageAdapter"), (connector_usage, "class ConnectorAsUsageAdapter extends UsageAdapter"),(feature, 'return getDefaultSupertype(hasStructureType()? isSubobject()? "subobject": "object": hasClassType()? isSuboccurrence()? "suboccurrence": isPortion()? "portion": "occurrence": hasDataType()? "dataValue": "base");'),(succession_usage, "class SuccessionAsUsageAdapter extends SuccessionAdapter"),
        (connector, 'return hasStructureType()? numEnds != 2? getDefaultSupertype("object"): getDefaultSupertype("binaryObject"): numEnds != 2? getDefaultSupertype("base"): getDefaultSupertype("binary");'),
        (feature, 'return feature.getOwnedTyping().stream().map(FeatureTyping::getType).anyMatch(Structure.class::isInstance) || TypeUtil.getImplicitGeneralTypesFor(feature, SysMLPackage.Literals.FEATURE_TYPING).stream().anyMatch(Structure.class::isInstance);'),
        (binding_impl, 'class BindingConnectorImpl extends ConnectorImpl'),
        (succession_impl, 'class SuccessionImpl extends ConnectorImpl'),
        (feature, 'result != null && target.getOwnedSpecialization().isEmpty() && target.getDirection() == null'),
    ]
    guards.append((feature, "if (FeatureUtil.isOwnedCrossFeature(target)) { for (Type type: ((Feature)owner).getType()) { addImplicitGeneralType(SysMLPackage.eINSTANCE.getFeatureTyping(), type); }"))
    if any(compact(rule) not in compact(code) for code, rule in guards):
        raise ValueError("Unsupported connector/feature default predicate or inheritance")
    selections = {"ConnectionUsage": ["base", "binary"], "TransitionUsage": ["base", "stateTransition", "actionTransition"]}
    selections.update({kind: ["base", "binary", "object", "binaryObject"] for kind in ["Connector", "BindingConnector", "Succession"]})
    selections["BindingConnectorAsUsage"] = ["base", "object", "occurrence", "portion", "dataValue"]
    selections["SuccessionAsUsage"] = ["base", "binary", "binaryObject"]
    defaults = {}
    for kind, selectors in selections.items():
        defaults[kind] = {}
        for selector in selectors:
            values = re.findall(r'put\(' + kind + r'Impl.class,\s*"' + selector + r'",\s*"([^"\n]+)"\);', table)
            if not values and kind == "BindingConnectorAsUsage":
                values = re.findall(r'put\(FeatureImpl.class,\s*"' + selector + r'",\s*"([^"\n]+)"\);', table)
            if not values and kind in ("BindingConnector", "Succession"):
                values = re.findall(r'put\(ConnectorImpl.class,\s*"' + selector + r'",\s*"([^"\n]+)"\);', table)
            if len(values) != 1:
                raise ValueError(f"Expected one {kind}.{selector} mapping")
            defaults[kind][selector] = values[0]
    result = {"schema_version": 1, "source": {"pilot_commit": git("rev-parse", "HEAD"), "pilot_dirty": False, "files": {path: hashlib.sha256((a.pilot_root / path).read_bytes()).hexdigest() for path in paths}, "extractor": Path(__file__).name}, "usage_defaults": defaults}
    if a.check:
        if result != json.loads(a.out.read_text(encoding="utf-8")):
            raise ValueError("Conditional usage default extraction drift")
    else:
        a.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(f"{len(defaults)} conditional usage defaults verified")

if __name__ == "__main__":
    main()
