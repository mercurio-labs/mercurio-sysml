"""Extract and guard the Pilot semantic metadata base feature and adapter policy."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pilot-root", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    def git(*command):
        return subprocess.check_output(["git", "-C", str(args.pilot_root), *command], text=True).strip()
    if git("status", "--porcelain"):
        raise ValueError("Pilot checkout must be clean")
    root = "org.omg.sysml.logic/src/main/java/org/omg/sysml/"
    paths = [root + "adapter/" + name + "Adapter.java" for name in ("Type", "Classifier", "Feature")]
    paths.append(root + "util/ImplicitGeneralizationMap.java")
    texts = [(args.pilot_root / path).read_text(encoding="utf-8") for path in paths]
    compact = [re.sub(r"\s+", "", text) for text in texts]
    guards = [
        (0, "filter(f->TypeUtil.specializes(f,getBaseTypeFeature(metadataFeature)))"),
        (0, "map(FeatureUtil::getValueExpressionFor)"),
        (0, "map(EvaluationUtil::getMetaclassReferenceOf)"),
        (1, "flatMap(type->typeinstanceofFeature?((Feature)type).getType().stream():Stream.of(type))"),
        (1, "filter(Classifier.class::isInstance)"),
        (2, "filter(Feature.class::isInstance)"),
    ]
    for index, fragment in guards:
        if fragment not in compact[index]:
            raise ValueError("unsupported semantic metadata adapter shape: " + paths[index])
    anchors = set(re.findall(r'"(Metaobjects::[^"\n]+::baseType)"', texts[3]))
    if len(anchors) != 1:
        raise ValueError("missing or ambiguous semantic base feature")
    result = {"schema_version": 1, "source": {"pilot_commit": git("rev-parse", "HEAD"), "pilot_dirty": False,
        "files": {path: hashlib.sha256((args.pilot_root / path).read_bytes()).hexdigest() for path in paths},
        "extractor": "generate_release_semantic_metadata.py"}, "base_feature": anchors.pop(),
        "classifier_base": "feature_types_or_classifier", "feature_base": "feature_only"}
    if args.check:
        if result != json.loads(args.out.read_text(encoding="utf-8")):
            raise ValueError("semantic metadata artifact drift")
    else:
        args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
