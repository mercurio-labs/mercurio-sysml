"""Extract all leading Pilot checkAllTypes/checkOneType predicates, guards, and Ecore ancestry.
Direct reference-kind predicates and the bounded flow-end check are also extracted.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

SOURCE = "org.omg.sysml.xtext/src/org/omg/sysml/xtext/validation/SysMLValidator.xtend"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("pilot-root", "metamodel", "out"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    def git(*command):
        return subprocess.check_output(["git", "-C", str(args.pilot_root), *command], text=True).strip()
    if git("status", "--porcelain"):
        raise ValueError("Pilot checkout must be clean")
    source_path = args.pilot_root / SOURCE
    source = source_path.read_text(encoding="utf-8")
    constants = dict(re.findall(r'public static val (\w+)\s*=\s*"([^"\n]*)"', source))
    # Retain offsets while ignoring comments. Fail closed if a type-check
    # invocation is not the supported leading call/negative-instance guard shape.
    clean = re.sub(r"/\*.*?\*/|//[^\n]*", lambda match: "".join("\n" if char == "\n" else " " for char in match[0]), source, flags=re.S)
    checks = []
    invocations = list(re.finditer(r"(?<!def boolean )\bcheck(?:AllTypes|OneType)\(\w+,", clean))
    methods = list(re.finditer(r"@Check\s+def (check\w+)\((\w+) (\w+)\)\s*\{", clean))
    for call in invocations:
        method = next((item for item in reversed(methods) if item.end() <= call.start()), None)
        if method is None:
            raise ValueError("type predicate outside an active check")
        method_name, context, variable = method.groups()
        prefix = clean[method.end():call.start()].strip()
        excluded = []
        if prefix:
            guard = re.fullmatch(r"if\s*\(\s*!\s*\(\s*(.*?)\s*\)\s*\)", prefix, flags=re.S)
            if guard is None:
                raise ValueError("unsupported type predicate control flow: " + method_name)
            for condition in re.split(r"\s*\|\|\s*", guard[1]):
                term = re.fullmatch(re.escape(variable) + r"\s+instanceof\s+([\w.]+)", condition.strip())
                if term is None:
                    raise ValueError("unsupported type guard: " + condition)
                excluded.append(term[1].rsplit(".", 1)[-1])
        operation = call[0].split("(", 1)[0]
        arguments = re.match(re.escape(operation) + r"\(" + re.escape(variable) + r",\s*([\w.]+),\s*(\w+),\s*([\w.]+),\s*(\w+)\)", clean[call.start():])
        if arguments is None:
            raise ValueError("unsupported type predicate arguments: " + method_name)
        required, message, reference, issue = arguments.groups()
        checks.append({"quantifier": "all" if operation == "checkAllTypes" else "one", "context": context, "excluded_contexts": excluded, "required_type": required.rsplit(".", 1)[-1], "message": constants[message], "issue": constants[issue], "source_file": SOURCE, "line": clean[:call.start()].count("\n")+1, "reference": reference, "method": method_name})
    if not checks:
        raise ValueError("no active type predicates found")
    # Match each complete active body, including the PerformAction exclusions.
    # Unsupported new statements fail closed rather than disappearing from extraction.
    reference_checks = []
    reference_methods = ["checkIncludeUseCaseUsage", "checkPerformActionUsage", "checkExhibitStateUsage", "checkAssertConstraintUsage", "checkSatisfyRequirementUsage"]
    for method_name in reference_methods:
        method = re.search(r"@Check\s+def " + method_name + r"\((\w+) (\w+)\)\s*\{", clean)
        if method is None:
            raise ValueError("missing reference validator: " + method_name)
        context, variable = method.groups()
        end, depth = method.end(), 1
        while depth and end < len(clean):
            depth += (clean[end] == "{") - (clean[end] == "}")
            end += 1
        body = clean[method.end():end-1]
        call = re.search(r"checkReferenceType\((\w+),\s*(\w+),\s*(\w+),\s*(\w+)\)", body)
        if call is None or call[1] != variable:
            raise ValueError("unsupported reference predicate: " + method_name)
        argument, required, message, issue = call.groups()
        compact = lambda value: re.sub(r"\s+", "", value)
        expected = compact(call[0])
        excluded = []
        if method_name == "checkPerformActionUsage":
            excluded = ["ExhibitStateUsage", "IncludeUseCaseUsage"]
            expected = "if(!(" + variable + "instanceofExhibitStateUsage||" + variable + "instanceofIncludeUseCaseUsage)){" + expected + "}"
        if compact(body) != expected:
            raise ValueError("unsupported reference validator body: " + method_name)
        reference_checks.append({"context": context, "excluded_contexts": excluded, "required_type": required, "message": constants[message], "issue": constants[issue], "source_file": SOURCE, "line": clean[:method.start()].count("\n") + 1, "method": method_name})
    # A conservative owned-end branch of checkFlowDefinition. Inherited-end
    # collection and complete diagnostic multiplicity remain explicitly untraced.
    flow = re.search(r"@Check\s+def checkFlowDefinition\(FlowDefinition cdef\)\s*\{", clean)
    if flow is None:
        raise ValueError("missing flow definition check")
    end = flow.end()
    depth = 1
    while depth and end < len(clean):
        depth += (clean[end] == "{") - (clean[end] == "}")
        end += 1
    body = re.sub(r"\s+", "", clean[flow.end():end-1])
    expected = 'valends=cdef.endFeatureif(ends.size>2){valownedEnds=cdef.ownedEndFeatureif(ownedEnds.size<=2){error(INVALID_FLOW_DEFINITION_END_MSG,cdef,null,INVALID_FLOW_DEFINITION_END)}else{for(vari=2;i<ends.size;i++){error(INVALID_FLOW_DEFINITION_END_MSG,ends.get(i),null,INVALID_FLOW_DEFINITION_END)}}}'
    if body != expected:
        raise ValueError("unsupported flow end bound check shape")
    owned_end_checks = [{"context": "FlowDefinition", "maximum": 2, "issue": constants["INVALID_FLOW_DEFINITION_END"], "message": constants["INVALID_FLOW_DEFINITION_END_MSG"], "method": "checkFlowDefinition", "source_file": SOURCE, "line": clean[:flow.start()].count("\n") + 1, "coverage": "Owned-end overflow only; inherited end collection and all diagnostic locations remain open."}]
    metamodel = json.loads(args.metamodel.read_text(encoding="utf-8"))
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    result = {"schema": "dev.mercurio.release-type-checks.v1", "source": {"pilot_commit": git("rev-parse", "HEAD"), "pilot_dirty": False, "validator_sha256": digest(source_path), "metamodel_sha256": digest(args.metamodel), "typing_algorithm_sources": {path: digest(args.pilot_root / path) for path in ["org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/FeatureAdapter.java", "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/TypeUtil.java"]}, "selection": "All active leading checkAllTypes/checkOneType invocations and their negative instance guards; complete bodies of five direct reference-kind methods (including PerformAction exclusions); owned flow-end overflow. Computed basic-feature chains and inherited flow ends remain outside native coverage."}, "checks": checks, "reference_checks": reference_checks, "owned_end_checks": owned_end_checks, "generalizations": [{"specific": x["specific"], "general": x["general"]} for x in metamodel["generalizations"]]}
    if args.check:
        if result != json.loads(args.out.read_text(encoding="utf-8")):
            raise ValueError("type-check artifact drift")
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
