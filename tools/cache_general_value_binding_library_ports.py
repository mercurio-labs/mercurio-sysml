"""One bounded independent cache for newly revealed library type ports."""
from pathlib import Path
import argparse,collections
from run_general_value_binding_dependency_batch import setup
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run,port_descriptor
PREFIX="general-value-binding-dependencies-"


def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);parser.add_argument("--preflight-revision",required=True);args=parser.parse_args()
    checkpoint,initial,record,model,index,pending,inputs=setup()
    preflight_path=EV/("general-value-binding-"+args.preflight_revision+"-preflight-run.json");preflight=read(preflight_path)
    assert preflight["status"]=="dependency_bundle_preflight_observed" and preflight["inputs_unchanged"] and preflight["current_record_sha256"]==sha(record)
    assert all(sha(path)==digest for path,digest in preflight["input_sha256"].items())
    keys=sorted({(row["dependency"]["prerequisite"]["owner_id"],row["dependency"]["prerequisite"]["field"]) for row in preflight["required_dependencies"]
      if row["dependency"]["prerequisite"]["kind"]=="read_field" and "/sysml.library/" in bytes.fromhex(row["dependency"]["prerequisite"]["owner_id"].split(".")[2]).decode("utf-8").replace(chr(92),"/")})
    assert len(keys)==8
    existing=read(EV/(PREFIX+"initial-reference-observations.json"))["observations"];old={ (row["root"]["owner_id"],row["root"]["field"]):row for row in existing if row["root"]["required_outcome"]=="positive_library_reference" }
    assert len(old)==3 and old.keys()<=set(keys)
    added=[key for key in keys if key not in old];assert len(added)==5
    ports=[port_descriptor(owner,field,index,pending) for owner,field in added]
    spec=EV/(PREFIX+args.revision+"-library-reference-spec.json");assert not spec.exists();save(spec,dict(source_files=model["input_files"],ports=ports,qualification_certificate=False))
    jar=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08/org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    assert sha(jar)=="4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotLifecycleReferenceRead.java";java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    folder=ROOT/("target/"+PREFIX+args.revision+"-library-reference");folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    observation=EV/(PREFIX+args.revision+"-library-reference-observations.json");assert not observation.exists();output=EV/(PREFIX+args.revision+"-library-reference-run.json");assert not output.exists()
    inputs.update({str(path):sha(path) for path in [preflight_path,spec,jar,source,java,javac,Path(__file__),ROOT/"tools/run_general_value_binding_dependency_batch.py",EV/(PREFIX+"initial-reference-run.json"),EV/(PREFIX+"initial-reference-observations.json")]})
    manifest=dict(schema="dev.mercurio.general-value-binding-library-reference.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],required_library_ports=8,reused_library_observations=3,additional_library_observations=5)
    save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],
      [str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotLifecycleReferenceRead",str(ROOT.parent/"target/upstream/SysML-v2-Release/sysml.library"),str(spec),str(observation)]]
    for number,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(number),manifest,output)
        if code:manifest["status"]="reference_failed";save(output,manifest);raise SystemExit(code)
    rows=read(observation)["observations"];assert len(rows)==5
    manifest.update(status="reference_observed",output_path=str(observation),output_sha256=sha(observation),getter_outcomes=dict(collections.Counter(row["status"] for row in rows)),
      boundary="Only five newly revealed original library ports were observed. The earlier 227 getters remain cached unchanged. This is reference evidence, never native support or qualification.")
    save(output,manifest);print("Cached five additional library observations; reused three existing ports and 224 provider getters.",flush=True)

if __name__=="__main__":main()
