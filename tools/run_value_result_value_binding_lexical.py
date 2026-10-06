"""Batch the complete registered lexical dependency bundle without target hints.

Pinned Pilot getters provide independent cached evidence. Native target selection
uses the existing Xtext/Ecore consumers and typed scheduler over all real inputs.
"""
from pathlib import Path
import argparse, collections, json, subprocess, time
from export_value_result_expression_featuring_reference import ROOT, EV, read, sha
from audit_cached_native_dependencies import assess_successful_wave, exact_json
from audit_value_result_provider_plan import native_identity
SPEC=EV/"value-result-value-binding-lexical-bundle-spec.json"
REFERENCE_SPEC=EV/"value-result-value-binding-lexical-reference-spec.json"

def save(path,value):path.write_text(json.dumps(value,indent=2)+"\n",encoding="utf-8")

def verify_inputs(spec):
    assert spec["required_ports"]==36 and spec["required_resolvable_ports"]==34 and spec["required_missing_name_ports"]==2
    assert spec["fixed_contexts"]==26 and spec["fixed_resources"]==119 and spec["fixed_libraries"]==94
    assert all(sha(p)==h for p,h in spec["input_sha256"].items())
    inventory=read(EV/"value-result-invocation-input-integrity.json")
    release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(inventory["library_sha256"])==94 and all(sha(release/p)==h for p,h in inventory["library_sha256"].items())
    assert all(sha(EV.parent/p)==h for p,h in inventory["frozen_acceptance_sha256"].items())
    return inventory

def hash_matches(path,expected):
    if sha(path)==expected:return True
    for archive in ["r36-source-before","r36-replay-regression-before","r36-replay-verifier-before"]:
        archive_path=ROOT/("target/"+archive+"/witnesses.json")
        if not archive_path.exists():continue
        archived=read(archive_path).get(str(Path(path)))
        if archived is not None and archived["sha256"]==expected and sha(archived["archived_path"])==expected:return True
    return False

def run_command(command,log):
    start=time.monotonic()
    with log.open("w",encoding="utf-8") as handle:code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    return dict(command=command,exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log))

