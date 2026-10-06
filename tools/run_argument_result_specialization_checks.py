"""One bounded R42 consumer/test/build gate with source and executable provenance."""
from pathlib import Path
import json,re,subprocess,time
from run_value_result_library_dependency_bundle import ROOT,EV,read,sha,save

def main():
    previous=read(EV/"general-value-bindings-partial-projection-regression-run.json")
    paths=list(previous["input_sha256"])
    paths += ["tools/export_argument_result_specialization_bindings.py",str(Path(__file__).relative_to(ROOT)),
        "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/argument-result-specialization-bindings.extract.json"]
    paths += ["crates/mercurio-sysml/src/language_frontend/lowering/emit/"+name for name in
        ["argument_result_specialization.rs","argument_result_specialization_generated.rs","argument_result_specialization_tests.rs","expression_contributions.rs","relative_namespace.rs"]]
    witness={p:sha(ROOT/p) for p in sorted(set(paths))}
    output=EV/"argument-result-specialization-checks-run.json";assert not output.exists()
    manifest=dict(schema="dev.mercurio.argument-result-specialization-checks.v1",qualification_certificate=False,status="running",input_sha256=witness,runs=[],strict_families_qualified=0)
    save(output,manifest)
    binary=ROOT/"target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe"
    plans=[(["cargo","test","-p","mercurio-sysml","--release","--lib","definition_argument_result_rule","--","--nocapture"],6)]
    plans += [([str(binary),name,"--test-threads=1"],count) for name,count in [
        ("definition_general_value_binding_",5),("definition_reference_binding_batch_",5),("definition_result_feature_defaults_",5),
        ("definition_chain_specialization_",4),("definition_general_value_inputs_",3),("definition_",533)]]
    plans += [(["cargo","test","-p","mercurio-tools","--bin","audit_release_compile","--release","cached_definition_","--","--test-threads=1"],13),
        (["cargo","build","-p","mercurio-tools","--bin","audit_release_compile","--release"],None)]
    for number,(command,count) in enumerate(plans):
        log=EV/("argument-result-specialization-checks-"+str(number)+".log");start=time.monotonic()
        with log.open("w",encoding="utf-8") as handle:code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
        text=log.read_text(encoding="utf-8");count_ok=count is None or bool(re.search(rf"test result: ok\. {count} passed; 0 failed;",text))
        manifest["runs"].append(dict(command=command,exit_code=code,required_test_count=count,required_count_observed=count_ok,elapsed_seconds=round(time.monotonic()-start,3),log_path=str(log),log_sha256=sha(log)))
        manifest["inputs_unchanged"]=all(sha(ROOT/p)==h for p,h in witness.items());save(output,manifest)
        print("R42 checks",number,"exit",code,"required",count,"count_ok",count_ok,flush=True)
        if code or not count_ok or not manifest["inputs_unchanged"]:
            manifest["status"]="failed";save(output,manifest);print(text[-6500:],flush=True);raise SystemExit(1)
    manifest.update(status="passed",native_controls=533,focused_controls=28,tooling_controls=13,
        binary_path=str(binary),binary_sha256=sha(binary),cli_path=str(ROOT/"target/release/audit_release_compile.exe"),cli_sha256=sha(ROOT/"target/release/audit_release_compile.exe"))
    save(output,manifest)

if __name__=="__main__":main()
