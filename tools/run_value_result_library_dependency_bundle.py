"""Bounded native preflight and atomic verification of a fixed library dependency bundle.

The existing native getter/scheduler determines dependencies and targets. This
build-time orchestration supplies no endpoint hints and makes no production edits.
Independent Pilot observations are cached once after dependency discovery.
"""
from pathlib import Path
import argparse,collections,json,subprocess,time
from run_value_result_value_binding_lexical import ROOT,EV,read,sha,save,hash_matches
from audit_cached_native_dependencies import assess_successful_wave,exact_json
from audit_value_result_provider_plan import native_identity
PREFIX="value-result-value-binding-library-"
INV=EV/"value-result-value-binding-transitive-library-dependency-inventory.json"
CHECKPOINT=EV/"value-result-value-binding-lexical-progress.json"
BUILD=EV/"value-result-value-binding-lexical-replay-regression-repaired-build-run.json"
IDENTITIES=EV/"value-result-lifecycle-shared-services-source-identities.json"


def setup():
    checkpoint=read(CHECKPOINT);inventory=read(INV);build=read(BUILD)
    assert checkpoint["status"]=="bounded_positive_lexical_stage_verified" and checkpoint["positive_ports_matched"]==34
    assert build["status"]=="passed" and build["inputs_unchanged"]
    assert all(sha(ROOT/p)==h for p,h in build["input_sha256"].items())
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    assert all(sha(p)==h for p,h in inventory["input_sha256"].items())
    record=Path(checkpoint["current_record_path"]);assert sha(record)==checkpoint["current_record_sha256"]
    model=read(record);assert len(model["input_files"])==119
    pending={(row["owner_id"],row["field"]):row for row in model["inspection"]["pending_references"]}
    assert len(pending)==model["pending_reference_count"]
    integrity=read(EV/"value-result-invocation-input-integrity.json");release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(integrity["library_sha256"])==94 and all(sha(release/p)==h for p,h in integrity["library_sha256"].items())
    assert all(sha(EV.parent/p)==h for p,h in integrity["frozen_acceptance_sha256"].items())
    inputs={str(p):sha(p) for p in [CHECKPOINT,INV,BUILD,binary,record,IDENTITIES,Path(__file__),ROOT/"tools/audit_cached_native_dependencies.py",ROOT/"tools/audit_value_result_provider_plan.py",EV/"value-result-invocation-input-integrity.json"]}
    inputs.update({str(release/p):h for p,h in integrity["library_sha256"].items()})
    return checkpoint,inventory,build,binary,record,model,pending,inputs


def port_descriptor(owner_id,field,index,pending):
    contract=pending[(owner_id,field)];assert contract["owner_kind"]==index[owner_id]["kind"]
    prefix,encoded,suffix=owner_id.removeprefix("definition.").split(".",2)
    assert prefix=="resource"
    resource=bytes.fromhex(encoded).decode("utf-8").replace(chr(92),"/")
    assert "/sysml.library/" in resource,"Only actual-library registered ports belong to this bundle"
    tokens=suffix.split(".");assert len(tokens)%2==0
    names={"owned_relationship":"ownedRelationship","owned_related_element":"ownedRelatedElement"}
    assert all(tokens[i] in names and tokens[i+1].isdigit() for i in range(0,len(tokens),2))
    fragment="/"+"".join("/@"+names[tokens[i]]+"."+tokens[i+1] for i in range(0,len(tokens),2))
    return dict(owner_id=owner_id,field=field,owner_kind=index[owner_id]["kind"],resource=resource,emf_fragment=fragment,
      feature_name=contract["feature_id"].rsplit("/",1)[-1],spelling=contract["spelling"],context="actual-library-"+Path(resource).stem,
      pending_contract=contract,required_outcome="positive_library_reference")


def run(command,folder,stem,manifest,path):
    log=folder/(stem+".log");start=time.monotonic()
    with log.open("w",encoding="utf-8") as handle:code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
    row=dict(command=command,exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log))
    manifest["runs"].append(row);manifest["inputs_unchanged"]=all(sha(p)==h for p,h in manifest["input_sha256"].items());save(path,manifest)
    assert manifest["inputs_unchanged"],"Input or consumer changed during the fixed bundle"
    return code


