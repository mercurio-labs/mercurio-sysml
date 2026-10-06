"""Time-boxed finite prediction graph evaluation; does not promote native support."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
from export_pilot_prediction import ROOT, PIN, GRAMMAR, GRAMMARS, HELPER, decision_requests, digest, PredicateContext, guarded_nodes, predicate_signature, source_signature

NFA_HELPER = HELPER.with_name("PilotNfaExporter.java")


def validate_graph(graph):
    states = graph["states"]
    for identity, state in states.items():
        if state["stop"] and state["edges"]:
            raise ValueError("Rule stop must return through its call stack")
        for edge in state["edges"] + state.get("follow_edges", []):
            if str(edge["target"]) not in states:
                raise ValueError("Dangling graph target")
            kind = edge["kind"]
            if kind == "call" and str(edge["follow"]) not in states:
                raise ValueError("Dangling call return")
            if kind not in {"call", "epsilon", "action", "predicate", "tokens"}:
                raise ValueError("Unknown edge semantics")
            if kind == "tokens" and not edge["symbols"]:
                raise ValueError("Empty token edge")
    for decision in graph["decisions"].values():
        if not decision["entries"] or any(str(entry) not in states for entry in decision["entries"]):
            raise ValueError("Invalid decision entry")



def bind_predicates(graph, structured, context):
    rules = {r["id"]: r for g in structured["grammars"] for r in g["rules"]}
    effective = structured["resolution_contexts"][context]["effective_rules"]
    predicates = PredicateContext(structured, context)
    grouped = {}
    for state in graph["states"].values():
        for edge in state["edges"] + state.get("follow_edges", []):
            if edge["kind"] == "predicate":
                condition = edge["condition"]
                for key in ("source_id", "first_set", "binding_error", "occurrence_binding"):
                    condition.pop(key, None)
                key = (state.get("rule", ""), tuple(condition.get("signature", [])))
                grouped.setdefault(key, []).append(condition)
    for (name, signature), conditions in grouped.items():
        rule = effective.get(name.removeprefix("rule"))
        guards = list(guarded_nodes(rules[rule], predicates)) if rule else []
        matches = {guard["id"]:guard for guard in guards
                   if predicate_signature(guard, predicates) == list(signature)}
        if len(matches) == 1:
            bindings = [(condition, next(iter(matches.values()))) for condition in conditions]
        elif len(matches) > 1:
            positions = [(condition.get("source_line"), condition.get("source_column")) for condition in conditions]
            offsets = [guard.get("span", {}).get("offset") for guard in matches.values()]
            if (graph.get("rule_signatures", {}).get(name) != source_signature(rules[rule]["fields"]["alternatives"], predicates)
                    or len(conditions) != len(matches) or len(set(positions)) != len(positions)
                    or any(line is None or line <= 0 or column is None or column < 0 for line, column in positions)
                    or any(offset is None for offset in offsets) or len(set(offsets)) != len(offsets)):
                for condition in conditions:
                    condition["binding_error"] = "Unproven repeated guard occurrence"
                continue
            ordered = sorted(conditions, key=lambda c: (c["source_line"], c["source_column"]))
            bindings = list(zip(ordered, sorted(matches.values(), key=lambda g: g["span"]["offset"])))
            for index, (condition, _) in enumerate(bindings):
                condition["occurrence_binding"] = {"index":index, "count":len(bindings), "whole_rule_verified":True}
        else:
            for condition in conditions:
                condition["binding_error"] = "Expected a source guard, found none"
            continue
        for condition, guard in bindings:
            condition["source_id"] = guard["id"]
            condition["first_set"] = bool(guard["fields"].get("firstSetPredicated"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-bin", type=Path, required=True)
    parser.add_argument("--pilot", type=Path, default=ROOT.parent / "target/support-2026-08/pilot-pinned-2026-08")
    parser.add_argument("--output", type=Path, default=GRAMMAR.with_name("xtext-prediction-nfa.experimental.json"))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    def git(*command):
        return subprocess.check_output(["git", "-C", str(args.pilot), *command])
    if git("rev-parse", "HEAD").decode().strip() != PIN:
        raise ValueError("Unexpected Pilot pin")
    structured = json.loads(GRAMMAR.read_text(encoding="utf-8"))
    if structured["provenance"]["pilot_revision"] != PIN:
        raise ValueError("Structured grammar pin mismatch")
    for path, expected in structured["provenance"]["sources_sha256"].items():
        if path.endswith(".xtext") and digest((args.pilot / path).read_bytes()) != expected:
            raise ValueError("Changed structured source: " + path)
    tool = args.pilot / "org.omg.sysml.xtext/.antlr-generator-3.2.0-patch.jar"
    runtime = args.pilot / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    classpath = os.pathsep.join(map(str, [tool, runtime]))
    suffix = ".exe" if os.name == "nt" else ""
    scope_sources = {}
    for path in [
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/scoping/KerMLScopeProvider.xtend",
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/scoping/KerMLScope.xtend",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/NamespaceAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/NamespaceImportAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/MembershipImportAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/delegate/invocation/Membership_isDistinguishableFrom_InvocationDelegate.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/PackageAdapter.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/adapter/LibraryPackageAdapter.java",
        "org.omg.sysml.xtext/src/org/omg/sysml/xtext/scoping/SysMLScopeProvider.xtend",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/NamespaceUtil.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/ElementUtil.java",
        "org.omg.sysml.logic/src/main/java/org/omg/sysml/util/FeatureUtil.java",
    ]:
        content = (args.pilot / path).read_bytes().replace(b"\r\n", b"\n")
        if content != git("show", PIN + ":" + path).replace(b"\r\n", b"\n"):
            raise ValueError("Changed pinned scope dependency: " + path)
        scope_sources[path] = digest(content)
    graphs, sources = {}, {}
    with tempfile.TemporaryDirectory(prefix="prediction-nfa-") as temp:
        temp = Path(temp)
        subprocess.run([str(args.java_bin / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", classpath,
                        "-d", str(temp), str(HELPER), str(NFA_HELPER)], check=True, timeout=60)
        for context, (project, package, name) in GRAMMARS.items():
            path = f"{project}/src-gen/{package}/parser/antlr/internal/Internal{name}.g"
            content = (args.pilot / path).read_bytes().replace(b"\r\n", b"\n")
            if content != git("show", PIN + ":" + path).replace(b"\r\n", b"\n"):
                raise ValueError("Changed pinned generated grammar")
            sources[path] = digest(content)
            requests, gaps = decision_requests(structured, context, include_repetitions=True)
            if gaps:
                raise ValueError("Unresolved structured decision requests")
            scope, output = temp / "requests.json", temp / "graph.json"
            scope.write_text(json.dumps(requests), encoding="utf-8")
            subprocess.run([str(args.java_bin / ("java" + suffix)), "-Xmx512m", "-cp", str(temp) + os.pathsep + classpath,
                            "dev.mercurio.pilot.PilotNfaExporter", str(args.pilot / path), str(output), str(scope)],
                           check=True, timeout=60)
            graph = json.loads(output.read_text(encoding="utf-8"))
            bind_predicates(graph, structured, context)
            validate_graph(graph)
            graphs[context] = graph
            print(context, len(graph["states"]), "finite states;", len(graph["decisions"]), "mapped decisions;",
                  len(graph["unsupported"]), "unresolved identities; execution NOT qualified", flush=True)
        probe = HELPER.with_name("PilotNfaProbe.java")
        branch_helper = HELPER.with_name("PilotExpandedDecisionProbe.java")
        subprocess.run([str(args.java_bin / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", classpath,
                        "-d", str(temp), str(probe), str(branch_helper)], check=True, timeout=60)
        controls = temp / "controls.json"
        subprocess.run([str(args.java_bin / ("java" + suffix)), "-Xmx512m", "-cp", str(temp) + os.pathsep + classpath,
                        "dev.mercurio.pilot.PilotNfaProbe", str(controls)], check=True, timeout=60)
        cases = json.loads(controls.read_text(encoding="utf-8"))
        hidden = json.loads(Path(str(controls) + ".hidden.json").read_text(encoding="utf-8"))
        if len(hidden) != 16: raise ValueError("Incomplete hidden-token controls")
        links = json.loads(Path(str(controls) + ".links.json").read_text(encoding="utf-8"))
        disagreements = json.loads(Path(str(controls) + ".scope-disagreements.json").read_text(encoding="utf-8"))
        membership_conflicts = json.loads(Path(str(controls) + ".membership-conflicts.json").read_text(encoding="utf-8"))
        if len(membership_conflicts) != 4:
            raise ValueError("Changed membership conflict controls")
        if len(disagreements) != 1:
            raise ValueError("Changed scope disagreement controls")
        if len(links) != 34 or sum(len(row["links"]) for row in links) != 75:
            raise ValueError("Incomplete link dependency controls")
        models = json.loads(Path(str(controls) + ".models.json").read_text(encoding="utf-8"))
        if len(models) != 27 or sum(row["accepted"] for row in models) != 22:
            raise ValueError("Changed complete-definition model controls: " + repr([(row["source"], row["accepted"]) for row in models]))
        conflicts = json.loads(Path(str(controls) + ".conflicts.json").read_text(encoding="utf-8"))
        negatives = json.loads(Path(str(controls) + ".negative.json").read_text(encoding="utf-8"))
        if len(negatives) != 7 or any(row["accepted"] for row in negatives):
            raise ValueError("Malformed control unexpectedly accepted: " + repr([row for row in negatives if row["accepted"]]))
        if len(cases) != 8:
            raise ValueError("Incomplete blocked-decision controls")
    result = {"schema": "dev.mercurio.experimental-prediction-nfa.v1", "contexts": graphs, "hidden_token_controls": hidden, "blocked_decision_controls": cases, "negative_controls": negatives, "predicate_conflict_controls": conflicts, "complete_model_controls": models, "link_dependency_controls": links, "scope_disagreement_controls": disagreements, "membership_conflict_controls": membership_conflicts,
              "provenance": {"pilot_revision": PIN, "source_sha256": sources, "scope_sources_sha256": scope_sources, "grammar_sha256": digest(GRAMMAR.read_bytes()),
                             "helper_sha256": digest(NFA_HELPER.read_bytes()), "signature_helper_sha256": digest(HELPER.read_bytes()),
                             "probe_sha256": digest(probe.read_bytes()), "branch_helper_sha256": digest(branch_helper.read_bytes()),
                             "driver_sha256": digest(Path(__file__).read_bytes()), "tool_sha256": digest(tool.read_bytes()), "runtime_sha256": digest(runtime.read_bytes())}}
    rendered = json.dumps(result, separators=(",", ":"), sort_keys=True) + "\n"
    if args.check:
        if args.output.read_text(encoding="utf-8") != rendered:
            raise ValueError("Stale experimental finite graph")
    else:
        args.output.write_text(rendered, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
