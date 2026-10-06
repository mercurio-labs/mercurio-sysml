from pathlib import Path
import json,hashlib,subprocess,time,collections,sys
root=Path(__file__).resolve().parents[1];ev=root/'docs/conformance/2026-08-support/definition-pipeline-evidence'
work=root/'target/value-result-invocation-repaired';work.mkdir(exist_ok=True)
sys.path.insert(0,str(root/'tools'))
from audit_cached_native_dependencies import exact_json,assess_successful_wave
from audit_value_result_ownership import compare
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def read(p):return json.loads(Path(p).read_text(encoding='utf-8'))
def save(p,d):Path(p).write_text(json.dumps(d,indent=2)+'\n',encoding='utf-8')
binary=root/'target/release/audit_release_compile.exe';build=read(ev/'value-result-invocation-build-run.json')
assert sha(binary)==build['binary_sha256'] and build['exit_code']==0
manifest_path=ev/'value-result-invocation-fresh-execution-run.json'
assert not manifest_path.exists(),'Do not restart an existing current execution'
library_spec=ev/'value-result-provider-plan-actual-spec.json'
source_spec=ev/'value-result-lifecycle-source-spec.json'
baseline=root/'target/value-result-fresh-structure/constructor-records.jsonl'
assert sha(baseline)=='09a5c7e70529df2269bcaf33a4e79e72999c8e8ea8bbff50659b35e7767d9a22'
inputs=[binary,library_spec,source_spec,baseline,ev/'value-result-invocation-build-run.json',Path(__file__),root/'tools/audit_cached_native_dependencies.py',root/'tools/audit_value_result_ownership.py']
inputs += [Path(f) for spec in [read(library_spec),read(source_spec)] for case in spec['cases'] for f in case['input_files']]
before={str(p):sha(p) for p in inputs}
m=dict(schema='dev.mercurio.native-parameter-initialization-execution.v1',qualification_certificate=False,input_sha256=before,
 consumer_sha256=build['input_sha256'],runs=[],status='running',complete_native_contexts=0,strict_families_qualified=0,candidate_promoted=False)
save(manifest_path,m)
def run(name,command,allowed=(0,)):
 log=work/(name+'.log');start=time.monotonic()
 with log.open('w',encoding='utf-8') as f:code=subprocess.run(command,cwd=root,stdout=f,stderr=subprocess.STDOUT).returncode
 row=dict(name=name,command=[str(v) for v in command],exit_code=code,elapsed_seconds=round(time.monotonic()-start,3),log_sha256=sha(log))
 if Path(command[-1]).exists():row['output_sha256']=sha(command[-1])
 m['runs'].append(row);save(manifest_path,m)
 print(name,code,row['elapsed_seconds'],'seconds',flush=True)
 assert code in allowed,log.read_text(encoding='utf-8')[-3000:]
library_output=work/'library-inspection.jsonl';source_output=work/'source-inspection.jsonl'
run('fresh-library',[str(binary),'--definition-dependencies',str(library_spec),str(library_output)])
run('fresh-sources',[str(binary),'--definition-structure',str(source_spec),str(source_output)],(1,))
library=read(library_output)
all_sources=[json.loads(line) for line in source_output.read_text(encoding='utf-8').splitlines()]
negative=[s for s in all_sources if s['relative_path']=='valuation-missing-value']
assert len(all_sources)==26 and len(negative)==1 and negative[0]['status']=='blocked'
assert negative[0]['error'].startswith('Candidate definition document syntax:') and 'Incomplete Xtext group' in negative[0]['error']
m['expected_syntax_rejection']=dict(record=negative[0],stage='syntax',terminal_context_qualification=False)
sources=[s for s in all_sources if s['relative_path']!='valuation-missing-value']
assert len(sources)==25 and library['status']=='dependency_inspection'
assert all(s['status']=='unlinked_inspection' for s in sources)
previous=read(baseline)
rows=[library]+sources
nodes=[n for row in rows for n in row['inspection']['constructed_elements']]
pending=[p for row in rows for p in row['inspection']['pending_references']]
files=[f for row in rows for f in row['input_files']]
assert files==previous['input_files'] and len(files)==119 and len(set(files))==119
assert len(nodes)==93708 and [n['id'] for n in nodes]==[n['id'] for n in previous['inspection']['constructed_elements']]
assert exact_json(pending,previous['inspection']['pending_references'])
index={n['id']:n for n in nodes}
expected={}
ecore=read(root/'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json')
parents={c['name']:[p.split('#//')[-1] for p in c['super_types']] for c in ecore['classes']}
def conforms(kind,base):
 return kind==base or any(conforms(parent,base) for parent in parents[kind])
for member in nodes:
 kind=member['kind'].split('::')[-1]
 if not conforms(kind,'ParameterMembership'):continue
 children=member['properties']['owned_related_element']
 if children:
  expected[children[0]]='out' if conforms(kind,'ReturnParameterMembership') else 'in'
