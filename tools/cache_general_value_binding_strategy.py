"""One pinned independent observation batch; preserved negative scopes."""
from pathlib import Path
import argparse,collections,subprocess
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save,run
from verify_value_result_library_dependency_bundle import observe_sources

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--revision",required=True);args=parser.parse_args()
    sealed=read(EV/"canonical-chain-result-resolved-sealed-progress.json");record=Path(sealed["current_record_path"]);assert sha(record)==sealed["current_record_sha256"]
    model=read(record);bundle_path=EV/"value-result-general-value-provider-dependency-bundle.json";bundle=read(bundle_path);assert len(bundle["valuations"])==45
    valuations=[]
    for value in bundle["valuations"]:
        source,fragment=value["source_identity"].rsplit("#",1)
        valuations.append(dict(context=value["context"],valuation_id=value["valuation_id"],source_file=source,valuation_fragment=fragment,eligible=value["eligible_for_binding_stage"]))
    prefix="general-value-bindings-"+args.revision+"-reference";spec=EV/(prefix+"-spec.json");assert not spec.exists()
    save(spec,dict(schema="dev.mercurio.general-value-binding-reference-spec.v1",source_files=model["input_files"],valuations=valuations,qualification_certificate=False))
    pilot=ROOT.parent/"target/support-2026-08/pilot-pinned-2026-08";release=ROOT.parent/"target/upstream/SysML-v2-Release"
    assert subprocess.check_output(["git","rev-parse","HEAD"],cwd=pilot,text=True).strip()=="692170b71867353b8f90341e61556f49a5beb0e5"
    assert subprocess.check_output(["git","rev-parse","HEAD"],cwd=release,text=True).strip()=="fb97b754f29588b8e9c7a35f370880cd15eb29e7"
    jar=pilot/"org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    assert sha(jar)=="4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945"
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotGeneralValueBindingReference.java"
    java=Path("D:/dev/jdks/jdk21/bin/java.exe");javac=java.with_name("javac.exe")
    folder=ROOT/("target/"+prefix);folder.mkdir(exist_ok=False);classes=folder/"classes";classes.mkdir()
    observation=EV/(prefix+"-observations.json");assert not observation.exists();output=EV/(prefix+"-run.json");assert not output.exists()
    inputs=observe_sources(model);inputs.update({str(p):sha(p) for p in [record,spec,bundle_path,jar,source,java,javac,Path(__file__)]})
    for relative in ["adapter/FeatureAdapter.java","util/FeatureUtil.java","util/ConnectorUtil.java","util/ElementUtil.java"]:
        p=pilot/("org.omg.sysml.logic/src/main/java/org/omg/sysml/"+relative);inputs[str(p)]=sha(p)
    manifest=dict(schema="dev.mercurio.general-value-binding-reference-cache.v1",qualification_certificate=False,status="running",input_sha256=inputs,runs=[],required_valuations=45,eligible_valuations=41,
        fixed_contexts=26,fixed_resources=119,fixed_libraries=94,strict_families_qualified=0,candidate_promoted=False)
    save(output,manifest)
    commands=[[str(javac),"-encoding","UTF-8","-cp",str(jar),"-d",str(classes),str(source)],
        [str(java),"-Xmx4g","-cp",str(classes)+";"+str(jar),"dev.mercurio.pilot.PilotGeneralValueBindingReference",str(release/"sysml.library"),str(spec),str(observation)]]
    for number,command in enumerate(commands):
        code=run(command,folder,"reference-"+str(number),manifest,output)
        if code:manifest["status"]="reference_failed";save(output,manifest);raise SystemExit(code)
    rows=read(observation)["observations"];assert len(rows)==45
    outcomes=dict(collections.Counter(row["status"] for row in rows))
    manifest.update(status="reference_observed_with_explicit_outcomes",observation_outcomes=outcomes,output_path=str(observation),output_sha256=sha(observation),
        boundary="Reference behavior only, with every required negative scope retained. Stage errors, endpoint/flag/effect disagreements and enclosing lifecycle remain explicitly unqualified.")
    save(output,manifest);print("Cached general value-binding strategy:",outcomes,"; strict families 0/34",flush=True)

if __name__=="__main__":main()
