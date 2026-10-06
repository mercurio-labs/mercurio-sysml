"""Reproduce bounded native V01 result evidence from pinned independent inputs.

This audit cannot award complete context, library or release qualification.
Run native inspections with audit_release_compile first, then use --check to
verify these recorded comparisons without editing evidence.
"""
import argparse
import json
from pathlib import Path


def write_or_check(path, data, check):
    encoded = json.dumps(data, indent=2) + "\n"
    if check:
        if path.read_text(encoding="utf-8") != encoded:
            raise ValueError("Bounded native result evidence is stale: " + str(path))
    else:
        path.write_text(encoded, encoding="utf-8")


def audit_actual_providers(check=False):
    from pathlib import Path
    import json,hashlib
    r=Path(__file__).resolve().parents[1];b=r/'docs/conformance/2026-08-support/definition-pipeline-evidence'
    sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
    frozen=json.loads((b/'value-result-parameter-plan-provider-input.json').read_text())
    run=json.loads((b/'value-result-parameter-plan-provider-run.json').read_text())
    if any(x['exit_code'] for x in run['runs']) or not run['inputs_unchanged']: raise RuntimeError('Native run not verified')
    for p,h in run['consumer_sha256'].items():
     if sha(r/p)!=h: raise RuntimeError('Consumer changed: '+p)
    if sha(frozen['resource'])!=frozen['resource_sha256'] or sha(r/frozen['reference_cache'])!=frozen['reference_cache_sha256']: raise RuntimeError('Provider input changed')
    path=b/'value-result-parameter-plan-provider-inspection.jsonl'
    rows=[json.loads(l) for l in path.read_text().splitlines() if l.strip()]
    if len(rows)!=1 or rows[0]['status']!='unlinked_inspection' or rows[0]['publication']!='not_attempted': raise RuntimeError('Inspection boundary changed')
    row=rows[0];nodes=row['inspection']['constructed_elements'];index={n['id']:n for n in nodes}
    if len(index)!=len(nodes): raise RuntimeError('Duplicate native identity')
    queries={tuple(q['owner_path']):q for q in row['queries']}
    if len(queries)!=3: raise RuntimeError('Exactly three observed provider queries required')
    verified=[];pending=row['inspection']['pending_references']
    def kind(n): return n['kind'].split('::')[-1]
    def children(n,field):
     ids=n['properties'].get(field,[])
     if not isinstance(ids,list) or len(set(ids))!=len(ids): raise RuntimeError('Noncanonical ordered collection')
     return [index[i] for i in ids]
    def descendants(n):
     todo=[n];reached={}
     while todo:
      c=todo.pop()
      if c['id'] in reached: raise RuntimeError('Ownership cycle or duplicate containment')
      reached[c['id']]=c
      for field,inverse in [('owned_relationship','owning_related_element'),('owned_related_element','owning_relationship')]:
       for child in children(c,field):
        if child['properties'].get(inverse)!=c['id']: raise RuntimeError('Broken reciprocal containment')
        todo.append(child)
     return reached
    for ref in frozen['providers']:
     q=queries[tuple(ref['owner']['qualified_name'].split('::'))]
     if q['status']!='query_evaluated' or len(q['targets'])!=1: raise RuntimeError('Actual native result query failed: '+json.dumps(q))
     result=index[q['targets'][0]['id']];owner=index[q['owner_id']]
     membership=index[result['properties']['owning_relationship']]
     if kind(owner)!=ref['owner']['kind'] or kind(result)!=ref['result']['kind'] or kind(membership)!=ref['membership']['kind']: raise RuntimeError('Independent metaclass mismatch')
     if children(membership,'owned_related_element')!=[result] or membership['properties'].get('owning_related_element')!=owner['id'] or membership not in children(owner,'owned_relationship'): raise RuntimeError('Result reciprocal ownership mismatch')
     if result['properties'].get('direction')!=ref['result']['stored_attributes']['direction']: raise RuntimeError('Direction mismatch')
     for role,node in [('owner',owner),('membership',membership),('result',result)]:
      expected=ref[role]
      if node['properties'].get('declared_name')!=expected['stored_attributes'].get('declaredName'): raise RuntimeError('Stored declaration name mismatch; inherited names cannot be substituted')
      if node['properties'].get('is_implied_included',False)!=expected['stored_attributes']['isImpliedIncluded']: raise RuntimeError('Fabricated provider completion')
      for field,upstream in [('owned_relationship','ownedRelationship'),('owned_related_element','ownedRelatedElement')]:
       if upstream not in expected['reference_sequences']: continue
       if [kind(c) for c in children(node,field)]!=[c['kind'] for c in expected['reference_sequences'][upstream]]:
        raise RuntimeError('Ordered ownership kind mismatch for '+role)
     owned=descendants(owner);ports=[p for p in pending if p['owner_id'] in owned]
     verified.append({'provider':ref['result']['qualified_name'],'native_owner':owner['id'],'native_result':result['id'],
      'contexts':ref['contexts'],'result_selection':'verified_against_independent_direct_query',
      'canonical_stored_ownership':'verified_reciprocal_order_and_metaclasses',
      'stored_declarations_and_direction':'verified','completion_flags':'unchanged_incomplete',
      'native_owned_nodes':len(owned),'pending_reference_ports':ports,
      'required_native_consumers':[
       {'operation':'owned result/parameter selection','consumer':'parameter_members::owned_parameters/owned_result','status':'verified_for_actual_provider'},
       {'operation':'scope/link pending fields','consumer':'generated_resource_reference_traced + typed read/construction scheduler','status':'unverified_for_actual_provider'},
       {'operation':'general/prototype/default closure','consumer':'definition_general_type_inputs_with_view + imported dispatch/delegates','status':'unverified_for_actual_provider'},
       {'operation':'lifecycle/semantic validation/publication/persistence','consumer':'native transformation and validation pipeline','status':'required_unverified'}],
      'native_context_qualified':False})
    summary={'schema':'dev.mercurio.value-result-native-provider-evidence.v1','qualification_certificate':False,
     'scope':'Actual pinned Performances source parsed/constructed by native imported grammar/Ecore consumers; three direct result providers independently checked. Full source/library semantics unqualified.',
     'native_provider_queries_verified':len(verified),'complete_contexts_verified':0,'strict_families_qualified':0,
     'source_element_count':row['element_count'],'source_pending_reference_count':row['pending_reference_count'],
     'providers':verified,'inputs':{'provider_input_sha256':sha(b/'value-result-parameter-plan-provider-input.json'),
     'inspection_sha256':sha(path),'run_sha256':sha(b/'value-result-parameter-plan-provider-run.json')},
     'remaining':['All recorded pending reads and required semantic dependencies must execute in the native source/provider bundle.',
     'Whole original nine/three result selection and every 26-context terminal outcome require native complete-pipeline comparison.',
     'This diagnostic inspection has no publication API and cannot qualify a model or lifecycle.']}
    write_or_check(b/'value-result-parameter-plan-provider-evidence.json', summary, check)
    print(json.dumps({k:summary[k] for k in ['native_provider_queries_verified','source_element_count','source_pending_reference_count','complete_contexts_verified']},indent=2))
    for p in verified: print(p['provider'],'owned nodes',p['native_owned_nodes'],'pending ports',len(p['pending_reference_ports']))


