"""Cache independent, stage-aligned Pilot chain-producer observations."""
from pathlib import Path
import json,hashlib,subprocess,time
ROOT=Path(__file__).resolve().parents[1]
EV=ROOT/"docs/conformance/2026-08-support/definition-pipeline-evidence"
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+"\n",encoding="utf-8")
def main():
    prior=json.loads((EV/"value-result-chain-input-reference-run.json").read_text(encoding="utf-8"))
    inputs=prior["input_sha256"].copy()
    for p,h in inputs.items():assert sha(p)==h,"Pinned reference changed: "+p
    source=ROOT/"tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotExpressionContributionReference.java"
    spec=EV/"value-result-expression-contribution-batch-spec.json"
    inputs.update({str(source):sha(source),str(spec):sha(spec)})
    original=prior["runs"][1]["command"]
    # Use the actual pinned jar argument; avoid relying on positional classpath assumptions.
    jar=prior["runs"][0]["command"][prior["runs"][0]["command"].index("-cp")+1]
    folder=ROOT/"target/value-result-expression-contribution-resolved-reference";folder.mkdir(exist_ok=False)
    classes=folder/"classes";classes.mkdir()
    output=EV/"value-result-expression-contribution-resolved-reference-observations.json"
    out=EV/"value-result-expression-contribution-resolved-reference-run.json";assert not out.exists() and not output.exists()
    commands=[
        ["D:/dev/jdks/jdk21/bin/javac.exe","-encoding","UTF-8","-cp",jar,"-d",str(classes),str(source)],
        ["D:/dev/jdks/jdk21/bin/java.exe","-Xmx4g","-cp",str(classes)+";"+jar,
         "dev.mercurio.pilot.PilotExpressionContributionReference",original[-3],str(spec),str(output)]
    ]
    result=dict(schema="dev.mercurio.explicit-chain-reference-run.v1",qualification_certificate=False,
        status="running",input_sha256=inputs,runner_sha256=sha(__file__),runs=[])
    save(out,result)
    for number,command in enumerate(commands):
        log=folder/("phase-"+str(number)+".log");start=time.monotonic()
        with log.open("w",encoding="utf-8") as handle:
            code=subprocess.run(command,cwd=ROOT,stdout=handle,stderr=subprocess.STDOUT).returncode
        result["runs"].append(dict(command=command,exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),
            log_path=str(log),log_sha256=sha(log)))
        result["inputs_unchanged"]=all(sha(p)==h for p,h in inputs.items());save(out,result)
        print("phase",number,"exit",code,flush=True)
        if code or not result["inputs_unchanged"]:
            result["status"]="failed";save(out,result);print(log.read_text(encoding="utf-8")[-5000:],flush=True);raise SystemExit(1)
    observations=json.loads(output.read_text(encoding="utf-8"))["observations"]
    assert len(observations)==56
    result.update(status="reference_observed",contexts_observed=len(observations),output_path=str(output),output_sha256=sha(output))
    save(out,result)
    from collections import Counter
    print("Fixed expression observations:",dict(Counter(row["status"] for row in observations)),flush=True)
if __name__=="__main__":main()
