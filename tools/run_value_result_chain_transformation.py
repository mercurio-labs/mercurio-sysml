"""Run the complete explicit chain producer bundle on the preserved graph."""
from pathlib import Path
import collections,json,hashlib,subprocess,time
from audit_cached_native_dependencies import assess_successful_wave
from audit_value_result_provider_plan import native_identity
ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
def read(p):return json.loads(Path(p).read_text(encoding="utf-8"))
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+"\n",encoding="utf-8")
def children(index,node,field):
    return [index[k] for k in node["properties"].get(field,[])]
def owned_features(index,node):
    result=[]
    for relation in children(index,node,"owned_relationship"):
        if relation["kind"].rsplit("::",1)[-1] in {"FeatureMembership","ParameterMembership","ReturnParameterMembership"}:
            result.extend(children(index,relation,"owned_related_element"))
    return result
def compare_structures(graph,reference):
    index={node["id"]:node for node in graph};rows=[]
    for row in reference["observations"]:
        control=row["control"];chain=index[control["chain_id"]];parameter=index[control["parameter_id"]]
        source=owned_features(index,parameter)[0]
        returns=[r for r in children(index,chain,"owned_relationship") if r["kind"].endswith("::ReturnParameterMembership")]
        assert len(returns)==1
        result=children(index,returns[0],"owned_related_element")[0]
        actual=[]
        for relation in children(index,source,"owned_relationship"):
            if relation["kind"].endswith("::Redefinition"):
                assert relation["properties"]["redefining_feature"]==source["id"]
                actual.append(relation["properties"]["redefined_feature"])
        expected=[]
        for endpoint in row["cached_source_redefinitions"]:
            resource=endpoint["resource"]
            if resource.startswith("sysml.library/"):
                expected.append(native_identity(resource,endpoint["emf_fragment"]))
            else:
                # The direct target is an existing original node, mapped by its
                # canonical original resource spelling and containment fragment.
                fields={"ownedRelationship":"owned_relationship","ownedRelatedElement":"owned_related_element"}
                import re
                prefix="definition.resource."+control["source_file"].encode().hex()
                expected.append(prefix+"".join("."+fields[name]+"."+number for name,number in re.findall(r"@([A-Za-z_]+)\.(\d+)",endpoint["emf_fragment"])))
        chains=[]
        for relation in children(index,result,"owned_relationship"):
            if relation["kind"].endswith("::Subsetting") and relation["properties"].get("is_implied") is True:
                assert relation["properties"]["subsetting_feature"]==result["id"]
                general=index[relation["properties"]["subsetted_feature"]]
                links=[r["properties"]["chaining_feature"] for r in children(index,general,"owned_relationship")
                       if r["kind"].endswith("::FeatureChaining")]
                if links:chains.append(links)
        expected_chain=[[parameter["id"],source["id"]]]
        reference_chains=row["cached_result_subsettings"]
        assert len(reference_chains)==1 and len(reference_chains[0]["direct_chain"])==2
        # Canonical source role, not generated fragment coincidence: first link
        # is the fixed parameter, second is its first owned Feature in Pilot.
        assert reference_chains[0]["direct_chain"][0]["resource"]==row["parameter"]["resource"]
        assert reference_chains[0]["direct_chain"][0]["emf_fragment"]==row["parameter"]["emf_fragment"]
        assert reference_chains[0]["direct_chain"][1]["resource"]==row["source"]["resource"]
        assert reference_chains[0]["direct_chain"][1]["emf_fragment"]==row["source"]["emf_fragment"]
        flags={name:node["properties"].get("is_implied_included",False)
               for name,node in [("chain",chain),("parameter",parameter),("source",source),("result",result)]}
        rows.append(dict(context=control["context"],ordered_redefinitions_match=actual==expected,
            ordered_result_chains_match=chains==expected_chain,actual_redefinitions=actual,expected_redefinitions=expected,
            actual_result_chains=chains,expected_result_chains=expected_chain,native_flags=flags,
            reference_flags=row["flags_after_producers"],flags_match=flags==row["flags_after_producers"]))
    return rows
