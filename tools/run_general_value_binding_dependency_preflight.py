"""Fixed general binding dependency preflight over original native records.

Actual native getters reveal missing reads/constructions before producer edits.
Root identities are probes, never endpoint hints supplied to a native resolver.
"""
from pathlib import Path
import argparse,collections,json
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--build-revision",default="valuation-envelope");args=parser.parse_args()
    checkpoint_path=EV/"general-value-provider-valuation-envelope-sealed-progress.json";checkpoint=read(checkpoint_path)
    assert checkpoint["status"]=="bounded_non_typing_valuation_dependency_stage_sealed" and checkpoint["provider_getters_matched"]==18
    historical={}
    for path,digest in checkpoint["consumer_sha256"].items():
        current=ROOT/path
        if sha(current)==digest:historical[str(current)]=digest
        else:
            archived=ROOT/"target/r39-result-source-before"/path
            assert sha(archived)==digest,"Historical R38 consumer witness unavailable: "+path
            historical[str(archived)]=digest
    build_path=EV/("general-value-provider-"+args.build_revision+"-build-run.json");build=read(build_path);binary=Path(build["binary_path"])
    assert build["inputs_unchanged"] and all(sha(ROOT/path)==digest for path,digest in build["input_sha256"].items())
    assert build["status"]=="passed" and sha(binary)==build["binary_sha256"]
    record=Path(checkpoint["current_record_path"]);query=Path(checkpoint["current_query_path"])
    assert sha(record)==checkpoint["current_record_sha256"] and sha(query)==checkpoint["current_query_sha256"]
    model=read(record);index={node["id"]:node for node in model["inspection"]["constructed_elements"]};queries={(row["owner_id"],row["field"]):row for row in read(query)["queries"]}
    bundle_path=EV/"value-result-general-value-provider-dependency-bundle.json";bundle=read(bundle_path);assert len(bundle["valuations"])==45
    metadata=ROOT/"crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/connector-defaults.extract.json";defaults=read(metadata)
    controls=[row for row in defaults["binding_connector"]["controls"] if row["owned_ends"]==2]
    names={row["default_supertype"] for row in controls};assert names=={"Links::selfLinks"}
    def qualified(node):
        names=[];seen=set()
        while node:
            assert node["id"] not in seen;seen.add(node["id"])
            name=node["properties"].get("declared_name")
            if name:names.append(name)
            parent=node["properties"].get("owning_relationship") or node["properties"].get("owning_related_element");node=index.get(parent)
        return "::".join(reversed(names))
    library={}
    for name in [next(iter(names)),"Base::things::that","Occurrences::Occurrence::startShot"]:
        candidates=[node for node in index.values() if node["properties"].get("declared_name")==name.split("::")[-1] and qualified(node)==name]
        assert len(candidates)==1 and candidates[0]["kind"]=="SysML::Feature";library[name]=candidates[0]["id"]
    roots={};valuations=[]
    def demand(owner,field,reason):
        assert owner in index
        roots.setdefault((owner,field),[]).append(reason)
    for row in bundle["valuations"]:
        result=queries[(row["expression_id"],"result")];assert result["status"]=="query_evaluated" and len(result["targets"])==1
        result_id=result["targets"][0]["id"];assert result_id in index
        context=row["context"];valuations.append(dict(row,result_id=result_id,result_kind=index[result_id]["kind"]))
        for owner,label in [(row["owner_id"],"value receiver"),(result_id,"actual result")]:
            for field in ["type","featuring_type"]:demand(owner,field,context+": "+label)
        for owner,label in [(row["expression_id"],"value expression"),(result_id,"actual result")]:
            demand(owner,"chaining_feature",context+": "+label+" chain flattening")
    default=library[next(iter(names))]
    for field in ["type","end_feature"]:demand(default,field,"Imported binary-default dependency")
    for name in ["Base::things::that","Occurrences::Occurrence::startShot"]:
        for field in ["type","featuring_type","chaining_feature"]:demand(library[name],field,"Required initial-value context: "+name)
    folder=ROOT/("target/general-value-binding-"+args.revision+"-preflight");folder.mkdir(exist_ok=False)
    spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=[dict(owner_id=owner,field=field) for owner,field in sorted(roots)]))
    output=EV/("general-value-binding-"+args.revision+"-preflight-run.json");assert not output.exists()
    inputs=observe_sources(model);inputs.update(historical);inputs.update({str(ROOT/path):digest for path,digest in build["input_sha256"].items()});inputs.update({str(path):sha(path) for path in [checkpoint_path,build_path,binary,record,query,bundle_path,metadata,spec,Path(__file__)]})
    manifest=dict(schema="dev.mercurio.general-value-binding-dependency-preflight.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],fixed_contexts=26,fixed_resources=119,fixed_libraries=94,required_valuations=45,eligible_valuations=41,
      valuations=valuations,library_probe_identities=library,queries=[dict(owner_id=owner,field=field,required_by=required_by) for (owner,field),required_by in sorted(roots.items())],strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,candidate_promoted=False)
    save(output,manifest);native=folder/"queries.json";code=run([str(binary),"--definition-query-records",str(record),str(spec),str(native)],folder,"focused-dependencies",manifest,output);assert code==0
    rows=read(native)["queries"];assert len(rows)==len(roots) and {(row["owner_id"],row["field"]) for row in rows}==roots.keys()
    manifest.update(status="dependency_bundle_preflight_observed",focused_query_outcomes=dict(collections.Counter(row["status"] for row in rows)),
      required_dependencies=[row for row in rows if row["status"]=="dependency_required"],unsupported_dependency_reads=[row for row in rows if row["status"]=="unavailable"],
      current_record_path=str(record),current_record_sha256=sha(record),query_path=str(native),query_sha256=sha(native),
      candidate_unchanged=sha(record)==checkpoint["current_record_sha256"],
      boundary="All original valuation categories are retained. This preflight executes required getters and exposes dependencies; it does not construct bindings, complete expressions or qualify any context.")
    save(output,manifest);print("General binding preflight:",len(rows),"getter roots across all 45 valuations;",manifest["focused_query_outcomes"],flush=True)

if __name__=="__main__":main()