def discover(args):
    checkpoint,inventory,build,binary,record,model,pending,inputs=setup()
    folder=ROOT/("target/"+PREFIX+args.revision+"-discovery");folder.mkdir(exist_ok=False)
    output=EV/(PREFIX+args.revision+"-discovery-run.json");assert not output.exists()
    manifest=dict(schema="dev.mercurio.library-dependency-discovery.v1",qualification_certificate=False,status="running",input_sha256=inputs,
      consumer_sha256=build["input_sha256"],fixed_contexts=26,fixed_resources=119,fixed_libraries=94,runs=[],waves=[],complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False)
    consumers=[row for port in inventory["positive_ports"] for row in port["consumers"]]
    assert len(consumers)==14 and len({(r["owner_id"],r["field"]) for r in consumers})==14
    queries_spec=folder/"focused-queries.json";save(queries_spec,dict(inspection_queries=consumers));manifest["input_sha256"][str(queries_spec)]=sha(queries_spec)
    index={row["id"]:row for row in model["inspection"]["constructed_elements"]}
    roots={(row["owner_id"],row["field"]) for row in inventory["positive_ports"]};original_roots=set(roots);completed=set()
    save(output,manifest)
    current_record=record;current=model
    for number in range(16):
        wave=sorted(roots-completed);assert wave and len(roots)<=128,"Preflight bound reached; support cannot be claimed"
        descriptions=[port_descriptor(owner,field,index,pending) for owner,field in wave]
        requirements=[dict(kind="read_field",owner_id=owner,field=field) for owner,field in wave]
        plan=folder/("wave-"+str(number)+"-plan.json");save(plan,dict(requirements=requirements));manifest["input_sha256"][str(plan)]=sha(plan)
        native=folder/("wave-"+str(number)+"-native.jsonl");command=[str(binary),"--definition-plan-records",str(current_record),str(plan),str(native)]
        code=run(command,folder,"wave-"+str(number)+"-native",manifest,output)
        if code:
            manifest.update(status="native_dependency_bundle_rejected",failed_output=read(native) if native.exists() else None);save(output,manifest);raise SystemExit(1)
        result=read(native);prior=current["inspection"]
        integrity=assess_successful_wave(prior["constructed_elements"],prior["pending_references"],result,requirements)
        assert integrity["native_nodes_added"]==0 and result["input_files"]==model["input_files"]
        completed.update((row["owner_id"],row["field"]) for row in result["committed_reference_fields"])
        # A nested native dependency is part of the same audited original registry.
        for key in completed:port_descriptor(*key,index,pending)
        roots.update(completed)
        queries=folder/("wave-"+str(number)+"-queries.json");code=run([str(binary),"--definition-query-records",str(native),str(queries_spec),str(queries)],folder,"wave-"+str(number)+"-queries",manifest,output)
        assert code==0
        rows=read(queries)["queries"];assert len(rows)==14 and {(r["owner_id"],r["field"]) for r in rows}=={(r["owner_id"],r["field"]) for r in consumers}
        required=[];external=[]
        for row in rows:
            if row["status"]!="dependency_required":continue
            dependency=row["dependency"]["prerequisite"]
            if dependency["kind"]=="read_field" and (dependency["owner_id"],dependency["field"]) in pending:
                try:port_descriptor(dependency["owner_id"],dependency["field"],index,pending)
                except AssertionError:external.append(dependency);continue
                required.append((dependency["owner_id"],dependency["field"]))
            else:external.append(dependency)
        manifest["waves"].append(dict(number=number,ports=descriptions,integrity=integrity,native_path=str(native),native_sha256=sha(native),query_path=str(queries),query_sha256=sha(queries),query_outcomes=dict(collections.Counter(r["status"] for r in rows)),semantic_dependencies_outside_library_read_strategy=external))
        current_record,current=native,result;roots.update(required);save(output,manifest)
        if not roots-completed:
            manifest.update(status="library_lexical_dependency_bundle_discovered",initial_registered_ports=len(original_roots),discovered_registered_ports=len(roots),
              ports=[port_descriptor(owner,field,index,pending) for owner,field in sorted(roots)],focused_queries=consumers,current_record_path=str(native),current_record_sha256=sha(native),
              current_query_path=str(queries),current_query_sha256=sha(queries),focused_query_outcomes=dict(collections.Counter(r["status"] for r in rows)),
              semantic_dependencies_outside_library_read_strategy=external,unsupported_provider_queries=[r for r in rows if r["status"]=="unavailable"],
              boundary="Native typed getters discovered one library read strategy. Unsupported providers and non-library semantic jobs remain explicit; no source/resource lifecycle is completed.")
            save(output,manifest);print("Library preflight:",len(roots),"registered ports;",manifest["focused_query_outcomes"],flush=True);return
        assert not any(key in completed for key in required),"A completed dependency cannot be requested repeatedly"
        print("Preflight wave",number,"committed",integrity["native_ports_committed"],"next library ports",len(roots-completed),flush=True)
    manifest.update(status="preflight_evaluation_limit",boundary="The reviewed algorithm bound was reached; the remaining strategy is not qualified.");save(output,manifest);raise SystemExit(1)


