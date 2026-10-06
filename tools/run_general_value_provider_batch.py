"""One fixed non-typing valuation dependency batch and independent getter cache.

This verifies selector dependencies, not bindings or full model qualification.
"""
from pathlib import Path
import argparse,ast,collections,json
from run_value_result_library_dependency_bundle import ROOT,EV,IDENTITIES,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources,compare,retained_projections
from audit_cached_native_dependencies import exact_json
PREFIX="general-value-provider-"
CHECKPOINT=EV/"value-result-value-binding-library-progress.json"


def setup():
    checkpoint=read(CHECKPOINT);assert checkpoint["status"]=="bounded_library_dependency_stage_verified"
    record=Path(checkpoint["current_record_path"]);assert sha(record)==checkpoint["current_record_sha256"]
    baseline=Path(checkpoint["current_query_path"]);assert sha(baseline)==checkpoint["current_query_sha256"]
    model=read(record);assert len(model["input_files"])==119 and len(model["inspection"]["constructed_elements"])==94142
    identities=read(IDENTITIES)["canonical_resource_fragment_to_native_id"];reverse={value:key for key,value in identities.items()}
    index={node["id"]:node for node in model["inspection"]["constructed_elements"]}
    previous=read(baseline)["queries"];assert len(previous)==4462
    roots=[]
    for row in previous:
        if row["status"]!="unavailable":continue
        assert row["field"] in ["type","inherited_membership"]
        resource,fragment=reverse[row["owner_id"]].split("#",1)
        roots.append(dict(owner_id=row["owner_id"],field=row["field"],owner_kind=index[row["owner_id"]]["kind"],resource=resource,emf_fragment=fragment,
          context=Path(resource).stem,feature_name={"type":"type","inherited_membership":"inheritedMembership"}[row["field"]],required_outcome="provider_getter_reference"))
    assert len(roots)==18 and len({row["context"] for row in roots})==9
    integrity=read(EV/"value-result-invocation-input-integrity.json");release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(integrity["library_sha256"])==94 and all(sha(release/p)==h for p,h in integrity["library_sha256"].items())
    assert all(sha(EV.parent/p)==h for p,h in integrity["frozen_acceptance_sha256"].items())
    witnesses=ROOT/"target/r38-provider-source-before/witnesses.json"
    for row in read(witnesses):assert sha(row["archived_path"])==row["sha256"]
    inputs=observe_sources(model)
    inputs.update({str(path):sha(path) for path in [CHECKPOINT,record,baseline,IDENTITIES,witnesses,EV/"value-result-general-value-provider-dependency-bundle.json",EV/"value-result-invocation-input-integrity.json",Path(__file__),ROOT/"tools/verify_value_result_library_dependency_bundle.py"]})
    return checkpoint,record,baseline,model,roots,inputs


def reference(args):
    checkpoint,record,baseline,model,roots,inputs=setup()
    spec=EV/(PREFIX+args.revision+"-reference-spec.json");assert not spec.exists();save(spec,dict(source_files=model["input_files"],ports=roots,qualification_certificate=False))
    jar=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08/org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    assert sha(jar)=="4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLifecycleReferenceRead.java";java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    folder=ROOT/("target/"+PREFIX+args.revision+"-reference");folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    observation=EV/(PREFIX+args.revision+"-reference-observations.json");assert not observation.exists();output=EV/(PREFIX+args.revision+"-reference-run.json");assert not output.exists()
    inputs.update({str(path):sha(path) for path in [spec,jar,source,java,javac]})
    manifest=dict(schema="dev.mercurio.general-value-provider-reference.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[])
    save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],
      [str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotLifecycleReferenceRead",str(ROOT.parent/"target/upstream/SysML-v2-Release/sysml.library"),str(spec),str(observation)]]
    for number,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(number),manifest,output)
        if code:manifest["status"]="reference_failed";save(output,manifest);raise SystemExit(code)
    rows=read(observation)["observations"];assert len(rows)==18
    manifest.update(status="reference_observed",output_path=str(observation),output_sha256=sha(observation),getter_outcomes=dict(collections.Counter(row["status"] for row in rows)),
      boundary="Independent build-time reference getters; every upstream getter mutation is recorded. No full context lifecycle or validation proof.")
    save(output,manifest);print("Independent general value-provider cache:",manifest["getter_outcomes"],flush=True)


