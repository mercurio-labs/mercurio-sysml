"""Compare the fixed native value-binding stage against every cached Pilot frame.

Counts, ordered effects, lifecycle flags and getter differences remain visible.
This build-time audit never emits an obligation or family certificate.
"""
from pathlib import Path
import argparse, collections, json
from run_value_result_library_dependency_bundle import ROOT, EV, read, sha, save, run
from audit_value_result_provider_plan import native_identity
from audit_cached_native_dependencies import exact_json

FLAGS = ["is_implied_included", "is_end", "is_variable", "is_composite", "is_portion"]
TARGETS = {"ReferenceSubsetting": "referenced_feature", "Subsetting": "subsetted_feature",
           "Redefinition": "redefined_feature", "FeatureTyping": "type",
           "TypeFeaturing": "featuring_type", "FeatureChaining": "chaining_feature"}
GETTERS = ["type", "featuring_type", "chaining_feature"]


def differences(actual, expected, path=""):
    if type(actual) is not type(expected):
        return [dict(path=path, actual=actual, expected=expected)]
    if isinstance(actual, dict):
        rows = []
        for key in sorted(actual.keys() | expected.keys()):
            child = path + "/" + key
            if key not in actual or key not in expected:
                rows.append(dict(path=child, actual=actual.get(key), expected=expected.get(key), missing=True))
            else:
                rows.extend(differences(actual[key], expected[key], child))
        return rows
    if isinstance(actual, list):
        rows = []
        if len(actual) != len(expected):
            rows.append(dict(path=path + "/length", actual=len(actual), expected=len(expected)))
        for ordinal, (left, right) in enumerate(zip(actual, expected)):
            rows.extend(differences(left, right, path + "/" + str(ordinal)))
        return rows
    return [] if actual == expected else [dict(path=path, actual=actual, expected=expected)]


def roles_for(value):
    binding = value["valuation_id"] + ".implicit.value-binding"
    roles = {"owner": value["owner_id"], "value": value["expression_id"], "connector": binding,
             "end.0": binding + ".end.0", "end.1": binding + ".end.1", "value_chain": binding + ".source"}
    if value["is_initial"]:
        roles["initial_context"] = binding + ".initial-context"
    return roles