def reference(args):
    checkpoint,inventory,build,binary,record,model,pending,inputs=setup()
    discovery_path=EV/(PREFIX+args.discovery_revision+"-discovery-run.json");discovery=read(discovery_path)
    assert discovery["status"]=="library_lexical_dependency_bundle_discovered" and discovery["inputs_unchanged"]
    assert all(sha(p)==h for p,h in discovery["input_sha256"].items())
    identities=read(IDENTITIES)["canonical_resource_fragment_to_native_id"];reverse={value:key for key,value in identities.items()}
    index={row["id"]:row for row in model["inspection"]["constructed_elements"]}
    ports=list(discovery["ports"])
    for query in discovery["focused_queries"]:
        resource,fragment=reverse[query["owner_id"]].split("#",1)
        field={"inherited_membership":"inheritedMembership","type":"type","result":"result"}[query["field"]]
        ports.append(dict(**query,owner_kind=index[query["owner_id"]]["kind"],resource=resource,emf_fragment=fragment,feature_name=field,
          context=Path(resource).stem,required_outcome="provider_getter_reference"))
    spec=EV/(PREFIX+args.revision+"-reference-spec.json");assert not spec.exists();save(spec,dict(source_files=model["input_files"],ports=ports,qualification_certificate=False))
    jar=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08/org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    assert sha(jar)=="4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLifecycleReferenceRead.java"
    java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    folder=ROOT/("target/"+PREFIX+args.revision+"-reference");folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    observation=EV/(PREFIX+args.revision+"-reference-observations.json");assert not observation.exists()
    output=EV/(PREFIX+args.revision+"-reference-run.json");assert not output.exists()
    inputs.update({str(p):sha(p) for p in [spec,discovery_path,jar,source,java,javac]})
    manifest=dict(schema="dev.mercurio.library-dependency-reference.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[])
    save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],
      [str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotLifecycleReferenceRead",str(ROOT.parent/"target/upstream/SysML-v2-Release/sysml.library"),str(spec),str(observation)]]
    for number,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(number),manifest,output)
        if code:manifest["status"]="reference_failed";save(output,manifest);raise SystemExit(code)
    rows=read(observation)["observations"];assert len(rows)==len(ports)
    manifest.update(status="reference_observed",output_path=str(observation),output_sha256=sha(observation),getter_outcomes=dict(collections.Counter(r["status"] for r in rows)),required_library_ports=len(discovery["ports"]),required_provider_getters=14,
      boundary="Independent build-time getters only. Any upstream getter state changes are recorded; no full context validation or qualification.")
    save(output,manifest);print("Cached library reference:",manifest["getter_outcomes"],flush=True)


def main():
    parser=argparse.ArgumentParser();parser.add_argument("phase",choices=["discover","reference"]);parser.add_argument("--revision",required=True);parser.add_argument("--discovery-revision");args=parser.parse_args()
    if args.phase=="discover":discover(args)
    else:assert args.discovery_revision;reference(args)

if __name__=="__main__":main()
