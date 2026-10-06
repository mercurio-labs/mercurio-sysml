"""One independent actual-library binary cross featuring producer observation."""
from pathlib import Path
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run

def main():
    prior_path=EV/"canonical-chain-lifecycle-reference-run.json";prior=read(prior_path);assert prior["inputs_unchanged"] and all(sha(Path(p))==h for p,h in prior["input_sha256"].items())
    pilot=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08";jar=pilot/"org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotCrossFeaturingReference.java";spec=ROOT/"target/library-crossing-strategy-physical-reference/spec.json"
    java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    folder=ROOT/"target/binary-cross-featuring-reference";folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    observation=EV/"binary-cross-featuring-reference-observations.json";output=EV/"binary-cross-featuring-reference-run.json";assert not observation.exists() and not output.exists()
    inputs=prior["input_sha256"].copy();inputs.update({str(p):sha(p) for p in [prior_path,source,Path(__file__)]})
    manifest=dict(schema="dev.mercurio.binary-cross-featuring-reference-cache.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],strict_families_qualified=0);save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],[str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotCrossFeaturingReference",str(ROOT.parent/"target/upstream/SysML-v2-Release/sysml.library"),str(spec),str(observation)]]
    for n,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(n),manifest,output)
        if code:manifest["status"]="failed";save(output,manifest);raise SystemExit(code)
    rows=read(observation)["observations"];assert len(rows)==1
    manifest.update(status="binary_cross_featuring_observed",output_path=str(observation),output_sha256=sha(observation),inputs_unchanged=all(sha(Path(p))==h for p,h in inputs.items()),boundary="Actual getter after the independent owned-cross featuring producer and insertion. The written prose/formal otherEnds exclusion discrepancy is retained for normative review; no full qualification.")
    save(output,manifest);print("Actual-library binary cross featuring independently observed",flush=True)

if __name__=="__main__":main()