def native(args):
    checkpoint,record,baseline,model,roots,inputs=setup()
    build_path=EV/(PREFIX+args.build_revision+"-build-run.json");build=read(build_path)
    assert build["status"]=="passed" and build["inputs_unchanged"] and all(sha(ROOT/path)==h for path,h in build["input_sha256"].items())
    binary=Path(build["binary_path"]);assert sha(binary)==build["binary_sha256"]
    reference_path=EV/(PREFIX+args.reference_revision+"-reference-run.json");reference=read(reference_path)
    assert reference["status"]=="reference_observed" and reference["inputs_unchanged"]
    compatibility=[]
    for path,digest in reference["input_sha256"].items():
        if sha(path)==digest:continue
        assert Path(path)==Path(__file__),"Pinned reference input changed: "+path
        archive=ROOT/"target/r38-provider-envelope-before"/Path(path).relative_to(ROOT)
        assert sha(archive)==digest
        def functions(source):
            return {node.name:ast.dump(node,include_attributes=False) for node in ast.parse(source).body if isinstance(node,ast.FunctionDef)}
        before=functions(archive.read_text(encoding="utf-8"));after=functions(Path(__file__).read_text(encoding="utf-8"))
        assert all(before[name]==after[name] for name in ["setup","reference"])
        compatibility.append(dict(path=path,archived_path=str(archive),original_sha256=digest,current_sha256=sha(path),reference_and_setup_ast_unchanged=True,
          reason="Only native verification/progress bookkeeping changed; the pinned reference loader/exporter functions and all upstream/source/toolchain/schema inputs are unchanged."))
    observation=Path(reference["output_path"]);assert sha(observation)==reference["output_sha256"]
    folder=ROOT/("target/"+PREFIX+args.revision+"-native");folder.mkdir(exist_ok=False);output=EV/(PREFIX+args.revision+"-native-run.json");assert not output.exists()
    spec=folder/"queries-spec.json";save(spec,dict(inspection_queries=[dict(owner_id=row["owner_id"],field=row["field"]) for row in roots]))
    inputs.update({str(path):sha(path) for path in [build_path,binary,reference_path,observation,spec]})
    inputs.update({row["archived_path"]:sha(row["archived_path"]) for row in compatibility})
    manifest=dict(schema="dev.mercurio.general-value-provider-native.v1",qualification_certificate=False,status="running",input_sha256=inputs,consumer_sha256=build["input_sha256"],runs=[],
      reference_cache_compatibility=compatibility,fixed_contexts=26,fixed_resources=119,fixed_libraries=94,required_provider_getters=18,strict_contexts_qualified=0,strict_obligations_qualified=0,strict_families_qualified=0,candidate_promoted=False)
    save(output,manifest)
    queries_path=folder/"queries.json";code=run([str(binary),"--definition-query-records",str(record),str(spec),str(queries_path)],folder,"focused-queries",manifest,output);assert code==0
    queries=read(queries_path)["queries"];assert len(queries)==18
    compared=compare(read(observation)["observations"],model["inspection"]["constructed_elements"],queries)
    assert len(compared)==18
    comparison=EV/(PREFIX+args.revision+"-comparison.json");save(comparison,dict(qualification_certificate=False,provider_getters=compared))
    manifest.update(status="provider_getters_compared",provider_getters_matched=sum(row["exact_ordered_match"] for row in compared),
      unqualified_provider_getters=[row for row in compared if not row["exact_ordered_match"]],focused_query_outcomes=dict(collections.Counter(row["status"] for row in queries)),
      current_query_path=str(queries_path),current_query_sha256=sha(queries_path),comparison_path=str(comparison),comparison_sha256=sha(comparison),
      input_graph_unchanged=sha(record)==checkpoint["current_record_sha256"],input_pending_registry_unchanged=read(record)["inspection"]["pending_references"]==model["inspection"]["pending_references"])
    save(output,manifest);print("General value-provider getters",manifest["provider_getters_matched"],"/18;",manifest["focused_query_outcomes"],flush=True)
    if manifest["unqualified_provider_getters"]:manifest["status"]="provider_stage_remaining_dependencies";save(output,manifest);raise SystemExit(1)
    fixed=EV/"value-result-lifecycle-reference-read-spec.json";full=folder/"fixed-queries.json";inputs[str(fixed)]=sha(fixed)
    code=run([str(binary),"--definition-query-records",str(record),str(fixed),str(full)],folder,"batch-boundary-queries",manifest,output);assert code==0
    current=read(full)["queries"];previous={(row["owner_id"],row["field"]):row for row in read(baseline)["queries"]}
    assert len(current)==4462 and {(row["owner_id"],row["field"]) for row in current}==previous.keys()
    regressed=[row for row in current if previous[(row["owner_id"],row["field"])]["status"]=="query_evaluated" and row["status"]!="query_evaluated"]
    featuring,chain=retained_projections(current);assert not regressed
    save(comparison,dict(qualification_certificate=False,provider_getters=compared,featuring=featuring,chain=chain))
    # Re-read the persisted original candidate in a fresh native invocation.
    replay=folder/"fresh-queries.json";code=run([str(binary),"--definition-query-records",str(record),str(spec),str(replay)],folder,"fresh-query-replay",manifest,output);assert code==0
    assert exact_json(read(replay)["queries"],queries) and sha(record)==checkpoint["current_record_sha256"]
    manifest.update(status="bounded_non_typing_valuation_dependency_stage_verified",provider_getters_matched=18,regressed_reads=regressed,
      fixed_query_outcomes=dict(collections.Counter(row["status"] for row in current)),featuring_getters_matched=48,chain_getters_matched=8,
      current_record_path=str(record),current_record_sha256=sha(record),current_query_path=str(full),current_query_sha256=sha(full),comparison_sha256=sha(comparison),
      fresh_query_replay_path=str(replay),fresh_query_replay_sha256=sha(replay),fresh_query_replay_unchanged=True,
      boundary="The complete fixed non-typing valuation selector dependency stage is verified. General bindings, bound specialization, initial context, validation, both Ecore mutations, publication, full receiver lifecycle and terminal comparisons remain unqualified.")
    save(output,manifest)
    progress=dict(manifest);progress["evidence_sha256"]={str(path):sha(path) for path in [output,reference_path,comparison]}
    progress_path=EV/(PREFIX+args.revision+"-progress.json");assert not progress_path.exists();save(progress_path,progress)
    print("Verified all 18 selector getters;",manifest["fixed_query_outcomes"],"; strict closure unchanged",flush=True)


def main():
    parser=argparse.ArgumentParser();parser.add_argument("phase",choices=["reference","native"]);parser.add_argument("--revision",required=True);parser.add_argument("--reference-revision");parser.add_argument("--build-revision");args=parser.parse_args()
    if args.phase=="reference":reference(args)
    else:assert args.reference_revision and args.build_revision;native(args)

if __name__=="__main__":main()
