"""Summarize raw comparison evidence; the projection is triage, never a waiver."""
import argparse, collections, gzip, json, statistics
from pathlib import Path
from release_expression_projection import compare as compare_expression


def load(path):
    data = Path(path).read_bytes()
    if str(path).endswith('.gz'): data = gzip.decompress(data)
    return json.loads(data)


def direct(element):
    return {k:v['direct_value'] for k,v in element['declared_attributes'].items()
            if v.get('has_direct_value') and 'direct_value' in v}


def pairs(document):
    sides=[]
    for side in ('mercurio_snapshot','pilot_snapshot'):
        index=collections.defaultdict(list)
        for e in document[side]['elements']:
            line=(e.get('source_span') or {}).get('start_line')
            if e.get('declared_name') and line is not None:
                index[(e['declared_name'], line)].append(e)
        sides.append(index)
    for key in sorted(sides[0].keys() & sides[1].keys()):
        if len(sides[0][key]) == len(sides[1][key]) == 1:
            yield sides[0][key][0], sides[1][key][0]


def normalized_relation(value, mapping):
    values = value if isinstance(value,list) else [value]
    if not all(isinstance(v,str) for v in values): return None
    return sorted(set(mapping.get(v,v) for v in values))


def main():
    ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--root',type=Path,default=Path('../target/assessment-2026-08-detailed'));a=ap.parse_args();root=a.root
    summary=load(root/'semantic-summary.json');rows=summary['cases'];aggregate=collections.Counter();raw_fields=collections.Counter();n_only=collections.Counter();p_only=collections.Counter();triage=[];projection=collections.Counter();case_rows=[]
    expression_rows=[]
    expression_counts=collections.Counter()
    group_maps=collections.defaultdict(dict)
    observed_maps=collections.defaultdict(lambda:collections.defaultdict(set))
    for row in rows:
        if row['status']!='compared':continue
        doc=load(row['artifact']['path'])
        for n,p in pairs(doc): observed_maps[row['group']][n['id']].add(p['id'])
    for group,ids in observed_maps.items():
        group_maps[group]={key:next(iter(values)) for key,values in ids.items() if len(values)==1}
    for row in rows:
        aggregate[row['status']]+=1
        if row['status']!='compared':continue
        doc=load(row['artifact']['path']);report=doc['report'];nc=len(doc['mercurio_snapshot']['elements']);pc=len(doc['pilot_snapshot']['elements']);mapping=group_maps[row['group']]
        for key in ['native_elements','pilot_elements','exact_pairs','mismatched_pairs','native_only','pilot_only']:aggregate[key]+=row[key]
        n_only.update(row['native_only_kinds']);p_only.update(row['pilot_only_kinds'])
        for m in report['mismatches']:
            av=m.get('declared_attributes')
            if av:
                for key in sorted(av['mercurio'].keys()|av['pilot'].keys()):
                    if av['mercurio'].get(key)!=av['pilot'].get(key):raw_fields[key]+=1
        local=collections.Counter()
        named_indexes=[]
        for side in ('mercurio_snapshot','pilot_snapshot'):
            named=collections.defaultdict(list)
            for element in doc[side]['elements']:
                line=(element.get('source_span') or {}).get('start_line')
                if element.get('declared_name') and line is not None:
                    named[(element['declared_name'],line)].append(element)
            named_indexes.append(named)
        for key in sorted(named_indexes[0].keys() | named_indexes[1].keys()):
            left=named_indexes[0].get(key,[]);right=named_indexes[1].get(key,[])
            if len(left)>1 or len(right)>1: projection['ambiguous_named_anchors']+=1
            elif left and not right: projection['native_named_anchors_without_pilot_pair']+=1
            elif right and not left: projection['pilot_named_anchors_without_native_pair']+=1
        pilot_elements={e['id']:e for e in doc['pilot_snapshot']['elements']}
        initializers=collections.defaultdict(list)
        for element in doc['pilot_snapshot']['elements']:
            if element['kind'].split('::')[-1] != 'FeatureValue': continue
            fields=direct(element)
            for source in fields.get('source',[]):
                initializers[source].extend(fields.get('target',[]))
        for n,p in pairs(doc):
            projection['unique_named_pairs']+=1;local['unique_named_pairs']+=1
            native=direct(n);pilot=direct(p)
            if p['id'] in initializers:
                projection['named_declarations_with_pilot_initializer']+=1
                expression=native.get('expression_ir')
                expression_result=compare_expression(expression, initializers[p['id']], mapping, pilot_elements, direct)
                expression_counts[expression_result['status']]+=1
                expression_rows.append({'source':row['relative_path'],'line':(n.get('source_span') or {}).get('start_line'),'name':n['declared_name'],**expression_result})
                if expression is None:
                    projection['pilot_initializer_without_native_expression_ir']+=1
                    local['pilot_initializer_without_native_expression_ir']+=1
                    triage.append({'source':row['relative_path'],'line':(n.get('source_span') or {}).get('start_line'),'name':n['declared_name'],'property':'initializer','native':None,'pilot':initializers[p['id']],'classification':'Pilot FeatureValue has no expression_ir on matched native declaration; inspect alternate lowering before classifying loss'})
                elif isinstance(expression,dict) and expression.get('kind')=='literal':
                    for target in initializers[p['id']]:
                        value=pilot_elements.get(target)
                        if not value or not value['kind'].split('::')[-1].startswith('Literal'): continue
                        fields=direct(value)
                        if 'value' not in fields: continue
                        projection['literal_initializers_checked']+=1
                        if expression.get('value') != fields['value'] or isinstance(expression.get('value'),bool) != isinstance(fields['value'],bool):
                            projection['literal_initializer_mismatches']+=1
                            triage.append({'source':row['relative_path'],'line':(n.get('source_span') or {}).get('start_line'),'name':n['declared_name'],'property':'initializer_literal_value','native':expression.get('value'),'pilot':fields['value'],'classification':'literal initializer value difference'})
            nk=(n.get('metatype') or n['kind']).split('::')[-1];pk=(p.get('metatype') or p['kind']).split('::')[-1]
            if nk!=pk:
                projection['metaclass_mismatches']+=1;local['metaclass_mismatches']+=1
                triage.append({'source':row['relative_path'],'line':(n.get('source_span') or {}).get('start_line'),'name':n['declared_name'],'property':'metaclass','native':nk,'pilot':pk,'classification':'declaration-level difference'})
            for left,right in [('type','type'),('definition','definition'),('owner','owner'),('specializes','specializes'),('subsetted_features','subsets'),('redefined_features','redefines')]:
                if left not in native or right not in pilot:
                    if left in native or right in pilot:
                        projection['relation_fields_missing_direct_counterpart']+=1
                        local['relation_fields_missing_direct_counterpart']+=1
                    continue
                x=normalized_relation(native[left],mapping);y=normalized_relation(pilot[right],{})
                if x is None or y is None:continue
                projection['relation_fields_checked']+=1
                if x==y:
                    projection['relation_fields_equal_after_explicit_projection']+=1
                else:
                    projection['relation_field_differences']+=1;local['relation_field_differences']+=1
                    triage.append({'source':row['relative_path'],'line':(n.get('source_span') or {}).get('start_line'),'name':n['declared_name'],'property':right,'native':x,'pilot':y,'classification':'unresolved relation difference; may include generated implicit relationships'})
            for key in ('declared_short_name','direction'):
                if pilot.get(key) not in (None,'') and native.get(key)!=pilot[key]:
                    projection['name_or_direction_differences']+=1;local['name_or_direction_differences']+=1
                    triage.append({'source':row['relative_path'],'line':(n.get('source_span') or {}).get('start_line'),'name':n['declared_name'],'property':key,'native':native.get(key),'pilot':pilot[key],'classification':'explicit name/direction field absent or different in native projection'})
            for key in ('is_abstract','is_variable','is_unique','is_ordered','is_derived','is_end'):
                if key in native and key in pilot and native[key]!=pilot[key]:
                    projection['boolean_differences']+=1;local['boolean_differences']+=1
                    triage.append({'source':row['relative_path'],'line':(n.get('source_span') or {}).get('start_line'),'name':n['declared_name'],'property':key,'native':native[key],'pilot':pilot[key],'classification':'direct Boolean difference'})
            for key in ('is_composite','is_reference','is_portion','is_constant','is_variation','may_time_vary','is_individual','is_standard'):
                if key not in native or key not in pilot:
                    if key in native or key in pilot:
                        projection['extended_boolean_fields_missing_counterpart']+=1
                        local['extended_boolean_fields_missing_counterpart']+=1
                    continue
                projection['extended_boolean_fields_checked']+=1
                if native[key]!=pilot[key]:
                    projection['extended_boolean_differences']+=1;local['extended_boolean_differences']+=1
                    triage.append({'source':row['relative_path'],'line':(n.get('source_span') or {}).get('start_line'),'name':n['declared_name'],'property':key,'native':native[key],'pilot':pilot[key],'classification':'direct extended Boolean difference'})
        case_rows.append({**{k:row[k] for k in ('relative_path','native_elements','pilot_elements','exact_pairs','mismatched_pairs','native_only','pilot_only')},'named_declaration_triage':dict(local),'artifact':row['artifact']})
    result={'scope':{'requested_targets':sum(len(group['cases']) for group in load(root/'source-lock.json')['groups']),'recorded_targets':len(rows)},'raw_aggregate':dict(aggregate),'pilot_validation_statuses':dict(collections.Counter(r.get('pilot_validation_status','unavailable') for r in rows)),'raw_attribute_difference_counts':dict(raw_fields.most_common()),'native_only_kinds':dict(n_only.most_common()),'pilot_only_kinds':dict(p_only.most_common()),'named_declaration_projection':dict(projection),'projection_rules':['Match only unique (declared name, start line) pairs within one source file.','Reference IDs map only through those observed pairs in the same isolated source group.','For the six enumerated reference fields only, scalar/singleton-array and duplicate/order differences are reported as equal sets.','Missing direct fields, anonymous expressions and unpaired elements are not declared equivalent.','No raw mismatch is removed; projection results are triage, not a conformance pass.'],'differences':triage,'cases':case_rows,'failures':[r for r in rows if r['status']!='compared']}
    result['ordered_expression_projection']={'counts':dict(expression_counts),'scope':'Named initializer value trees only; equality does not establish typing, implicit relationships, multiplicities or evaluation parity. Unsupported forms remain unassessed.','cases':expression_rows}
    (root/'semantic-analysis.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'scope':result['scope'],'raw':dict(aggregate),'projection':dict(projection)},indent=2))
    if (root/'performance-results.json').exists():
        data=load(root/'performance-results.json');groups=collections.defaultdict(lambda:collections.defaultdict(list));failures=[]
        for m in data['measurements']:
            timing_cases=(m.get('timings') or {}).get('cases',[])
            expected=sorted(m['cases']);observed=sorted(c['relative_path'] for c in timing_cases)
            if m['execution']['returncode']!=0 or not m['timings'] or expected!=observed or any(c['status']!='ok' for c in timing_cases):
                failures.append({'group':m['group'],'engine':m['engine'],'repetition':m['repetition'],'execution':m['execution'],'target_set_matches':expected==observed,'failed_cases':[c for c in timing_cases if c['status']!='ok']})
                continue
            phases=m['timings']['phases'];work=sum(v for k,v in phases.items() if k not in ('initialize_ms','load_library_ms'))
            groups[m['group']][m['engine']].append({'wall_ms':m['execution']['wall_ms'],'engine_ms':m['timings']['engine_total_ms'],'library_ms':phases['load_library_ms'],'model_work_ms':work})
        table=[]
        for group,engines in groups.items():
            entry={'group':group}
            for engine,values in engines.items():
                entry[engine]={'repetitions':len(values),**{k:{'median':statistics.median(v[k] for v in values),'min':min(v[k] for v in values),'max':max(v[k] for v in values)} for k in values[0]}}
            if 'native' in entry and 'pilot' in entry:
                entry['pilot_over_native_wall']=entry['pilot']['wall_ms']['median']/entry['native']['wall_ms']['median']
                entry['pilot_over_native_model_work']=entry['pilot']['model_work_ms']['median']/entry['native']['model_work_ms']['median']
            table.append(entry)
        perf={'method':data['method'],'successful_measurements':sum(len(v) for e in groups.values() for v in e.values()),'total_measurements':len(data['measurements']),'failures':failures,'groups':table,'caveat':'Native validation and semantic preservation are incomplete. These workflow timings do not establish equivalent work or a conformance-adjusted speedup.'}
        (root/'performance-analysis.json').write_text(json.dumps(perf,indent=2)+'\n',encoding='utf-8')


if __name__=='__main__':main()
