"""Extract semantic annotations and delegate provenance from Pilot's runtime Ecore.

This is an additive source inventory, not an OCL/Java interpreter or compliance
claim. GenModel documentation (including embedded formulas) remains documentation.
The existing metamodel extract remains the structural-schema source.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET

PINNED_COMMIT = "692170b71867353b8f90341e61556f49a5beb0e5"
ECORE = "http://www.eclipse.org/emf/2002/Ecore"
GENMODEL = "http://www.eclipse.org/emf/2002/GenModel"
XSI = "http://www.w3.org/2001/XMLSchema-instance"
SYSML_DELEGATE = "http://www.omg.org/spec/SysML"
MODEL = "org.omg.sysml.model/src/main/resources/model/SysML.ecore"
MODEL_COPY = "org.omg.sysml/model/SysML.ecore"
GENMODEL_PATH = "org.omg.sysml.model/src/main/resources/model/SysML.genmodel"
PLUGIN = "org.omg.sysml/plugin.xml"
LOGIC = "org.omg.sysml.logic/src/main/java/org/omg/sysml"
GRAMMARS = (
    "org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext",
    "org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext",
    "org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/KerMLExpressions.xtext",
)


def local_name(value: str) -> str:
    return value.rsplit("}", 1)[-1]


def children(node: ET.Element, tag: str) -> list[ET.Element]:
    return [child for child in node if local_name(child.tag) == tag]


def raw_xml(node: ET.Element) -> dict:
    """Retain semantic XML content, preserving child order and all attributes."""
    result = {"tag": node.tag, "attributes": dict(sorted(node.attrib.items()))}
    if node.text and node.text.strip():
        result["text"] = node.text
    if len(node):
        result["children"] = [raw_xml(child) for child in node]
    return result


def annotations(node: ET.Element) -> list[dict]:
    result = []
    for annotation in children(node, "eAnnotations"):
        keys = [detail.get("key") for detail in children(annotation, "details")]
        if len(keys) != len(set(keys)):
            raise ValueError("duplicate annotation detail key")
        row = raw_xml(annotation)
        row["interpretation"] = (
            "documentation_only_not_executable"
            if annotation.get("source") == GENMODEL
            else "uninterpreted_annotation_not_executed"
        )
        result.append(row)
    return result


def ecore_type(node: ET.Element) -> str:
    return node.get(f"{{{XSI}}}type", local_name(node.tag)).rsplit(":", 1)[-1]


def annotation_details(node: ET.Element, source: str) -> dict[str, str]:
    details = {}
    for annotation in children(node, "eAnnotations"):
        if annotation.get("source") == source:
            for detail in children(annotation, "details"):
                key = detail.get("key")
                if key in details:
                    raise ValueError(f"duplicate annotation detail {source}: {key}")
                details[key] = detail.get("value", "")
    return details


def type_signature(node: ET.Element, root_uri: str, references: dict[str, str]) -> str:
    generic = children(node, "eGenericType")
    if generic:
        return json.dumps(raw_xml(generic[0]), sort_keys=True, separators=(",", ":"))
    value = node.get("eType", "void").split()[-1]
    return references.get(value, root_uri + value) if value.startswith("#") else value


def extract_model(data: bytes) -> dict:
    """Parse XML rather than generated Java; reject colliding semantic IDs."""
    root = ET.fromstring(data)
    if local_name(root.tag) != "EPackage" or not root.get("nsURI"):
        raise ValueError("expected EPackage with nsURI")
    root_uri = root.attrib["nsURI"]
    records = []
    packages = []
    references = {}

    def index_classifiers(package: ET.Element, prefix: str = "") -> None:
        uri = package.get("nsURI", "")
        for classifier in children(package, "eClassifiers"):
            name = classifier.get("name", "")
            reference = "#//" + prefix + name
            if reference in references:
                raise ValueError(f"duplicate classifier reference: {reference}")
            references[reference] = uri + "#//" + name
        for nested in children(package, "eSubpackages"):
            index_classifiers(nested, prefix + nested.get("name", "") + "/")

    index_classifiers(root)
    seen_ids = set()
    seen_package_uris = set()

    def add(record: dict) -> dict:
        if record["id"] in seen_ids:
            raise ValueError(f"duplicate semantic identifier: {record['id']}")
        seen_ids.add(record["id"])
        records.append(record)
        return record

    def describe(node: ET.Element, identifier: str, kind: str, owner: str) -> dict:
        return add({
            "id": identifier,
            "kind": kind,
            "owner": owner,
            "name": node.get("name", ""),
            "annotations": annotations(node),
            "source_attributes": dict(sorted(node.attrib.items())),
        })

    def visit_package(package: ET.Element, parent: str | None = None, prefix: str = "") -> None:
        uri = package.get("nsURI")
        if not uri or uri in seen_package_uris:
            raise ValueError(f"missing or duplicate package nsURI: {uri}")
        seen_package_uris.add(uri)
        package_id = uri + "#/"
        metadata = annotation_details(package, ECORE)
        packages.append({
            "id": package_id, "name": package.get("name", ""), "ns_uri": uri,
            "ns_prefix": package.get("nsPrefix", ""), "parent": parent,
            "delegate_uris": {
                key: metadata.get(key + "Delegates", "").split()
                for key in ("setting", "invocation", "validation")
            },
        })
        describe(package, package_id, "EPackage", parent or "")
        for classifier in children(package, "eClassifiers"):
            name = classifier.get("name")
            if not name:
                raise ValueError("classifier without a name")
            identifier = uri + "#//" + name
            entry = describe(classifier, identifier, ecore_type(classifier), package_id)
            entry["source_fragment"] = "#//" + prefix + name
            entry["super_types"] = classifier.get("eSuperTypes", "").split()
            for generic in children(classifier, "eGenericSuperTypes"):
                entry.setdefault("generic_super_types", []).append(raw_xml(generic))
            for feature in children(classifier, "eStructuralFeatures"):
                feature_id = identifier + "/" + feature.attrib["name"]
                record = describe(feature, feature_id, ecore_type(feature), identifier)
                # Only the derivation flags are repeated from the structural extract.
                record["derivation_flags"] = {
                    key: feature.get(key, "false") == "true"
                    for key in ("derived", "volatile", "transient")
                }
            for operation in children(classifier, "eOperations"):
                parameters = children(operation, "eParameters")
                signature = operation.attrib["name"] + "(" + ",".join(
                    type_signature(parameter, root_uri, references) for parameter in parameters
                ) + ")"
                operation_id = identifier + "/operation:" + signature
                record = describe(operation, operation_id, "EOperation", identifier)
                record["signature"] = signature
                # Includes result bounds, exception types, ordered/unique flags,
                # generic types, type parameters, and ordered parameter metadata.
                record["definition"] = {
                    "attributes": dict(sorted(operation.attrib.items())),
                    "children": [raw_xml(child) for child in operation
                                 if local_name(child.tag) != "eAnnotations"],
                }
                for parameter in parameters:
                    parameter_id = operation_id + "/parameter:" + parameter.attrib["name"]
                    describe(parameter, parameter_id, "EParameter", operation_id)
            for literal in children(classifier, "eLiterals"):
                record = describe(
                    literal, identifier + "/literal:" + literal.attrib["name"],
                    "EEnumLiteral", identifier,
                )
                record["attributes"] = dict(sorted(literal.attrib.items()))
            for index, parameter in enumerate(children(classifier, "eTypeParameters")):
                name = parameter.get("name", str(index))
                record = describe(parameter, identifier + "/type-parameter:" + name,
                                  "ETypeParameter", identifier)
                record["definition"] = raw_xml(parameter)
        for nested in children(package, "eSubpackages"):
            visit_package(nested, package_id, prefix + nested.get("name", "") + "/")

    visit_package(root)
    return {
        "source_tree": raw_xml(root),
        "root_namespace_uri": root_uri,
        "packages": sorted(packages, key=lambda row: row["id"]),
        "elements": sorted(records, key=lambda row: row["id"]),
    }


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_record(root: Path, path: Path) -> dict:
    return {"path": path.relative_to(root).as_posix(), "sha256": digest(path)}


def inventory_delegates(root: Path) -> list[dict]:
    """Inventory filenames, not Java bodies or inferred executable semantics."""
    result = []
    seen = set()
    directory = root / LOGIC / "delegate"
    for path in sorted(directory.rglob("*.java")):
        relative = path.relative_to(directory)
        category = relative.parts[0]
        if category not in ("setting", "invocation"):
            continue
        identifier = f"org.omg.sysml.delegate.{category}.{path.stem}"
        if identifier in seen:
            raise ValueError(f"duplicate delegate class identifier: {identifier}")
        seen.add(identifier)
        row = {"class": identifier, "kind": category, **source_record(root, path),
               "algorithm_status": "source_inventoried_not_translated_or_executed"}
        suffix = f"_{category.capitalize()}Delegate"
        if path.stem.endswith(suffix) and "_" in path.stem[:-len(suffix)]:
            classifier, member = path.stem[:-len(suffix)].split("_", 1)
            row["filename_convention"] = {"classifier": classifier, "member": member}
        result.append(row)
    return result


def bind_delegates(model: dict, sources: list[dict]) -> list[dict]:
    packages = {row["id"]: row for row in model["packages"]}
    elements = {row["id"]: row for row in model["elements"]}
    classifiers = {row["id"]: row for row in model["elements"] if row["kind"] == "EClass"}
    by_fragment = {row["source_fragment"]: row["id"] for row in classifiers.values()}
    by_name = {}
    for row in classifiers.values():
        by_name.setdefault(row["name"], []).append(row["id"])
    bindings = []

    def subtype(candidate_id: str, base_id: str, visited: set[str]) -> bool:
        if candidate_id == base_id:
            return True
        if candidate_id in visited:
            return False
        visited = visited | {candidate_id}
        candidate = classifiers[candidate_id]
        # Ecore local references are relative to the containing resource's root.
        root_uri = model["root_namespace_uri"]
        for parent in candidate.get("super_types", []):
            parent_id = (by_fragment.get(parent, root_uri + parent)
                         if parent.startswith("#") else parent)
            if parent_id in classifiers and subtype(parent_id, base_id, visited):
                return True
        return False

    for element in model["elements"]:
        kind = ("setting" if element["kind"] in ("EReference", "EAttribute")
                else "invocation" if element["kind"] == "EOperation" else None)
        if kind is None:
            continue
        owner = elements[element["owner"]]
        package = packages[owner["owner"]]
        marker_uris = {row["attributes"].get("source") for row in element["annotations"]}
        for uri in package["delegate_uris"][kind]:
            if uri not in marker_uris:
                continue
            row = {"element": element["id"], "kind": kind, "delegate_uri": uri,
                   "binding_status": "marker_only_unresolved_factory"}
            # This mapping is the pinned Pilot factory filename convention, not
            # an assumption that arbitrary delegate URIs use Pilot's factories.
            if uri == SYSML_DELEGATE:
                candidates = []
                for source in sources:
                    convention = source.get("filename_convention", {})
                    if source["kind"] != kind or convention.get("member") != element["name"]:
                        continue
                    candidate_ids = by_name.get(convention.get("classifier"), [])
                    if kind == "setting":
                        applies = owner["id"] in candidate_ids
                    else:
                        applies = any(subtype(candidate, owner["id"], set())
                                      for candidate in candidate_ids)
                    if applies:
                        candidates.append(source["class"])
                row["candidate_source_classes"] = sorted(candidates)
                if kind == "setting":
                    row["binding_status"] = (
                        "custom_setting_delegate_source" if candidates
                        else "default_setting_delegate_fallback_source"
                    )
                    if not candidates:
                        row["fallback_class"] = (
                            "org.omg.sysml.delegate.setting.DefaultDerivedPropertySettingDelegate"
                        )
                else:
                    row["binding_status"] = "dynamic_invocation_candidates_not_selected"
                    row["dispatch_source_class"] = (
                        "org.omg.sysml.delegate.invocation.OperationInvocationDelegateSelector"
                    )
            bindings.append(row)
    return sorted(bindings, key=lambda row: (row["element"], row["kind"], row["delegate_uri"]))


def extract_registrations(path: Path) -> list[dict]:
    root = ET.parse(path).getroot()
    result = []
    for extension in root.findall("extension"):
        point = extension.get("point", "")
        if point in (
            "org.eclipse.emf.ecore.setting_delegate",
            "org.eclipse.emf.ecore.invocation_delegate",
            "org.eclipse.emf.ecore.generated_package",
        ):
            result.append({"extension_point": point,
                           "declarations": [raw_xml(child) for child in extension]})
    return result


def verify_pilot(root: Path, expected_commit: str) -> str:
    commit = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True, encoding="utf-8"
    ).strip()
    if commit != expected_commit:
        raise ValueError(f"Pilot commit mismatch: expected {expected_commit}, got {commit}")
    dirty = subprocess.check_output(
        ["git", "-C", str(root), "status", "--porcelain", "--untracked-files=all"],
        text=True, encoding="utf-8",
    ).strip()
    if dirty:
        raise ValueError("Pilot sources are modified or untracked")
    return commit


def build_extract(root: Path, commit: str) -> dict:
    model_path = root / MODEL
    model = extract_model(model_path.read_bytes())
    if model_path.read_bytes() != (root / MODEL_COPY).read_bytes():
        raise ValueError("runtime and Eclipse plugin Ecore copies differ")
    model_uris = {row["ns_uri"] for row in model["packages"]}
    grammar_imports = []
    source_paths = [model_path, root / MODEL_COPY, root / GENMODEL_PATH, root / PLUGIN]
    for grammar in GRAMMARS:
        path = root / grammar
        # Deliberately bounded to import declarations; the grammar AST exporter
        # is responsible for parsing the complete Xtext language.
        matches = re.findall(r'(?m)^import\s+"([^"]+)"\s+as\s+(\w+)', path.read_text(encoding="utf-8"))
        matching = [{"uri": uri, "alias": alias} for uri, alias in matches if uri in model_uris]
        if not matching:
            raise ValueError(f"grammar does not import runtime model: {grammar}")
        grammar_imports.append({"path": grammar, "model_imports": matching})
        source_paths.append(path)
    delegates = inventory_delegates(root)
    source_paths.extend(root / row["path"] for row in delegates)
    source_paths.append(root / LOGIC / "logic/SysMLLogicStandaloneSetup.java")
    bindings = bind_delegates(model, delegates)
    return {
        "schema": "dev.mercurio.ecore-semantics-extract.v1",
        "scope": {
            "purpose": "additive semantic-source inventory; not runtime conformance",
            "documentation": "raw XML-decoded text; embedded formulas are not executable contracts",
            "annotations": "retained without evaluating any annotation language",
            "delegates": "custom source provenance and binding candidates; Java algorithms are not translated",
            "invocation_dispatch": "runtime-class and superclass selection remains procedural",
            "structural_schema": "source_tree and source_attributes preserve explicit XML; metamodel.extract.json supplies normalized structural metadata",
            "defaults": "omitted XML attributes remain omitted; effective EMF defaults require a resolved model consumer",
        },
        "source": {
            "pilot_commit": commit,
            "profile_id": "sysml-2.0-pilot-2026-08",
            "extractor": {"path": "tools/extract_ecore_semantics.py",
                          "sha256": digest(Path(__file__))},
            "source_files": [source_record(root, path) for path in sorted(set(source_paths))],
            "runtime_ecore": MODEL,
            "identical_plugin_ecore": MODEL_COPY,
        },
        "grammar_model_imports": grammar_imports,
        "plugin_registrations": extract_registrations(root / PLUGIN),
        **model,
        "delegate_sources": delegates,
        "delegate_bindings": bindings,
        "counts": {
            "classifiers": sum(row["kind"] in ("EClass", "EEnum", "EDataType") for row in model["elements"]),
            "features": sum(row["kind"] in ("EReference", "EAttribute") for row in model["elements"]),
            "operations": sum(row["kind"] == "EOperation" for row in model["elements"]),
            "annotations": sum(len(row["annotations"]) for row in model["elements"]),
            "documentation_annotations": sum(
                annotation["interpretation"] == "documentation_only_not_executable"
                for row in model["elements"] for annotation in row["annotations"]
            ),
            "delegate_sources": len(delegates),
            "delegate_bindings": len(bindings),
        },
    }


def render(document: dict) -> bytes:
    return (json.dumps(document, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode("utf-8")


def write_or_check(document: dict, out: Path, check: bool) -> None:
    expected = render(document)
    if check:
        if not out.exists() or out.read_bytes() != expected:
            raise ValueError(f"Ecore semantic-source extract drift: {out}")
    else:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(expected)


def main() -> None:
    repo = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pilot-root", type=Path,
                        default=repo.parent / "target/upstream/SysML-v2-Pilot-Implementation")
    parser.add_argument("--expected-commit", default=PINNED_COMMIT)
    parser.add_argument("--out", type=Path,
                        default=repo / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-semantics.extract.json")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    commit = verify_pilot(args.pilot_root, args.expected_commit)
    document = build_extract(args.pilot_root, commit)
    write_or_check(document, args.out, args.check)
    print(json.dumps({"verified" if args.check else "written": str(args.out),
                      **document["counts"]}, sort_keys=True))


if __name__ == "__main__":
    main()