def main():
    before=read(EV/"value-result-chain-target-progress.json")
    build=read(EV/"value-result-chain-transformation-initial-build-run.json")
    assert build["status"]=="passed" and build["inputs_unchanged"]
    binary=ROOT/"target/release/audit_release_compile.exe";assert sha(binary)==build["binary_sha256"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    record=Path(before["current_record_path"]);assert sha(record)==before["current_record_sha256"]
    spec_path=EV/"value-result-chain-transformation-reference-spec.json"
    ref_run_path=EV/"value-result-chain-transformation-resolved-reference-run.json"
    ref_run=read(ref_run_path);assert ref_run["status"]=="reference_observed" and ref_run["inputs_unchanged"]
    ref_path=Path(ref_run["output_path"]);assert sha(ref_path)==ref_run["output_sha256"]
    assert all(sha(p)==h for p,h in ref_run["input_sha256"].items())
    spec=read(spec_path);assert len(spec["chains"])==4
    requirements=[dict(kind="chain_specialization",owner_id=row["chain_id"]) for row in spec["chains"]]
    folder=ROOT/"target/value-result-chain-transformation";folder.mkdir(exist_ok=False)
    plan=folder/"plan.json";save(plan,dict(requirements=requirements))
    out=EV/"value-result-chain-transformation-execution-run.json";assert not out.exists()
    files=[record,spec_path,ref_path,ref_run_path,binary,plan,Path(__file__)]
    witness={str(p):sha(p) for p in files}
    manifest=dict(schema="dev.mercurio.explicit-chain-execution.v1",qualification_certificate=False,status="running",
        input_sha256=witness,consumer_sha256=build["input_sha256"],required_contexts=4,required_getters=8,
        fixed_contexts=26,fixed_queries=4462,complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False)
    save(out,manifest)
    target=folder/"transformed.jsonl";log=folder/"transformation.log"
    command=[str(binary),"--definition-plan-records",str(record),str(plan),str(target)]
    start=time.monotonic()
    with log.open("w",encoding="utf-8") as handle:
        code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    manifest.update(command=command,exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),
        log_path=str(log),log_sha256=sha(log),output_path=str(target),output_sha256=sha(target))
    save(out,manifest)
    print("native chain bundle exit",code,manifest["elapsed_seconds"],flush=True)
    if code:
        manifest.update(status="native_bundle_rejected",failure=read(target),inputs_unchanged=all(sha(p)==h for p,h in witness.items()))
        save(out,manifest);print(manifest["failure"].get("error","unknown"),flush=True);raise SystemExit(code)
    prior=read(record)["inspection"];native=read(target)
    manifest["integrity"]=assess_successful_wave(prior["constructed_elements"],prior["pending_references"],native,requirements)
    comparison=compare_structures(native["inspection"]["constructed_elements"],read(ref_path))
    save(EV/"value-result-chain-transformation-structural-comparison.json",dict(
        schema="dev.mercurio.explicit-chain-structural-comparison.v1",qualification_certificate=False,
        required_contexts=4,contexts=comparison,full_transform_validation_publication="not_assessed"))
    manifest.update(status="physical_bundle_compared",contexts_matched=sum(
        all(row[field] for field in ["ordered_redefinitions_match","ordered_result_chains_match","flags_match"])
        for row in comparison),current_record_path=str(target),current_record_sha256=sha(target),
        inputs_unchanged=all(sha(p)==h for p,h in witness.items()))
    save(out,manifest)
    print(manifest["integrity"],"matched",manifest["contexts_matched"],"/4",flush=True)
    assert manifest["inputs_unchanged"] and manifest["contexts_matched"]==4
if __name__=="__main__":main()