def audit_original_results(check=False):
    from pathlib import Path
    import json,gzip,hashlib
    r=Path(__file__).resolve().parents[1];b=r/'docs/conformance/2026-08-support';out=b/'definition-pipeline-evidence'
    sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
    run=json.loads((out/'value-result-parameter-plan-original-run.json').read_text())
    if run['exit_code'] or not run['inputs_unchanged']: raise RuntimeError('Original query run failed')
    if sha(r/'target/release/audit_release_compile.exe')!=run['binary_sha256']:raise RuntimeError('Native binary changed')
    bundle=json.loads((b/'value-result-obligation-bundle.json').read_text())
    reference=json.loads(gzip.decompress((b/'value-result-reference-controls.json.gz').read_bytes()))
    frozen=json.loads((out/'value-result-parameter-plan-provider-input.json').read_text())
    if sha(r/frozen['reference_cache'])!=frozen['reference_cache_sha256'] or sha(frozen['resource'])!=frozen['resource_sha256']:raise RuntimeError('Reference environment changed')
    controls={c['id']:c for c in reference['controls']}
    contexts={c['id']:c for c in bundle['contexts'] if c['role']=='original_parsed_control'}
    raw=out/'value-result-parameter-plan-original-inspection.jsonl'
    rows=[json.loads(l) for l in raw.read_text().splitlines() if l.strip()]
    if len(rows)!=12 or {x['relative_path'] for x in rows}!=set(contexts):raise RuntimeError('Original context denominator changed')
    verified=[];split={'owned':0,'inherited':0}
    def short(e):return e['kind'].split('::')[-1]
    def parent(index,e):
     p=e['properties']
     return index[p['owning_relationship']] if p.get('owning_relationship') else index[p['owning_related_element']] if p.get('owning_related_element') else None
    def declared_path(index,e):
     names=[];seen=set()
     while e is not None:
      if e['id'] in seen:raise RuntimeError('Ownership cycle')
      seen.add(e['id'])
      if e['properties'].get('declared_name') is not None:names.append(e['properties']['declared_name'])
      e=parent(index,e)
     return list(reversed(names))
    for row in rows:
     c=contexts[row['relative_path']]
     if sha(r/c['source'])!=c['source_sha256']:raise RuntimeError('Frozen source changed')
     if row['status']!='unlinked_inspection' or row['publication']!='not_attempted' or len(row['queries'])!=1:raise RuntimeError('Inspection boundary changed')
     q=row['queries'][0]
     if q['status']!='query_evaluated' or len(q['targets'])!=1:raise RuntimeError('Result query unavailable')
     index={e['id']:e for e in row['inspection']['constructed_elements']}
     if len(index)!=row['element_count']:raise RuntimeError('Duplicate native identity')
     owner=index[q['owner_id']];target=index[q['targets'][0]['id']];member=parent(index,target);result_owner=parent(index,member)
     if short(member)!='ReturnParameterMembership' or member['properties']['owned_related_element']!=[target['id']]:raise RuntimeError('Return ownership missing')
     native_owned=result_owner['id']==owner['id'];branch='owned' if native_owned else 'inherited'
     if branch!=c['original_result_branch']:raise RuntimeError('Original owned/inherited branch changed')
     observed=controls[c['id']]['result_observations']
     source_function=[o for o in observed['queries'] if o['kind']=='Function' and (o.get('qualified_name') or '').endswith('::f')]
     if len(source_function)!=1:raise RuntimeError('Independent function unavailable')
     expected=source_function[0];witnesses={w['id']:w for w in observed['stored_witnesses']}
     ref_result=witnesses[expected['result']['id']];ref_membership=witnesses[ref_result['reference_sequences']['owningRelationship'][0]['id']]
     ref_owner=witnesses[ref_membership['reference_sequences']['owningRelatedElement'][0]['id']]
     if short(target)!=ref_result['kind'] or short(result_owner)!=ref_owner['kind']:raise RuntimeError('Result/provider metaclass mismatch')
     if target['properties'].get('direction')!=ref_result['stored_attributes']['direction']:raise RuntimeError('Result direction mismatch')
     if target['properties'].get('declared_name')!=ref_result['stored_attributes'].get('declaredName'):raise RuntimeError('Stored name mismatch')
     if declared_path(index,result_owner)!=ref_owner['qualified_name'].split('::'):raise RuntimeError('Actual selected provider identity mismatch')
     returns=[index[i] for i in owner['properties']['owned_relationship'] if short(index[i])=='ReturnParameterMembership']
     if len(returns)!=len(expected['owned_return_memberships']):raise RuntimeError('Fabricated or lost native return membership')
     library_root='definition.resource.'+frozen['resource'].encode('utf-8').hex()
     if not native_owned and not result_owner['id'].startswith(library_root+'.'):raise RuntimeError('Inherited provider not from actual supplied library')
     if owner['properties'].get('is_implied_included',False):raise RuntimeError('Fabricated source completion flag')
     split[branch]+=1
     verified.append({'context':c['id'],'native_result_query':'verified_against_independent_original',
      'branch':branch,'actual_result_owner':ref_owner['qualified_name'],
      'result_direction':target['properties']['direction'],'owned_return_memberships':len(returns),
      'parse_construct':'native_pinned_grammar_ecore','link_transform_validate_publish_persist':'unqualified',
      'pending_reference_count':row['pending_reference_count'],'native_context_complete':False})
    if split!={'owned':9,'inherited':3}:raise RuntimeError('Original split changed')
    e={'schema':'dev.mercurio.value-result-original-native-query-evidence.v1','qualification_certificate':False,
     'native_bounded_queries_verified':12,'native_original_split':split,'complete_contexts_verified':0,
     'boundary':'All original Function.result selections match the independent cache using native imported parsing/construction and actual supplied Performances providers. Source and library links, complete lifecycle, semantic validation, publication, persistence and full-model comparisons remain unqualified.',
     'contexts':verified,'input_sha256':{'bundle':sha(b/'value-result-obligation-bundle.json'),
     'reference':sha(b/'value-result-reference-controls.json.gz'),'native_inspection':sha(raw),
     'native_run':sha(out/'value-result-parameter-plan-original-run.json')}}
    write_or_check(out/'value-result-parameter-plan-original-evidence.json', e, check)
    print(json.dumps({k:e[k] for k in ['native_bounded_queries_verified','native_original_split','complete_contexts_verified']},indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Verify evidence without rewriting it.")
    args = parser.parse_args()
    audit_actual_providers(args.check)
    audit_original_results(args.check)


if __name__ == "__main__":
    main()