def reference(args,spec,inventory):
    prior=read(EV/"value-result-ownership-reference-run.json")
    assert prior["inputs_unchanged"] and prior["exit_code"]==0
    assert all(sha(p)==h for p,h in prior["input_sha256"].items())
    jar=Path(prior["runs"][0]["command"][prior["runs"][0]["command"].index("-cp")+1])
    assert sha(jar)=="4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLifecycleReferenceRead.java"
    java,javac=Path("D:/dev/jdks/jdk21/bin/java.exe"),Path("D:/dev/jdks/jdk21/bin/javac.exe")
    files=[SPEC,REFERENCE_SPEC,jar,source,java,javac,Path(__file__)]
    inputs={str(p):sha(p) for p in files};inputs.update(prior["input_sha256"])
    release=ROOT.parent/"target/upstream/SysML-v2-Release";inputs.update({str(release/p):h for p,h in inventory["library_sha256"].items()})
    folder=ROOT/("target/value-result-value-binding-lexical-"+args.revision+"-reference");folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    output=EV/("value-result-value-binding-lexical-"+args.revision+"-reference-observations.json")
    manifest_path=EV/("value-result-value-binding-lexical-"+args.revision+"-reference-run.json");assert not output.exists() and not manifest_path.exists()
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],
      [str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotLifecycleReferenceRead",str(release/"sysml.library"),str(REFERENCE_SPEC),str(output)]]
    manifest=dict(schema="dev.mercurio.value-binding-lexical-reference-run.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[])
    save(manifest_path,manifest)
    for number,command in enumerate(commands):
        result=run_command(command,folder/("phase-"+str(number)+".log"));manifest["runs"].append(result)
        manifest["inputs_unchanged"]=all(sha(p)==h for p,h in inputs.items());save(manifest_path,manifest)
        print("reference",number,"exit",result["exit_code"],flush=True)
        if result["exit_code"] or not manifest["inputs_unchanged"]:
            manifest["status"]="failed";save(manifest_path,manifest);print(Path(result["log_path"]).read_text(encoding="utf-8")[-5000:]);raise SystemExit(1)
    observations=read(output)["observations"];assert len(observations)==36
    manifest.update(status="reference_observed",output_path=str(output),output_sha256=sha(output),getter_outcomes=dict(collections.Counter(row["status"] for row in observations)),
        boundary="Stored and derived reference getters only; no whole-context transform, validation or qualification.")
    save(manifest_path,manifest);print("Cached lexical reference:",manifest["getter_outcomes"],flush=True)

def compare(graph,observations,identities):
    index={node["id"]:node for node in graph};rows=[]
    def target(value):
        resource=value["resource"].replace("\\","/");fragment=value["emf_fragment"]
        if resource.startswith("sysml.library/"):return native_identity(resource,fragment)
        key=resource+"#"+fragment;assert key in identities,"Missing original source identity "+key
        return identities[key]
    for observed in observations:
        port=observed["root"];owner=index[port["owner_id"]]
        if port["required_outcome"]=="link_negative_unqualified":
            assert port["field"] not in owner["properties"]
            rows.append(dict(owner_id=owner["id"],field=port["field"],status="required_link_negative_not_qualified",reference_status=observed["status"]));continue
        expected=[dict(id=target(endpoint),kind="SysML::"+endpoint["kind"]) for endpoint in observed["endpoints"]]
        value=owner["properties"].get(port["field"]);values=value if isinstance(value,list) else [value] if value is not None else []
        actual=[dict(id=id,kind=index[id]["kind"]) for id in values]
        rows.append(dict(owner_id=owner["id"],field=port["field"],spelling=port["spelling"],expected=expected,actual=actual,
            reference_status=observed["status"],exact_ordered_match=observed["status"]=="reference_observed" and bool(expected) and actual==expected))
    assert len(rows)==36;return rows

def native(args,spec):
    build_path=EV/(("value-result-value-binding-lexical-"+args.build_revision+"-build-run.json") if args.build_revision else "value-result-expression-featuring-repaired-build-run.json");build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"] and all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    reference_path=EV/("value-result-value-binding-lexical-"+args.reference_revision+"-reference-run.json");ref=read(reference_path)
    assert ref["status"]=="reference_observed" and ref["inputs_unchanged"] and all(hash_matches(p,h) for p,h in ref["input_sha256"].items())
    observed_path=Path(ref["output_path"]);assert sha(observed_path)==ref["output_sha256"]
    record=Path(spec["retained_record_path"]);assert sha(record)==spec["retained_record_sha256"]
    extra=[]
    if args.replay_revision:
        previous_path=EV/("value-result-value-binding-lexical-"+args.replay_revision+"-native-run.json");previous=read(previous_path)
        assert previous["status"]=="native_reference_compared" and previous["ports_matched"]==34 and previous["inputs_unchanged"]
        record=Path(previous["current_record_path"]);assert sha(record)==previous["current_record_sha256"]
        extra.append(previous_path)
    prior=read(record)["inspection"];roots=spec["native_roots"]
    folder=ROOT/("target/value-result-value-binding-lexical-"+args.revision+"-native");folder.mkdir(exist_ok=False)
    plan=folder/"requirements.json";save(plan,dict(requirements=roots));output=folder/"transformed.jsonl";log=folder/"native.log"
    manifest_path=EV/("value-result-value-binding-lexical-"+args.revision+"-native-run.json");assert not manifest_path.exists()
    identities_path=EV/"value-result-lifecycle-shared-services-source-identities.json"
    files=[SPEC,build_path,binary,reference_path,observed_path,record,plan,identities_path,Path(__file__),ROOT/"tools/audit_cached_native_dependencies.py",ROOT/"tools/audit_value_result_provider_plan.py",ROOT/"target/r36-source-before/witnesses.json"]+extra
    files += [ROOT/("target/"+archive+"/witnesses.json") for archive in ["r36-replay-regression-before","r36-replay-verifier-before"]]
    inputs={str(p):sha(p) for p in files};inputs.update(spec["input_sha256"])
    command=[str(binary),"--definition-plan-records",str(record),str(plan),str(output)]
    manifest=dict(schema="dev.mercurio.value-binding-lexical-native-run.v1",qualification_certificate=False,status="running",input_sha256=inputs,
        consumer_sha256=build["input_sha256"],command=command,required_ports=36,required_resolvable_ports=34,required_missing_name_ports=2,
        fixed_contexts=26,fixed_resources=119,fixed_libraries=94,strict_families_qualified=0,complete_native_contexts=0,candidate_promoted=False)
    save(manifest_path,manifest);result=run_command(command,log);manifest.update(result)
    manifest["inputs_unchanged"]=all(sha(p)==h for p,h in inputs.items()) and all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    if result["exit_code"]:
        manifest.update(status="native_bundle_rejected",failure=read(output) if output.exists() else log.read_text(encoding="utf-8")[-5000:]);save(manifest_path,manifest)
        print("Lexical dependency bundle rejected:",str(manifest["failure"])[:5500],flush=True);raise SystemExit(1)
    manifest.update(status="native_output_requires_verification",native_output_path=str(output),native_output_sha256=sha(output));save(manifest_path,manifest)
    current=read(output);graph=current["inspection"]["constructed_elements"]
    requested_commits=roots
    if args.replay_revision:
        # Every replay root has already completed. Exact target comparison and
        # bit-faithful graph/registry equality below prove read-only reuse; the
        # ordinary wave checker still requires every fresh requested commit.
        pending_keys={(port["owner_id"],port["field"]) for port in prior["pending_references"]}
        assert all(job["kind"]=="read_field" and (job["owner_id"],job["field"]) not in pending_keys for job in roots)
        requested_commits=[]
    integrity=assess_successful_wave(prior["constructed_elements"],prior["pending_references"],current,requested_commits)
    rows=compare(graph,read(observed_path)["observations"],read(identities_path)["canonical_resource_fragment_to_native_id"])
    matched=sum(row.get("exact_ordered_match",False) for row in rows)
    comparison=EV/("value-result-value-binding-lexical-"+args.revision+"-comparison.json");save(comparison,dict(schema="dev.mercurio.value-binding-lexical-comparison.v1",qualification_certificate=False,ports=rows,complete_native_contexts=0))
    manifest.update(status="native_reference_compared",ports_matched=matched,integrity=integrity,current_record_path=str(output),current_record_sha256=sha(output),comparison_path=str(comparison),comparison_sha256=sha(comparison))
    if args.replay_revision:
        assert integrity["native_nodes_added"]==0 and integrity["native_ports_committed"]==0 and exact_json(graph,prior["constructed_elements"])
        assert current["inspection"]["pending_references"]==prior["pending_references"];manifest["graph_unchanged"]=True
    save(manifest_path,manifest);print("Lexical references matched",matched,"/34;",integrity,flush=True)
    assert matched==34 and manifest["inputs_unchanged"]

def main():
    parser=argparse.ArgumentParser();parser.add_argument("phase",choices=["reference","native"]);parser.add_argument("--revision",required=True);parser.add_argument("--reference-revision",default="initial");parser.add_argument("--replay-revision");parser.add_argument("--build-revision");args=parser.parse_args()
    spec=read(SPEC);inventory=verify_inputs(spec)
    if args.phase=="reference":reference(args,spec,inventory)
    else:native(args,spec)
if __name__=="__main__":main()