changes=[];composite_changes=[];first_parameters=collections.Counter()
for old,new in zip(previous['inspection']['constructed_elements'],nodes):
 assert old.keys()==new.keys() and all(exact_json(v,new[k]) for k,v in old.items() if k!='properties')
 bp,ap=old['properties'],new['properties']
 assert exact_json({k:v for k,v in bp.items() if k not in ('direction','is_composite')},{k:v for k,v in ap.items() if k not in ('direction','is_composite')}),'Non-direction constructor drift: '+new['id']
 if not exact_json(bp.get('is_composite'),ap.get('is_composite')):
  assert new['id'] in expected and conforms(new['kind'].split('::')[-1],'Usage') and ap['is_composite'] is False
  composite_changes.append(dict(owner_id=new['id'],previous_composite=bp.get('is_composite'),native_composite=False))
 if new['id'] in expected:
  assert ap.get('direction')==expected[new['id']]
  first_parameters[expected[new['id']]]+=1
 if not exact_json(bp.get('direction'),ap.get('direction')):
  assert new['id'] in expected
  changes.append(dict(owner_id=new['id'],previous_direction=bp.get('direction'),native_direction=ap.get('direction')))
record=dict(previous)
record['inspection']=dict(previous['inspection'],constructed_elements=nodes,pending_references=pending)
record['assembly_boundary']='Current native construction union with parser parameterDirection initialization; no borrowed semantic answers, links or completion flags.'
record_path=work/'constructor-records.jsonl';record_path.write_text(json.dumps(record,separators=(',',':'))+'\n',encoding='utf-8')
m['constructor_fidelity']=dict(resource_count=len(files),native_nodes=len(nodes),pending_ports=len(pending),
 first_parameter_directions=dict(first_parameters),direction_changes=changes,directed_usage_composite_changes=composite_changes,
 all_values_outside_declared_initialization_effects_identical=True,canonical_order_identical=True,pending_registry_identical=True,
 output_path=str(record_path),output_sha256=sha(record_path))
save(manifest_path,m)
print('Fresh constructor fidelity:',len(changes),'parameter directions corrected across119resources',flush=True)
old_plan=read(ev/'value-result-cached-dependency-plan-spec.json')
requirements=list(next(b for b in old_plan['batches'] if b['name']=='complete-construction-wave')['requirements'])
bounds=read(ev/'value-result-ownership-bounds-closure-run.json')
requirements += read(ev/'value-result-ownership-bounds-dependency-plan.json')['requirements']
requirements += [req for wave in bounds['waves'] for req in wave['requirements']]
requirements=list({json.dumps(q,sort_keys=True):q for q in requirements}.values())
registry={(p['owner_id'],p['field']) for p in pending}
requirements=[q for q in requirements if q['kind']!='read_field' or (q['owner_id'],q['field']) in registry]
plan=work/'shared-plan.json';save(plan,dict(requirements=requirements))
target=work/'shared-dependencies.jsonl'
run('shared-dependencies',[str(binary),'--definition-plan-records',str(record_path),str(plan),str(target)])
after=read(target)
m['shared_dependency_integrity']=assess_successful_wave(nodes,pending,after,requirements)
m['dependency_scope_boundary']='All30 existing source construction jobs plus the verified bound read bundle. The prior complete mixed wave rejected the missing-function control atomically; required negative linker stages remain separate and unqualified.'
m['shared_requirements']=requirements
m['current_record_path']=str(target);m['current_record_sha256']=sha(target)
queries=ev/'value-result-lifecycle-reference-read-spec.json';query_output=work/'shared-queries.json'
run('fixed-queries',[str(binary),'--definition-query-records',str(target),str(queries),str(query_output)])
native=read(query_output)
comparison=compare(read(ev/'value-result-ownership-reference-spec.json'),read(ev/'value-result-ownership-reference-observations.json'),
 native,read(ev/'value-result-lifecycle-shared-services-source-identities.json'),read(queries))
save(ev/'value-result-invocation-shared-comparison.json',comparison)
m['current_query_path']=str(query_output);m['current_query_sha256']=sha(query_output)
m['query_outcomes']=dict(collections.Counter(q['status'] for q in native['queries']))
m['ownership_outcomes']=dict(collections.Counter(p['comparison'] for p in comparison['ports']))
m['invocation_outcomes']=[p for p in comparison['ports'] if p['algorithm']=='invocation_parameter_ownership']
m['status']='current_shared_dependencies_executed_invocation_closure_pending'
m['inputs_unchanged']=all(sha(p)==h for p,h in before.items())
assert m['inputs_unchanged']
save(manifest_path,m)
print('Current fixed queries:',m['query_outcomes'],flush=True)
print('Invocation:',[(p['context'],p['field'],p['comparison']) for p in m['invocation_outcomes']],flush=True)
