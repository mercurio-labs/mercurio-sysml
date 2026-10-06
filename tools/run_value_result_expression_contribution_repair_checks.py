"""Check grouped expression contributions and persisted replay regression."""
import json,hashlib,subprocess,time,re,argparse
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v): p.write_text(json.dumps(v,indent=2)+"\n",encoding="utf-8")
def run(phase,revision):
    paths=list(json.loads((EV/"value-result-expression-contribution-repaired-spec.json").read_text(encoding="utf-8"))["consumer_sha256"])
    witness={p:sha(ROOT/p) for p in paths}
    plans={
        "focused":[(["cargo","test","-p","mercurio-sysml","--release","--lib","::expression_contributions::tests::","--","--nocapture"],4),
                   (["cargo","test","-p","mercurio-sysml","--release","--lib","definition_action_contribution_transactions_persist_and_replay","--","--nocapture"],1)],
        "build":[(["cargo","test","-p","mercurio-tools","--bin","audit_release_compile","--release","cached_definition_","--","--test-threads=1"],13),
                 (["cargo","build","-p","mercurio-tools","--bin","audit_release_compile","--release"],None)],
        "regression":[([str(ROOT/"target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe"),"definition_","--test-threads=1"],506)],
    }
    prefix="value-result-expression-contribution-"+revision+"-"+phase
    out=EV/(prefix+"-run.json");assert not out.exists()
    result=dict(schema="dev.mercurio.expression-contribution-check-run.v1",qualification_certificate=False,
                status="running",input_sha256=witness,runner_sha256=sha(__file__),runs=[])
    save(out,result)
    for number,(command,count) in enumerate(plans[phase]):
        log=EV/(prefix+"-"+str(number)+".log");start=time.monotonic()
        with log.open("w",encoding="utf-8") as handle:
            code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
        content=log.read_text(encoding="utf-8")
        count_ok=count is None or bool(re.search(rf"test result: ok\. {count} passed; 0 failed;",content))
        result["runs"].append(dict(command=command,exit_code=code,log_path=str(log),log_sha256=sha(log),
            elapsed_seconds=round(time.monotonic()-start,3),required_test_count=count,required_count_observed=count_ok))
        result["inputs_unchanged"]=all(sha(ROOT/p)==s for p,s in witness.items())
        save(out,result)
        print(phase,number,"exit",code,"required",count,"count_ok",count_ok,flush=True)
        if code or not count_ok or not result["inputs_unchanged"]:
            result["status"]="failed";save(out,result);print(content[-6500:],flush=True);raise SystemExit(1)
    result["status"]="passed"
    binary=ROOT/("target/release/audit_release_compile.exe" if phase=="build" else "target/release/deps/mercurio_sysml-a53e5a7d9fbe8855.exe")
    result["binary_path"]=str(binary);result["binary_sha256"]=sha(binary);save(out,result)
if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("phase",choices=["focused","build","regression"]);parser.add_argument("--revision",required=True)
    args=parser.parse_args();run(args.phase,args.revision)
