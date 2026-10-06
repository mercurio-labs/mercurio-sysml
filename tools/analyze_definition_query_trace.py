"""Summarize nested native query costs; never infer semantic qualification."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re

EVENT = re.compile(r"definition query: elapsed_ms=(\d+) phase=(\w+) owner=(.*?) kind=(.*?) field=(.*?) elements=(\d+) succeeded=(.*)$")

def analyze(text):
    stack=[]; frames=[]; starts=Counter(); durations=Counter(); query_id=0; last=-1
    for line in text.splitlines():
        match=EVENT.fullmatch(line)
        if not match: continue
        tick,phase,owner,kind,field,count,outcome=match.groups(); tick=int(tick)
        if tick<last: raise ValueError("nonmonotonic query clock")
        last=tick
        if phase.endswith("Started"):
            stage=phase[:-7]
            if stage=="Reference": query_id+=1
            scope=query_id if stage=="Reference" else (stack[-1]["query_id"] if stack else None)
            frame={"stage":stage,"owner_id":owner,"kind":kind,"field":field,"started_ms":tick,"child_ms":0,"query_id":scope}
            stack.append(frame)
            if scope is not None: starts[(scope,stage,owner,field)]+=1
        elif phase.endswith("Finished"):
            stage=phase[:-8]
            if not stack or (stack[-1]["stage"],stack[-1]["owner_id"],stack[-1]["field"])!=(stage,owner,field):
                raise ValueError("mismatched query boundaries")
            frame=stack.pop(); elapsed=tick-frame["started_ms"]
            exclusive=elapsed-frame.pop("child_ms")
            if exclusive<0: raise ValueError("nested time exceeds parent")
            frame.update(elapsed_ms=elapsed,exclusive_ms=exclusive,outcome=outcome)
            frames.append(frame)
            if stack: stack[-1]["child_ms"]+=elapsed
            if frame["query_id"] is not None: durations[(frame["query_id"],stage,owner,field)]+=elapsed
    stages=defaultdict(lambda:{"completed":0,"inclusive_ms":0,"exclusive_ms":0,"max_ms":0})
    for frame in frames:
        row=stages[frame["stage"]];row["completed"]+=1;row["inclusive_ms"]+=frame["elapsed_ms"]
        row["exclusive_ms"]+=frame["exclusive_ms"];row["max_ms"]=max(row["max_ms"],frame["elapsed_ms"])
    repeated=[{"query_id":q,"stage":stage,"owner_id":owner,"field":field,"calls":calls,"completed_inclusive_ms":durations[(q,stage,owner,field)]}
        for (q,stage,owner,field),calls in starts.items() if calls>1]
    return {"schema":"dev.mercurio.query-cost-analysis.v1","qualification":"not_assessed",
        "completed_reference_attempts":stages.get("Reference",{}).get("completed",0),"stages":dict(stages),
        "slowest_frames":sorted(frames,key=lambda f:f["elapsed_ms"],reverse=True)[:30],
        "repeated_keys_within_one_reference":sorted(repeated,key=lambda f:f["completed_inclusive_ms"],reverse=True)[:30],
        "inflight":stack,"interpretation":"Inclusive times overlap. Exclusive time includes uninstrumented subcalls. Repeated keys are confined to one immutable reference attempt; outcomes are routine results, not semantic acceptance."}

def main():
    parser=argparse.ArgumentParser();parser.add_argument("trace",type=Path);parser.add_argument("output",type=Path)
    args=parser.parse_args();data=args.trace.read_bytes();result=analyze(data.decode("utf-8-sig"))
    result["trace"]={"path":args.trace.as_posix(),"sha256":hashlib.sha256(data).hexdigest()}
    args.output.write_text(json.dumps(result,indent=2,ensure_ascii=False)+"\n",encoding="utf-8")
    print("Query-cost analysis written; semantic qualification not assessed")

if __name__=="__main__": main()