def compare(graph, prior_graph, valuations, reference, identities, result_queries, queries):
    index = {node["id"]: node for node in graph}
    prior = {node["id"]: node for node in prior_graph}
    assert len(index) == len(graph) and len(prior) == len(prior_graph)
    fixed = {value["valuation_id"]: value for value in valuations}
    observed_ids = [row["control"]["valuation_id"] for row in reference["observations"]]
    assert len(fixed) == len(observed_ids) == 45 and set(fixed) == set(observed_ids)
    results = {(row["owner_id"], row["field"]): row for row in result_queries}
    reads = {(row["owner_id"], row["field"]): row for row in queries}
    assert len(reads) == len(queries) == 573
    rows = []
    for observation in reference["observations"]:
        value = fixed[observation["control"]["valuation_id"]]
        row = dict(context=value["context"], valuation_id=value["valuation_id"], qualification_certificate=False)
        if not value["eligible_for_binding_stage"]:
            assert observation["status"] == "required_link_negative_not_qualified"
            row.update(status="required_link_negative_not_qualified")
            rows.append(row)
            continue
        assert observation["status"] == "reference_observed"
        assert observation["is_default"] == value["is_default"] and observation["is_initial"] == value["is_initial"]
        original_flags = {}
        for name, identity in [("owner", value["owner_id"]), ("expression", value["expression_id"])]:
            before = prior[identity]["properties"].get("is_implied_included", False)
            after = index[identity]["properties"].get("is_implied_included", False)
            original_flags[name] = dict(native_before=before, native_after=after,
                reference_before=observation[name + "_completion_before"], reference_after=observation[name + "_completion_after"])
            assert before == after
        row["original_lifecycle_flags"] = original_flags
        roles = roles_for(value)
        native_count = int(roles["connector"] in index)
        raw_count = len(observation["bindings"])
        row.update(native_binding_count=native_count, reference_binding_count=raw_count,
                   binding_count_matches=native_count == raw_count)
        if value["is_default"]:
            assert native_count == raw_count == 0
            row.update(status="default_dispatch_stage_matched", exact_stage_match=True)
            rows.append(row)
            continue
        assert native_count == 1 and raw_count > 0
        result = results[(value["expression_id"], "result")]
        assert result["status"] == "query_evaluated" and len(result["targets"]) == 1
        roles["value_result"] = result["targets"][0]["id"]
        source = index[roles["value_chain"]]
        chaining = [index[identity] for identity in source["properties"]["owned_relationship"]]
        assert [relation["properties"]["chaining_feature"] for relation in chaining] == [roles["value"], roles["value_result"]]
        row["result_identity_proof"] = dict(native_result=roles["value_result"], evidence="fresh same-expression result getter and exact physical chain")

        def endpoint(target):
            if "local_role" in target:
                identity = roles[target["local_role"]]
            elif target["resource"].startswith("sysml.library/"):
                identity = native_identity(target["resource"], target["emf_fragment"])
            else:
                key = target["resource"].replace(chr(92), "/") + "#" + target["emf_fragment"]
                assert key in identities, "Unreviewed external endpoint " + key
                identity = identities[key]
            assert identity in index and index[identity]["kind"].rsplit("::", 1)[-1] == target["kind"]
            return identity

        def expected_shape(node):
            shape = {key: node[key] for key in ["kind", *FLAGS, "direction"]}
            shape["effects"] = []
            for effect in node["effects"]:
                assert not effect.get("unassessed_relation"), "Unassessed upstream relationship"
                shape["effects"].append(dict(kind=effect["kind"], is_implied=effect["is_implied"], target_id=endpoint(effect["target"])))
            return shape

        def actual_shape(identity):
            node = index[identity]
            shape = dict(kind=node["kind"].rsplit("::", 1)[-1], direction=node["properties"].get("direction"))
            shape.update({key: node["properties"].get(key, False) for key in FLAGS})
            shape["effects"] = []
            for relation_id in node["properties"].get("owned_relationship", []):
                relation = index[relation_id]; kind = relation["kind"].rsplit("::", 1)[-1]
                if kind.endswith("Membership"):
                    continue
                assert kind in TARGETS and relation["properties"]["owning_related_element"] == identity
                target = relation["properties"][TARGETS[kind]]
                assert target in index
                shape["effects"].append(dict(kind=kind, is_implied=relation["properties"].get("is_implied", False), target_id=target))
            return shape

        frames = []
        getter_expectations = collections.defaultdict(list)
        for ordinal, binding in enumerate(observation["bindings"]):
            role_nodes = {"connector": binding, **{"end." + str(i): end for i, end in enumerate(binding["ends"])}}
            assert len(binding["ends"]) == 2
            for chain in binding["chains"]:
                role = chain["local_role"]
                assert role not in role_nodes
                role_nodes[role] = chain
            wanted_roles = {"connector", "end.0", "end.1", "value_chain"} | ({"initial_context"} if value["is_initial"] else set())
            assert set(role_nodes) == wanted_roles
            actual = {role: actual_shape(roles[role]) for role in role_nodes}
            expected = {role: expected_shape(node) for role, node in role_nodes.items()}
            membership = index[index[roles["connector"]]["properties"]["owning_relationship"]]
            actual["membership"] = membership["kind"].rsplit("::", 1)[-1]
            expected["membership"] = binding["membership"]
            diff = differences(actual, expected)
            frames.append(dict(reference_ordinal=ordinal, exact_ordered_structure_and_flags=not diff, differences=diff))
            for role, node in role_nodes.items():
                for field in GETTERS:
                    getter_expectations[(roles[role], field)].append([endpoint(target) for target in node[field]])
            related = [endpoint(target) for target in binding["related_feature"]]
            assert len(related) == 2
            getter_expectations[(roles["connector"], "related_feature")].append(related)
            # Source and target are normative projections of observed ordered relatedFeature.
            # The exporter does not independently call those two Pilot getters.
            getter_expectations[(roles["connector"], "source")].append(related[:1])
            getter_expectations[(roles["connector"], "target")].append(related[1:])
        getter_rows = []
        for (owner, field), expected in getter_expectations.items():
            query = reads[(owner, field)]
            targets = [target["id"] for target in query.get("targets", [])] if query["status"] == "query_evaluated" else None
            getter_rows.append(dict(owner_id=owner, field=field, native_status=query["status"], native_targets=targets,
                reference_targets_by_ordinal=expected, matches_every_reference=targets is not None and all(targets == wanted for wanted in expected),
                reference_basis="normative projection of observed related_feature" if field in ["source", "target"] else "independent Pilot getter"))
        row.update(status="native_stage_compared_with_explicit_differences", reference_frames=frames, getters=getter_rows,
                   exact_stage_match=row["binding_count_matches"] and all(frame["exact_ordered_structure_and_flags"] for frame in frames)
                   and all(getter["matches_every_reference"] for getter in getter_rows))
        rows.append(row)
    return rows


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--revision", required=True)
    parser.add_argument("--native-revision", required=True)
    parser.add_argument("--reference-revision", required=True)
    parser.add_argument("--cached-getter-revision")
    args = parser.parse_args()
    native_path = EV / ("general-value-bindings-" + args.native_revision + "-native-run.json")
    reference_path = EV / ("general-value-bindings-" + args.reference_revision + "-reference-run.json")
    native = read(native_path); reference = read(reference_path)
    assert native["status"] == "native_value_binding_stage_verified_reference_comparison_required" and native["inputs_unchanged"]
    assert reference["status"] == "reference_observed_with_explicit_outcomes" and reference["inputs_unchanged"]
    assert all(sha(path) == digest for path, digest in native["input_sha256"].items())
    assert all(sha(path) == digest for path, digest in reference["input_sha256"].items())
    record = Path(native["current_record_path"]); replay = Path(native["replay_path"])
    observations = Path(reference["output_path"]); primary = Path(native["query_path"])
    for path, digest in [(record, native["current_record_sha256"]), (replay, native["replay_sha256"]),
                         (observations, reference["output_sha256"]), (primary, native["query_sha256"])]:
        assert sha(path) == digest
    sealed_path = EV / "canonical-chain-result-resolved-sealed-progress.json"; sealed = read(sealed_path)
    prior_record = Path(sealed["current_record_path"])
    assert sha(prior_record) == sealed["current_record_sha256"]
    bundle = EV / "value-result-general-value-provider-dependency-bundle.json"
    identities = EV / "value-result-lifecycle-shared-services-source-identities.json"
    valuations = read(bundle)["valuations"]
    extra = []
    for value in valuations:
        if not value["eligible_for_binding_stage"] or value["is_default"]:
            continue
        roles = roles_for(value)
        for role in ["connector", "end.0", "end.1", "value_chain"] + (["initial_context"] if value["is_initial"] else []):
            for field in GETTERS:
                if role == "connector" and field == "featuring_type":
                    continue
                extra.append(dict(owner_id=roles[role], field=field))
    assert len(extra) == 421
    folder = ROOT / ("target/general-value-bindings-" + args.revision + "-comparison")
    folder.mkdir(exist_ok=False)
    result_roots = [dict(owner_id=value["expression_id"], field="result") for value in valuations if value["eligible_for_binding_stage"]]
    assert len(result_roots) == len({row["owner_id"] for row in result_roots}) == 41
    result_spec = folder / "result-spec.json"; save(result_spec, dict(inspection_queries=result_roots))
    spec = folder / "getter-spec.json"; save(spec, dict(inspection_queries=extra))
    binary = Path(native["runs"][0]["command"][0])
    output = EV / ("general-value-bindings-" + args.revision + "-comparison.json"); assert not output.exists()
    witnesses = [native_path, reference_path, record, replay, observations, primary, sealed_path, prior_record,
                 bundle, identities, spec, result_spec, binary, Path(__file__), ROOT / "tools/audit_value_result_provider_plan.py",
                 ROOT / "tools/run_value_result_library_dependency_bundle.py", ROOT / "tools/audit_cached_native_dependencies.py"]
    manifest = dict(schema="dev.mercurio.general-value-bindings-comparison.v1", qualification_certificate=False, status="running",
        input_sha256={str(path): sha(path) for path in witnesses}, runs=[], strict_contexts_qualified=0,
        strict_obligations_qualified=0, strict_families_qualified=0, candidate_promoted=False)
    cached_outputs = []
    if args.cached_getter_revision:
        cache_path = EV / ("general-value-bindings-" + args.cached_getter_revision + "-comparison.json")
        cache = read(cache_path)
        assert cache["status"] == "comparison_driver_rejected_missing_result_witness" and cache["inputs_unchanged"]
        archives = read(ROOT / "target/r41-compare-before/witnesses.json")
        for path, digest in cache["input_sha256"].items():
            if sha(path) != digest:
                matched = [row for row in archives if row["sha256"] == digest and sha(row["archived_path"]) == digest]
                assert len(matched) == 1 and Path(path).name == "compare_general_value_bindings.py"
                manifest["input_sha256"][matched[0]["archived_path"]] = digest
        cache_runs = cache["runs"]
        assert len(cache_runs) == 2 and all(row["exit_code"] == 0 for row in cache_runs)
        for cached_run in cache_runs:
            command = cached_run["command"]
            assert command[:2] == [str(binary), "--definition-query-records"]
            assert command[2] in [str(record), str(replay)] and read(command[3])["inspection_queries"] == extra
            assert sha(cached_run["log_path"]) == cached_run["log_sha256"]
            cached_outputs.append(Path(command[4]))
        assert cache_runs[0]["command"][2] == str(record) and cache_runs[1]["command"][2] == str(replay)
        for path in [cache_path, *cached_outputs]:
            manifest["input_sha256"][str(path)] = sha(path)
        manifest["cached_getter_outputs"] = [str(path) for path in cached_outputs]
    save(output, manifest)
    fresh_answers = []; result_answers = []
    for ordinal, (label, source) in enumerate([("native", record), ("fresh", replay)]):
        if cached_outputs:
            fresh_answers.append(read(cached_outputs[ordinal])["queries"])
        else:
            target = folder / (label + "-getters.json")
            code = run([str(binary), "--definition-query-records", str(source), str(spec), str(target)], folder, label, manifest, output)
            assert code == 0
            fresh_answers.append(read(target)["queries"])
        result_target = folder / (label + "-results.json")
        code = run([str(binary), "--definition-query-records", str(source), str(result_spec), str(result_target)], folder, label + "-results", manifest, output)
        assert code == 0
        result_answers.append(read(result_target)["queries"])
    assert exact_json(*fresh_answers) and exact_json(*result_answers)
    assert len(result_answers[0]) == 41 and all(row["status"] == "query_evaluated" for row in result_answers[0])
    answers = read(primary)["queries"] + fresh_answers[0]
    rows = compare(read(record)["inspection"]["constructed_elements"], read(prior_record)["inspection"]["constructed_elements"],
        valuations, read(observations), read(identities)["canonical_resource_fragment_to_native_id"], result_answers[0], answers)
    compared = [row for row in rows if "reference_frames" in row]
    getters = [getter for row in compared for getter in row["getters"]]
    assert len(getters) == 573
    manifest.update(status="native_value_binding_stage_compared_unqualified", required_valuations=45, eligible_valuations=41,
        native_bindings_constructed=len(compared), default_dispatches_matched=3, required_link_negative_valuations=4,
        native_getters_evaluated=sum(getter["native_status"] == "query_evaluated" for getter in getters),
        native_getters_matching_all_cached_frames=sum(getter["matches_every_reference"] for getter in getters),
        getter_outcomes=dict(collections.Counter(getter["native_status"] for getter in getters)),
        raw_binding_count_disagreements=sum(not row["binding_count_matches"] for row in compared),
        fresh_getter_replay_exact=True, result_identity_getters_verified=41, result_getter_replay_exact=True, observations=rows,
        explicit_unqualified_dependencies=["Raw Pilot queues two connectors per nondefault valuation; native normative producer constructs one",
          "Owned-chain and enclosing Feature/Expression lifecycle must be completed and compared", "Bound specialization and applicable validation",
          "Both source-negative scopes, both original Ecore mutations, complete terminal publication/persistence and strict certificates"],
        boundary="Every raw reference frame is retained. Only named stored fields and getters are assessed; source/target use normative projections of observed relatedFeature. No full context or family is qualified.")
    save(output, manifest)
    print(str(manifest["native_getters_matching_all_cached_frames"]) + "/573 getters match every cached frame; " +
          str(manifest["raw_binding_count_disagreements"]) + " raw binding-count differences retained; strict families 0/34", flush=True)


if __name__ == "__main__":
    main()
