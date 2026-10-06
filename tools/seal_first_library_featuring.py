"""Seal bounded dependency closure, leaving strict acceptance unchanged."""
from pathlib import Path
import re
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save

def main():
    native_path=EV/"first-library-featuring-native-run.json";native=read(native_path)
    assert native["status"]=="same_stage_first_library_featuring_verified" and native["inputs_unchanged"]
    assert all(sha(Path(p))==h for p,h in native["input_sha256"].items())
    assert native["physical_features_matching"]==native["required_features"]==2
    assert native["same_stage_chains_matching"]==native["required_chains"]==44
    assert native["same_stage_reference_frames_matching"]==native["required_reference_frames"]==83
    assert native["original_raw_chains_matching"]==42 and native["original_raw_reference_frames_matching"]==80
    assert native["retained_end_types_matching"]==76 and native["query_outcomes"]=={"query_evaluated":213}
    assert native["fresh_queries_exact"] and native["constructor_replay_exact"] and native["previous_candidate_preserved"]
    paths=[native_path,EV/"first-library-featuring-checks-run.json",EV/"first-library-featuring-reference-run.json",EV/"first-library-featuring-reference-observations.json",EV/"first-library-featuring-ecore-model-checks-run.json",EV/"first-library-featuring-xtext-terminals-checks-run.json",EV/"value-result-enclosing-lifecycle-batch-contract.json",ROOT/"target/r44-before/witnesses.json",ROOT/"target/chain-first-library-featuring/reference-spec.json",Path(__file__)]
    checks=read(paths[1]);assert checks["status"]=="passed" and checks["native_controls"]==542 and checks["tooling_controls"]==13
    assert all(sha(ROOT/p)==h for p,h in checks["input_sha256"].items())
    for run in [native,checks,read(paths[2])]:
        for row in run["runs"]:assert row["exit_code"]==0 and sha(Path(row["log_path"]))==row["log_sha256"]
    for field in ["current_record","current_query","replay","fresh_query"]:
        assert sha(Path(native[field+"_path"]))==native[field+"_sha256"]
    for path,count in [(paths[4],19),(paths[5],11)]:
        control=read(path);assert control["status"]=="passed" and control["passed_controls"]==count
        assert sha(Path(control["log_path"]))==control["log_sha256"] and sha(Path(control["binary_path"]))==control["binary_sha256"]==checks["binary_sha256"]
    before=read(ROOT/"target/r44-before/witnesses.json")["inputs"]
    assert len(before)==87 and all(sha(Path(row["path"]))==row["sha256"] for row in before.values())
    previous=read(EV/"canonical-chain-lifecycle-open-progress.json")
    assert all(before[str(ROOT/p)]["sha256"]==h for p,h in previous["consumer_sha256"].items())
    original=read(EV/"argument-result-specialization-sealed-progress.json")
    assert sha(Path(original["current_record_path"]))==original["current_record_sha256"]
    model=read(Path(native["current_record_path"]));assert len(model["input_files"])==119
    assert model["element_count"]==len(model["inspection"]["constructed_elements"])==94838
    assert model["pending_reference_count"]==len(model["inspection"]["pending_references"])==18758
    # Re-evaluate the semantic projection from frozen outputs and independent
    # observations. This comparison does not depend on the execution driver's
    # unrecorded import-time identity helper; this verifier is itself witnessed.
    index={node["id"]:node for node in model["inspection"]["constructed_elements"]}
    queries=read(Path(native["current_query_path"]))["queries"]
    assert len(queries)==213 and all(row["status"]=="query_evaluated" for row in queries)
    answers={(row["owner_id"],row["field"]):[target["id"] for target in row["targets"]] for row in queries}
    def identity(row,roles):
        if "local_role" in row:return roles[row["local_role"]]
        resource=row["resource"];fragment=row["emf_fragment"]
        assert resource.startswith("sysml.library/")
        parts=re.findall(r"@([A-Za-z_]+)\.(\d+)",fragment)
        assert fragment=="//"+"/".join("@"+name+"."+ordinal for name,ordinal in parts) and parts
        fields={"ownedRelationship":"owned_relationship","ownedRelatedElement":"owned_related_element"}
        assert all(name in fields and str(int(ordinal))==ordinal for name,ordinal in parts)
        path=str(ROOT.parent/"target/upstream/SysML-v2-Release"/resource).replace(chr(92),"/")
        return "definition.resource."+path.encode("utf-8").hex()+"".join("."+fields[name]+"."+ordinal for name,ordinal in parts)
    reference=read(EV/"first-library-featuring-reference-observations.json")
    for row in reference["features"]:
        owner=row["control"]["owner_id"];assert identity(row["after_insertion"]["identity"],{})==owner
        effects=[index[id] for id in index[owner]["properties"]["owned_relationship"] if index[id]["kind"]=="SysML::TypeFeaturing"]
        expected=row["after_insertion"]["owned_type_featuring"];assert len(effects)==len(expected)==1
        for node,wanted in zip(effects,expected,strict=True):
            assert node["kind"]=="SysML::"+wanted["kind"]
            for field in ["is_implied","is_implied_included"]:assert node["properties"][field]==wanted[field]
            for field in ["feature_of_type","featuring_type","owning_related_element"]:assert node["properties"][field]==identity(wanted[field],{})
            assert node["properties"]["owned_related_element"]==[identity(x,{}) for x in wanted["owned_related_element"]]
        assert answers[(owner,"featuring_type")]==[identity(x,{}) for x in row["after_insertion"]["featuring_type"]]
        assert answers[(owner,"owned_type_featuring")]==[node["id"] for node in effects]
    baseline=read(EV/"canonical-chain-lifecycle-native-run.json");frames_verified=0
    for old in baseline["chain_comparison"]:
        id=old["owner_id"];properties=index[id]["properties"];effects=[]
        for child in properties["owned_relationship"]:
            edge=index[child];kind=edge["kind"].removeprefix("SysML::");assert kind in ["FeatureChaining","Subsetting"]
            effects.append(dict(kind=kind,is_implied=edge["properties"]["is_implied"],target=edge["properties"]["chaining_feature" if kind=="FeatureChaining" else "subsetted_feature"]))
        actual=dict(kind=index[id]["kind"].removeprefix("SysML::"),is_implied_included=properties["is_implied_included"],effects=effects,**{field:answers[(id,field)] for field in ["type","featuring_type","chaining_feature"]})
        fresh=[frame for frame in reference["wrappers"] if frame["context"]==old["context"] and frame["role"]==old["role"]]
        if not fresh:
            expected=[frame["shape"] for frame in old["expected_frames"]]
        else:
            assert len(fresh)==len(old["expected_frames"])
            roles={}
            if old["role"]=="initial_context":
                value=index[old["valuation_id"]+".implicit.value-binding.source"]
                endpoints=[index[child]["properties"]["chaining_feature"] for child in value["properties"]["owned_relationship"] if index[child]["kind"]=="SysML::FeatureChaining"]
                roles=dict(owner=old["binding_owner"],value=endpoints[0],value_result=endpoints[1])
            expected=[]
            for frame in fresh:
                shape=frame["shape"];expected.append(dict(kind=shape["kind"],is_implied_included=shape["is_implied_included"],effects=[dict(kind=x["kind"],is_implied=x["is_implied"],target=identity(x["target"],roles)) for x in shape["effects"]],**{field:[identity(x,roles) for x in shape[field]] for field in ["type","featuring_type","chaining_feature"]}))
        assert all(actual==frame for frame in expected);frames_verified+=len(expected)
    assert frames_verified==83 and len(baseline["chain_comparison"])==44
    normative=EV/"first-library-featuring-normative-review.json";review=read(normative)
    assert review["qualification_certificate"] is False
    review["status"]="same_stage_dependency_verified_full_feature_validation_open";review["native_evidence"]="first-library-featuring-native-run.json";save(normative,review);paths.append(normative)
    output=EV/"first-library-featuring-sealed-progress.json";assert not output.exists()
    report=dict(schema="dev.mercurio.first-library-featuring-sealed-progress.v1",qualification_certificate=False,status="bounded_shared_dependency_stage_verified",candidate_promoted=False,
        native_controls_passed=542,focused_controls_passed=3,tooling_controls_passed=13,ecore_controls_passed=19,xtext_terminal_controls_passed=11,
        required_first_library_features=2,physical_first_library_features_verified=2,required_chain_wrappers=44,same_stage_chain_wrappers_verified=44,
        required_same_stage_reference_frames=83,same_stage_reference_frames_verified=83,original_raw_wrappers_matching=42,original_raw_frames_matching=80,
        required_end_types=76,end_types_matching_every_cached_frame=76,evaluated_public_reads=213,native_nodes_added=2,native_ports_committed=0,
        original_lifecycle_flags_changed=False,constructor_replay_exact=True,fresh_queries_exact=True,fixed_contexts=26,fixed_valuations=45,fixed_resources=119,fixed_libraries=94,
        element_count=94838,pending_reference_count=18758,strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,release_gates_qualified=0,
        consumer_sha256=checks["input_sha256"],evidence_sha256={str(p):sha(p) for p in paths},retained_r43_failure_archives=previous["retained_failure_archives"],
        next_dependency_contract="value-result-enclosing-lifecycle-batch-contract.json",remaining_dependencies=review["remaining"],
        metric_boundary="The two same-stage library featuring prerequisites and complete 44-wrapper projection batch are verified. This does not execute full Feature validation or close any terminal context, obligation or family. Original raw differences are preserved; execution durations are not a paired benchmark.",
        **{k:v for k,v in native.items() if k in ["current_record_path","current_record_sha256","current_query_path","current_query_sha256","replay_path","replay_sha256","fresh_query_path","fresh_query_sha256"]},
        original_r42_candidate_preserved=True,original_r43_candidate_preserved=True)
    save(output,report);print("R44 sealed:2/2 shared library dependencies,44/44 wrappers,83/83 same-stage frames; strict0/26 contexts,0/10 obligations,0/34 families",flush=True)

if __name__=="__main__":main()
