"""Bounded, cached getters and atomic library prerequisites for all valuations.

Original-EObject anchors resolve generated result owners in upstream tooling.
Native resolvers receive only registered demands, never reference endpoints.
This records component evidence and awards no qualification certificate.
"""
from pathlib import Path
import argparse,collections,json
from run_value_result_library_dependency_bundle import ROOT,EV,IDENTITIES,read,sha,save,run,port_descriptor
from verify_value_result_library_dependency_bundle import observe_sources
PREFIX="general-value-binding-dependencies-"


def setup():
    checkpoint_path=EV/"general-value-provider-valuation-envelope-sealed-progress.json";checkpoint=read(checkpoint_path)
    assert checkpoint["status"]=="bounded_non_typing_valuation_dependency_stage_sealed"
    initial_path=EV/"general-value-binding-initial-preflight-run.json";initial=read(initial_path)
    assert initial["status"]=="dependency_bundle_preflight_observed" and initial["required_valuations"]==45
    record=Path(checkpoint["current_record_path"]);assert sha(record)==checkpoint["current_record_sha256"]==initial["current_record_sha256"]
    model=read(record);assert len(model["input_files"])==119
    inputs=observe_sources(model)
    integrity_path=EV/"value-result-invocation-input-integrity.json";integrity=read(integrity_path);release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert len(integrity["library_sha256"])==94 and all(sha(release/path)==digest for path,digest in integrity["library_sha256"].items())
    assert all(sha(EV.parent/path)==digest for path,digest in integrity["frozen_acceptance_sha256"].items())
    inputs.update({str(path):sha(path) for path in [checkpoint_path,initial_path,record,IDENTITIES,integrity_path,Path(__file__),ROOT/"tools/verify_value_result_library_dependency_bundle.py",ROOT/"tools/run_value_result_library_dependency_bundle.py"]})
    index={node["id"]:node for node in model["inspection"]["constructed_elements"]}
    pending={(row["owner_id"],row["field"]):row for row in model["inspection"]["pending_references"]}
    return checkpoint,initial,record,model,index,pending,inputs


def reference(args):
    checkpoint,initial,record,model,index,pending,inputs=setup()
    reverse={value:key for key,value in read(IDENTITIES)["canonical_resource_fragment_to_native_id"].items()}
    results={row["result_id"]:row for row in initial["valuations"]}
    def library_anchor(owner):
        resource=bytes.fromhex(owner.split(".")[2]).decode("utf-8").replace(chr(92),"/")
        assert "/sysml.library/" in resource
        parts=owner.split(".")[3:];assert len(parts)%2==0
        names={"owned_relationship":"ownedRelationship","owned_related_element":"ownedRelatedElement"}
        assert all(parts[i] in names and parts[i+1].isdigit() for i in range(0,len(parts),2))
        return resource,"/"+"".join("/@"+names[parts[i]]+"."+parts[i+1] for i in range(0,len(parts),2))
    ports=[]
    for query in initial["queries"]:
        owner,field=query["owner_id"],query["field"];path=[];anchor=owner
        resource=bytes.fromhex(owner.split(".")[2]).decode("utf-8").replace(chr(92),"/")
        if "/sysml.library/" in resource:resource,fragment=library_anchor(owner)
        elif owner in reverse:resource,fragment=reverse[owner].split("#",1)
        else:
            assert owner in results
            anchor=results[owner]["expression_id"];resource,fragment=reverse[anchor].split("#",1)
            path=[dict(feature_name="result",expected_kind=index[owner]["kind"])]
        ports.append(dict(query,owner_kind=index[owner]["kind"],anchor_id=anchor,anchor_kind=index[anchor]["kind"],anchor_fragment=fragment,getter_path=path,
          resource=resource,context=Path(resource).stem,feature_name={"type":"type","featuring_type":"featuringType","chaining_feature":"chainingFeature","end_feature":"endFeature"}[field],required_outcome="provider_getter_reference"))
    library_roots=sorted({(row["dependency"]["prerequisite"]["owner_id"],row["dependency"]["prerequisite"]["field"]) for row in initial["required_dependencies"]
      if "/sysml.library/" in bytes.fromhex(row["dependency"]["prerequisite"]["owner_id"].split(".")[2]).decode("utf-8").replace(chr(92),"/")})
    assert len(library_roots)==3
    for owner,field in library_roots:
        descriptor=port_descriptor(owner,field,index,pending)
        ports.append(dict(descriptor,anchor_id=owner,anchor_kind=index[owner]["kind"],anchor_fragment=descriptor["emf_fragment"],getter_path=[]))
    assert len(ports)==227 and len({(row["owner_id"],row["field"]) for row in ports})==227
    spec=EV/(PREFIX+args.revision+"-reference-spec.json");assert not spec.exists();save(spec,dict(source_files=model["input_files"],ports=ports,qualification_certificate=False))
    jar=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08/org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    assert sha(jar)=="4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotAnchoredReferenceRead.java";java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    folder=ROOT/("target/"+PREFIX+args.revision+"-reference");folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    observation=EV/(PREFIX+args.revision+"-reference-observations.json");assert not observation.exists();output=EV/(PREFIX+args.revision+"-reference-run.json");assert not output.exists()
    inputs.update({str(path):sha(path) for path in [spec,jar,source,java,javac]})
    manifest=dict(schema="dev.mercurio.general-value-binding-dependency-reference.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],required_getters=224,required_library_ports=3,anchored_result_getters=75)
    save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],
      [str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotAnchoredReferenceRead",str(ROOT.parent/"target/upstream/SysML-v2-Release/sysml.library"),str(spec),str(observation)]]
    for number,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(number),manifest,output)
        if code:manifest["status"]="reference_failed";save(output,manifest);raise SystemExit(code)
    rows=read(observation)["observations"];assert len(rows)==227
    manifest.update(status="reference_observed",output_path=str(observation),output_sha256=sha(observation),getter_outcomes=dict(collections.Counter(row["status"] for row in rows)),
      boundary="Independent getters and anchor-path side effects on pinned inputs, including explicit negative-context roots. No full context transformation, validation or qualification.")
    save(output,manifest);print("General binding dependency cache:",manifest["getter_outcomes"],flush=True)


def main():
    parser=argparse.ArgumentParser();parser.add_argument("phase",choices=["reference"]);parser.add_argument("--revision",required=True);args=parser.parse_args()
    reference(args)

if __name__=="__main__":main()
