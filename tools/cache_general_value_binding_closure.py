"""Cache independent endpoints once the bounded native read closure is known."""
from pathlib import Path
import argparse,collections
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources
PREFIX="general-value-binding-dependencies-"


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--closure-revision",required=True);args=parser.parse_args()
    closure_path=EV/(PREFIX+args.closure_revision+"-closure-run.json");closure=read(closure_path)
    assert closure["status"]=="native_library_read_closure_discovered" and closure["inputs_unchanged"]
    assert all(sha(path)==digest for path,digest in closure["input_sha256"].items())
    record=Path(closure["current_record_path"]);assert sha(record)==closure["current_record_sha256"]
    model=read(record);ports=closure["ports"];assert 1<=len(ports)<=128
    folder=ROOT/("target/"+PREFIX+args.revision+"-closure-reference");folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    spec=EV/(PREFIX+args.revision+"-closure-reference-spec.json");assert not spec.exists();save(spec,dict(source_files=model["input_files"],ports=ports,qualification_certificate=False))
    jar=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08/org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    assert sha(jar)=="4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLifecycleReferenceRead.java";java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    observation=EV/(PREFIX+args.revision+"-closure-reference-observations.json");assert not observation.exists();output=EV/(PREFIX+args.revision+"-closure-reference-run.json");assert not output.exists()
    inputs=observe_sources(model);inputs.update({str(path):sha(path) for path in [closure_path,record,spec,jar,source,java,javac,Path(__file__)]})
    manifest=dict(schema="dev.mercurio.general-value-binding-closure-reference.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],required_library_ports=len(ports))
    save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],
      [str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotLifecycleReferenceRead",str(ROOT.parent/"target/upstream/SysML-v2-Release/sysml.library"),str(spec),str(observation)]]
    for number,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(number),manifest,output)
        if code:manifest["status"]="reference_failed";save(output,manifest);raise SystemExit(code)
    rows=read(observation)["observations"];assert len(rows)==len(ports)
    manifest.update(status="reference_observed",output_path=str(observation),output_sha256=sha(observation),getter_outcomes=dict(collections.Counter(row["status"] for row in rows)),
      boundary="Original actual-library EReference endpoints only, observed independently after native dependency discovery. No native target hints, lifecycle or strict qualification.")
    save(output,manifest);print("Cached",len(rows),"additional transitive library port observations.",flush=True)

if __name__=="__main__":main()
