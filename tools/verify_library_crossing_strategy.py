"""Compare all five scheduled crossing nodes and fresh native read projections."""
from pathlib import Path
import json
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from audit_value_result_provider_plan import native_identity
from audit_cached_native_dependencies import exact_json

def main():
    native_run_path=EV/"argument-result-specialization-native-run.json";stage=read(native_run_path)
    assert stage["status"]=="native_argument_result_stage_compared" and stage["fresh_queries_exact"]
    record=Path(stage["current_record_path"]);replay=Path(stage["replay_path"])
    assert sha(record)==stage["current_record_sha256"] and sha(replay)==stage["replay_sha256"]
    current=read(record);index={n["id"]:n for n in current["inspection"]["constructed_elements"]}
    reference_run_path=EV/"library-crossing-strategy-physical-reference-run.json";reference_run=read(reference_run_path)
    assert reference_run["status"]=="crossing_producers_observed" and reference_run["inputs_unchanged"]
    assert all(sha(p)==h for p,h in reference_run["input_sha256"].items())
    reference_path=Path(reference_run["output_path"]);assert sha(reference_path)==reference_run["output_sha256"]
    rows=read(reference_path)["observations"];assert len(rows)==1
    before=read(ROOT/"target/general-value-bindings-complete-type-bundle/native.jsonl");old_ids={n["id"] for n in before["inspection"]["constructed_elements"]}
    original_library_nodes={id for id in index if id not in old_ids and "/sysml.library/" in bytes.fromhex(id.split(".")[2]).decode("utf-8").replace(chr(92),"/")}
    assert len(original_library_nodes)==5
    comparison=[];queries=[];expected=[]
    for reference in rows:
        owner_id=reference["control"]["owner_id"];owner=index[owner_id]
        crossings=[index[id] for id in owner["properties"]["owned_relationship"] if index[id]["kind"]=="SysML::CrossSubsetting"]
        assert len(crossings)==1;crossing=crossings[0];chain=index[crossing["properties"]["crossed_feature"]]
        links=[index[id] for id in chain["properties"]["owned_relationship"]];assert len(links)==2
        roles={"crossing":crossing["id"],"chain":chain["id"]}
        def identity(row):
            id=roles[row["local_role"]] if "local_role" in row else native_identity(row["resource"],row["emf_fragment"])
            assert index[id]["kind"]=="SysML::"+row["kind"]
            return id
        def relation(native,general):
            prop=native["properties"]
            return dict(kind=native["kind"].removeprefix("SysML::"),is_implied=prop["is_implied"],is_implied_included=prop["is_implied_included"],
                specific=prop["owning_related_element"],general=prop[general],owned_related_element=prop["owned_related_element"],owning_related_element=prop["owning_related_element"])
        def reference_relation(value):
            return {key:([identity(row) for row in item] if key=="owned_related_element" else identity(item) if isinstance(item,dict) else item) for key,item in value.items()}
        def compare(id,actual,wanted):comparison.append(dict(node_id=id,actual=actual,expected=wanted,exact_match=exact_json(actual,wanted)))
        compare(crossing["id"],relation(crossing,"crossed_feature"),reference_relation(reference["crossing"]))
        compare(chain["id"],dict(kind=chain["kind"].removeprefix("SysML::"),is_implied_included=chain["properties"]["is_implied_included"],
            owning_relationship=chain["properties"]["owning_relationship"],ordered_owned_relationships=[node["id"] for node in links]),
            dict(kind=reference["chain_kind"],is_implied_included=reference["chain_completion"],owning_relationship=identity(reference["chain_owning_relationship"]),
                ordered_owned_relationships=[node["id"] for node in links]))
        for ordinal,(node,wanted) in enumerate(zip(links,reference["ordered_chain"],strict=True)):
            prop=node["properties"];roles["chain-link-"+str(ordinal)]=node["id"]
            actual=dict(kind=node["kind"].removeprefix("SysML::"),is_implied=prop["is_implied"],is_implied_included=prop["is_implied_included"],
                chaining_feature=prop["chaining_feature"],owning_related_element=prop["owning_related_element"],owned_related_element=prop["owned_related_element"])
            expected_row={k:(identity(v) if isinstance(v,dict) else [identity(row) for row in v] if isinstance(v,list) else v) for k,v in wanted.items()}
            compare(node["id"],actual,expected_row)
        assert len(reference["cross_specializations"])==1
        wanted=reference["cross_specializations"][0];cross_id=identity(wanted["specific"]);cross=index[cross_id]
        native_effects=[index[id] for id in cross["properties"]["owned_relationship"] if index[id]["kind"] in ["SysML::FeatureTyping","SysML::Subsetting"]]
        assert len(native_effects)==1;typing=native_effects[0]
        compare(typing["id"],relation(typing,"type"),reference_relation(wanted))
        for node,field,targets in [(crossing,"specific",[owner_id]),(crossing,"general",[chain["id"]]),
            (chain,"chaining_feature",[identity(row["chaining_feature"]) for row in reference["ordered_chain"]]),
            (typing,"specific",[cross_id]),(typing,"general",[identity(wanted["general"])]),
            (cross,"type",[identity(wanted["general"])]),(owner,"owned_cross_subsetting",[crossing["id"]])]:
            queries.append(dict(owner_id=node["id"],field=field));expected.append(targets)
    assert len(comparison)==5 and {row["node_id"] for row in comparison}==original_library_nodes
    folder=ROOT/"target/library-crossing-strategy-verified";folder.mkdir(exist_ok=False)
    spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=queries))
    build_path=EV/"argument-result-specialization-checks-run.json";build=read(build_path);binary=Path(build["cli_path"]);assert sha(binary)==build["cli_sha256"]
    inputs={str(p):sha(p) for p in [native_run_path,record,replay,reference_run_path,reference_path,spec,build_path,binary,Path(__file__)]}
    inputs.update({str(ROOT/p):digest for p,digest in build["input_sha256"].items()})
    output=EV/"library-crossing-strategy-comparison.json";assert not output.exists()
    manifest=dict(schema="dev.mercurio.library-crossing-strategy-comparison.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],
        native_structure_comparison=comparison,structurally_matched_nodes=sum(row["exact_match"] for row in comparison),strict_families_qualified=0)
    save(output,manifest)
    answers=[]
    for stem,source in [("queries",record),("fresh-queries",replay)]:
        query_path=folder/(stem+".json")
        assert run([str(binary),"--definition-query-records",str(source),str(spec),str(query_path)],folder,stem,manifest,output)==0
        result=read(query_path)["queries"];assert len(result)==len(queries);answers.append(result)
    assert exact_json(*answers)
    getter_comparison=[dict(owner_id=row["owner_id"],field=row["field"],native_status=row["status"],actual=[target["id"] for target in row.get("targets",[])],expected=wanted,
        exact_match=row["status"]=="query_evaluated" and [target["id"] for target in row.get("targets",[])]==wanted) for row,wanted in zip(answers[0],expected,strict=True)]
    manifest.update(status="library_crossing_stage_compared",getter_comparison=getter_comparison,independently_matched_getters=sum(row["exact_match"] for row in getter_comparison),
        required_getters=len(queries),fresh_getter_replay_exact=True,current_record_path=str(record),current_record_sha256=sha(record),
        boundary="All five physical nodes and their canonical endpoints/ordered chain are assessed at the same upstream insertion stage. Fresh native getters and unchanged persistence replay are verified. Enclosing library lifecycle, transformation-job idempotence, applicable validation and full qualification remain separate requirements.")
    save(output,manifest);print("Library crossing nodes matched",manifest["structurally_matched_nodes"],"/5; getters matched",manifest["independently_matched_getters"],"/",len(queries),flush=True)

if __name__=="__main__":main()
