"""Execute the fixed positive invocation getter bundle with native typed jobs.
No reference values or completion flags enter the model. Negative symbol and
other ownership contexts remain visible in the unchanged 34/4462 comparison.
"""
from pathlib import Path
import hashlib,json,subprocess,time,sys,collections
ROOT=Path(__file__).resolve().parents[1];EV=ROOT/'docs/conformance/2026-08-support/definition-pipeline-evidence'
WORK=ROOT/'target/value-result-invocation-repaired/closure';BINARY=ROOT/'target/release/audit_release_compile.exe'
sys.path.insert(0,str(ROOT/'tools'))
from audit_value_result_ownership import compare
from audit_cached_native_dependencies import assess_successful_wave
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def read(p):return json.loads(Path(p).read_text(encoding='utf-8'))
def save(p,d):Path(p).write_text(json.dumps(d,indent=2)+'\n',encoding='utf-8')
def run():
 WORK.mkdir(exist_ok=True)
 output=EV/'value-result-invocation-positive-closure-run.json';assert not output.exists()
 current=read(EV/'value-result-invocation-fresh-execution-run.json')
 record=Path(current['current_record_path']);query_path=Path(current['current_query_path'])
 assert sha(record)==current['current_record_sha256'] and sha(query_path)==current['current_query_sha256']
 spec_path=EV/'value-result-ownership-reference-spec.json';ref_path=EV/'value-result-ownership-reference-observations.json'
 ids_path=EV/'value-result-lifecycle-shared-services-source-identities.json';requests_path=EV/'value-result-lifecycle-reference-read-spec.json'
 spec,ref,ids,requests=map(read,[spec_path,ref_path,ids_path,requests_path])
 build=read(EV/'value-result-invocation-build-run.json');assert sha(BINARY)==build['binary_sha256']
 inputs={str(p):sha(p) for p in [BINARY,record,query_path,spec_path,ref_path,ids_path,requests_path,Path(__file__)]}
 m=dict(schema='dev.mercurio.fixed-invocation-query-closure.v1',qualification_certificate=False,
  input_sha256=inputs,consumer_sha256=build['input_sha256'],fixed_context_denominator=26,fixed_getter_denominator=4462,
  fixed_ownership_getters=34,positive_invocation_getters_required=2,negative_invocation_getters_retained=2,
  waves=[],status='running',complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False)
 completed=set()
 for number in range(1,25):
  native=read(query_path);comparison=compare(spec,ref,native,ids,requests)
  rows=[r for r in comparison['ports'] if r['algorithm']=='invocation_parameter_ownership' and r['context']=='valuation-source-07']
  assert len(rows)==2
  matched=sum(r['comparison']=='exact_ordered_match' for r in rows)
  m['current_positive_getters_matched']=matched
  print('Positive invocation:',matched,'/2 exact ordered matches',flush=True)
  if matched==2:
   m['status']='both_positive_invocation_getters_independently_matched';save(EV/'value-result-invocation-positive-closure-comparison.json',comparison);break
  if any(r['comparison']!='dependency_required' for r in rows if r['comparison']!='exact_ordered_match'):
   m['status']='precise_semantic_or_implementation_gap';m['remaining']=rows;save(output,m);return
  answers={(q['owner_id'],q['field']):q for q in native['queries']}
  needed={json.dumps(answers[r['owner_id'],r['field']]['dependency']['prerequisite'],sort_keys=True):
    answers[r['owner_id'],r['field']]['dependency']['prerequisite'] for r in rows if r['comparison']=='dependency_required'}
  assert needed and not (completed & needed.keys())
  plan=WORK/f'wave-{number:02}-plan.json';save(plan,dict(requirements=list(needed.values())))
  target=WORK/f'wave-{number:02}.jsonl';log=WORK/f'wave-{number:02}.log'
  command=[str(BINARY),'--definition-plan-records',str(record),str(plan),str(target)]
  before=read(record)['inspection'];start=time.monotonic()
  with log.open('w',encoding='utf-8') as f:code=subprocess.run(command,cwd=ROOT,stdout=f,stderr=subprocess.STDOUT).returncode
  wave=dict(number=number,requirements=list(needed.values()),command=command,exit_code=code,
   elapsed_seconds=round(time.monotonic()-start,3),input_record_sha256=sha(record),output_sha256=sha(target),log_sha256=sha(log))
  m['waves'].append(wave);print('Invocation wave',number,code,wave['elapsed_seconds'],'seconds',flush=True)
  if code:
   m['status']='positive_dependency_bundle_rejected';m['failure']=read(target);save(output,m);return
  after=read(target);wave['integrity']=assess_successful_wave(before['constructed_elements'],before['pending_references'],after,list(needed.values()))
  next_queries=WORK/f'wave-{number:02}-queries.json';ql=WORK/f'wave-{number:02}-queries.log'
  query_command=[str(BINARY),'--definition-query-records',str(target),str(requests_path),str(next_queries)]
  with ql.open('w',encoding='utf-8') as f:code=subprocess.run(query_command,cwd=ROOT,stdout=f,stderr=subprocess.STDOUT).returncode
  assert code==0
  wave.update(query_command=query_command,query_output_sha256=sha(next_queries),query_log_sha256=sha(ql))
  completed.update(needed);record=target;query_path=next_queries;save(output,m)
 else:m['status']='bounded_wave_limit_reached'
 m.update(final_record_path=str(record),final_record_sha256=sha(record),final_query_path=str(query_path),final_query_sha256=sha(query_path),
  inputs_unchanged=all(sha(p)==h for p,h in inputs.items()),query_outcomes=dict(collections.Counter(q['status'] for q in read(query_path)['queries'])))
 assert m['inputs_unchanged'];save(output,m)
 print('Invocation closure:',m['status'],flush=True)
if __name__=='__main__':run()
