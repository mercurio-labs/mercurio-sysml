"""Cache the two crossing producer stages for actual scheduled library nodes."""
from pathlib import Path
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources

def main():
    assessment_path=EV/"general-value-bindings-remaining-feature-bundle-assessment.json";assessment=read(assessment_path)
    record=Path(assessment["current_record_path"]);assert sha(record)==assessment["current_record_sha256"]
    original=read(record);before=read(ROOT/"target/general-value-bindings-complete-type-bundle/native.jsonl")
    old_ids={node["id"] for node in before["inspection"]["constructed_elements"]};nodes=original["inspection"]["constructed_elements"]
    added=[node for node in nodes if node["id"] not in old_ids];assert len(added)==5
    crossings=[node for node in added if node["kind"]=="SysML::CrossSubsetting"];assert len(crossings)==1
    index={node["id"]:node for node in nodes};controls=[]
    names={"owned_relationship":"ownedRelationship","owned_related_element":"ownedRelatedElement"}
    for crossing in crossings:
        owner=index[crossing["properties"]["owning_related_element"]];assert owner["kind"]=="SysML::Feature"
        parts=owner["id"].split(".");source=bytes.fromhex(parts[2]).decode("utf-8");tokens=parts[3:]
        assert len(tokens)%2==0 and all(tokens[i] in names and tokens[i+1].isdigit() for i in range(0,len(tokens),2))
        fragment="/"+"".join("/@"+names[tokens[i]]+"."+tokens[i+1] for i in range(0,len(tokens),2))
        controls.append(dict(owner_id=owner["id"],source_file=source,emf_fragment=fragment))
    folder=ROOT/"target/library-crossing-strategy-physical-reference";folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    spec=folder/"spec.json";save(spec,dict(controls=controls,qualification_certificate=False))
    pinned=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08"
    jar=pinned/"org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    assert sha(jar)=="4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLibraryCrossingReference.java"
    java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    inputs=observe_sources(original)
    inputs.update({str(p):sha(p) for p in [record,assessment_path,spec,jar,source,java,javac,Path(__file__)]})
    for name in ["adapter/FeatureAdapter.java","util/FeatureUtil.java","util/TypeUtil.java"]:
        path=pinned/"org.omg.sysml.logic/src/main/java/org/omg/sysml"/name;inputs[str(path)]=sha(path)
    output=EV/"library-crossing-strategy-physical-reference-run.json";assert not output.exists()
    observations=EV/"library-crossing-strategy-physical-reference-observations.json";assert not observations.exists()
    manifest=dict(schema="dev.mercurio.library-crossing-strategy-reference-run.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],required_library_nodes=5,original_anchors=len(controls),strict_families_qualified=0)
    save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],
        [str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotLibraryCrossingReference",str(ROOT.parent/"target/upstream/SysML-v2-Release/sysml.library"),str(spec),str(observations)]]
    for number,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(number),manifest,output)
        if code:manifest.update(status="independent_stage_failed");save(output,manifest);raise SystemExit(code)
    rows=read(observations)["observations"];assert len(rows)==len(controls)
    manifest.update(status="crossing_producers_observed",output_path=str(observations),output_sha256=sha(observations),
        boundary="Independent actual-library producer snapshots; native structure, replay and full library lifecycle are separate accomplishments.")
    save(output,manifest);print("Cached independent crossing producer observations for five scheduled native nodes",flush=True)

if __name__=="__main__":main()
