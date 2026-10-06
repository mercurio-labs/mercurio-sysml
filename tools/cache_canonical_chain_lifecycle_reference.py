"""Cache one independent, attached-resource observation batch per chain producer."""
from pathlib import Path
import collections,subprocess
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources

def main():
    seal_path=EV/"argument-result-specialization-sealed-progress.json";seal=read(seal_path)
    record=Path(seal["current_record_path"]);assert sha(record)==seal["current_record_sha256"]
    model=read(record);pilot=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08";release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert subprocess.check_output(["git","rev-parse","HEAD"],cwd=pilot,text=True).strip()=="692170b71867353b8f90341e61556f49a5beb0e5"
    assert subprocess.check_output(["git","rev-parse","HEAD"],cwd=release,text=True).strip()=="fb97b754f29588b8e9c7a35f370880cd15eb29e7"
    jar=pilot/"org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar";assert sha(jar)=="4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945"
    specs=[EV/"general-value-bindings-per-binding-reference-spec.json",ROOT/"target/library-crossing-strategy-physical-reference/spec.json",EV/"value-result-chain-transformation-reference-spec.json"]
    names=["PilotAttachedValueChainReference","PilotCrossChainLifecycleReference","PilotResultChainLifecycleReference"]
    sources=[ROOT/("tools/pilot-exporter/src/main/java/dev/mercurio/pilot/"+name+".java") for name in names]
    java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    folder=ROOT/"target/canonical-chain-lifecycle-reference";folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    inputs=observe_sources(model);inputs.update({str(p):sha(p) for p in [seal_path,record,jar,java,javac,Path(__file__),*specs,*sources]})
    for relative in ["adapter/FeatureAdapter.java","adapter/TypeAdapter.java","adapter/NamespaceAdapter.java","adapter/ElementAdapter.java","adapter/FeatureChainExpressionAdapter.java","util/FeatureUtil.java","util/TypeUtil.java","util/ElementUtil.java"]:
        p=pilot/("org.omg.sysml.logic/src/main/java/org/omg/sysml/"+relative);inputs[str(p)]=sha(p)
    output=EV/"canonical-chain-lifecycle-reference-run.json";assert not output.exists()
    manifest=dict(schema="dev.mercurio.canonical-chain-lifecycle-reference-cache.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],observations=[],strict_families_qualified=0,candidate_promoted=False)
    save(output,manifest)
    code=run([str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),*[str(p) for p in sources]],folder,"compile",manifest,output)
    if code:manifest["status"]="compile_failed";save(output,manifest);raise SystemExit(code)
    for n,(name,spec) in enumerate(zip(names,specs,strict=True)):
        observation=EV/("canonical-chain-lifecycle-reference-"+str(n)+"-observations.json");assert not observation.exists()
        code=run([str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot."+name,str(release/"sysml.library"),str(spec),str(observation)],folder,"observe-"+str(n),manifest,output)
        if code:manifest["status"]="observation_failed";save(output,manifest);raise SystemExit(code)
        rows=read(observation)["observations"]
        manifest["observations"].append(dict(observer=name,spec_path=str(spec),output_path=str(observation),output_sha256=sha(observation),rows=len(rows),outcomes=dict(collections.Counter(row.get("status","stage_recorded") for row in rows))))
        save(output,manifest);print(name,len(rows),"observations",flush=True)
    manifest.update(status="attached_chain_stages_observed",inputs_unchanged=all(sha(Path(p))==h for p,h in inputs.items()),boundary="Attached producer-chain lifecycle only; source owner lifecycle, duplicate connector and normative initial-chain disagreements, validation and release qualification remain separate.")
    save(output,manifest)

if __name__=="__main__":main()
