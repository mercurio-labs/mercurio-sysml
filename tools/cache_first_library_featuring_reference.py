"""Cache one bounded independent first-library-feature producer bundle."""
from pathlib import Path
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run

def main():
    prior_path=EV/"attached-chain-lifecycle-reference-run.json";prior=read(prior_path)
    assert prior["inputs_unchanged"] and all(sha(Path(p))==h for p,h in prior["input_sha256"].items())
    pilot=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08";jar=pilot/"org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotFirstLibraryFeaturingReference.java"
    spec=ROOT/"target/chain-first-library-featuring/reference-spec.json";contract=read(spec)
    assert len(contract["first_feature_inventory"])==44 and len(contract["library_features"])==2
    java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    folder=ROOT/"target/first-library-featuring-reference";folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    observations=EV/"first-library-featuring-reference-observations.json";output=EV/"first-library-featuring-reference-run.json"
    assert not observations.exists() and not output.exists()
    review=EV/"canonical-chain-featuring-dependency-review.json"
    inputs=prior["input_sha256"].copy();inputs.update(read(review)["source_review"])
    inputs.update({str(p):sha(p) for p in [prior_path,jar,java,javac,source,spec,review,Path(__file__)]})
    manifest=dict(schema="dev.mercurio.first-library-featuring-reference-cache.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],strict_families_qualified=0,candidate_promoted=False);save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],[str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotFirstLibraryFeaturingReference",str(ROOT.parent/"target/upstream/SysML-v2-Release/sysml.library"),str(spec),str(observations)]]
    for n,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(n),manifest,output)
        if code:manifest["status"]="failed";save(output,manifest);raise SystemExit(code)
    rows=read(observations);assert len(rows["features"])==2 and len(rows["wrappers"])==3
    for row in rows["features"]:
        assert row["after_insertion"]==row["after_replay"] and row["nodes_after"]==row["nodes_after_replay"]==row["nodes_before"]+1
        assert row["before"]["is_implied_included"]==row["after_insertion"]["is_implied_included"] is False
    manifest.update(status="first_library_featuring_observed",output_path=str(observations),output_sha256=sha(observations),inputs_unchanged=all(sha(Path(p))==h for p,h in inputs.items()),boundary="Two actual library producer/insertion/replay controls and three duplicate-preserved wrapper frames. All other wrapper observations remain from their unchanged cached stages; full enclosing lifecycle and validation are not assessed.")
    save(output,manifest);print("First-library featuring observed:2 exact producers,3 dependent wrapper frames",flush=True)

if __name__=="__main__":main()
