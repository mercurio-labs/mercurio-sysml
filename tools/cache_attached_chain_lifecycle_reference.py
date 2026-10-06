"""Observe the explicit lifecycle stage of every attached value/context chain."""
from pathlib import Path
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run

def main():
    prior_path=EV/"canonical-chain-lifecycle-reference-run.json";prior=read(prior_path)
    assert prior["status"]=="attached_chain_stages_observed" and prior["inputs_unchanged"]
    assert all(sha(Path(p))==h for p,h in prior["input_sha256"].items())
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotAttachedChainLifecycleReference.java"
    folder=ROOT/"target/attached-chain-lifecycle-reference";folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    pilot=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08";jar=pilot/"org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    spec=EV/"general-value-bindings-per-binding-reference-spec.json";observation=EV/"attached-chain-lifecycle-reference-observations.json";assert not observation.exists()
    output=EV/"attached-chain-lifecycle-reference-run.json";assert not output.exists()
    inputs=prior["input_sha256"].copy();inputs.update({str(p):sha(p) for p in [prior_path,source,Path(__file__)]})
    manifest=dict(schema="dev.mercurio.attached-chain-lifecycle-reference-cache.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],strict_families_qualified=0,candidate_promoted=False);save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],[str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotAttachedChainLifecycleReference",str(ROOT.parent/"target/upstream/SysML-v2-Release/sysml.library"),str(spec),str(observation)]]
    for n,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(n),manifest,output)
        if code:manifest["status"]="failed";save(output,manifest);raise SystemExit(code)
    rows=read(observation)["observations"];assert len(rows)==45
    manifest.update(status="attached_chain_lifecycle_observed",output_path=str(observation),output_sha256=sha(observation),inputs_unchanged=all(sha(Path(p))==h for p,h in inputs.items()),required_contexts_preserved=True,boundary="Explicit producer-chain lifecycle only; upstream duplicates, normative initial targets, source-owner lifecycle and validation remain separate.")
    save(output,manifest);print("Attached chain lifecycle observed across all 45 retained valuations",flush=True)

if __name__=="__main__":main()
