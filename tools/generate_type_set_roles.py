"""Reviewed type-set roles from resolved Ecore/Xtext. Delegate algorithms remain named handwritten Rust."""
import argparse
import hashlib
import json
import re
from pathlib import Path
from generate_type_relationship_scope import derive as grammar_bindings
ROOT=Path(__file__).resolve().parents[1]
PROFILE=ROOT/'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08'
KINDS=('Differencing','Disjoining','Intersecting','Unioning')

def snake(name):
    return re.sub(r'(?<!^)(?=[A-Z])','_',name).lower()

def derive(grammar,ecore,semantics):
    grammar_rows=grammar_bindings(grammar,ecore)
    features={f['id']:f for f in ecore['features']}
    bindings={b['element']:b for b in semantics['delegate_bindings'] if b['kind']=='setting'}
    def declared(owner,name):
        candidates=[f for f in features.values() if f['owner'].endswith('#//'+owner) and f['name']==name]
        if len(candidates)!=1:raise ValueError('Changed type-set feature inventory')
        return candidates[0]
    def delegate(f,fallback=False):
        b=bindings[f['id']]
        expected='org.omg.sysml.delegate.setting.'+f['owner'].split('#//')[-1]+'_'+f['name']+'_SettingDelegate'
        valid=(b['binding_status']=='default_setting_delegate_fallback_source' and b['candidate_source_classes']==[] and b.get('fallback_class')=='org.omg.sysml.delegate.setting.DefaultDerivedPropertySettingDelegate') if fallback else (b['binding_status']=='custom_setting_delegate_source' and b['candidate_source_classes']==[expected])
        if b['delegate_uri']!='http://www.omg.org/spec/SysML' or not valid:
            raise ValueError('Unreviewed type-set delegate')
        return b
    rows=[]
    for kind in KINDS:
        # Explicit semantic role selection is reviewed, not inferred from a method signature.
        candidates=[f for f in ecore['features'] if f['owner'].endswith('#//'+kind) and f['opposite'] and '/owned' in f['opposite']]
        if len(candidates)!=1:raise ValueError('Unreviewed source/opposite role')
        owning=candidates[0];owned=features[owning['opposite']]
        refs=[r for r in grammar_rows if r['kind']==kind]
        target_candidates={r['ecore_feature'] for r in refs if not r['field'].startswith('type_')}
        if len(target_candidates)!=1:raise ValueError('Unreviewed grammar endpoint role')
        target=features[next(iter(target_candidates))]
        source=declared(kind,'typeDisjoined') if kind=='Disjoining' else owning
        for f in [source,target,owning]:
            if f['kind']!='reference' or not f['type'].endswith('#//Type') or f['upper_bound']!=1 or f['containment']:
                raise ValueError('Unreviewed scalar Type role')
        if source['lower_bound']!=1 or target['lower_bound']!=1 or target['derived'] or target['volatile'] or target['transient']:
            raise ValueError('Changed required stored endpoint')
        if source['derived']!=(kind!='Disjoining') or source['volatile']!=source['derived'] or source['transient']!=source['derived']:
            raise ValueError('Changed source storage contract')
        if not owning['derived'] or owned['opposite']!=owning['id'] or owned['type']!=owning['owner'] or owned['upper_bound']!=-1 or not owned['unique']:
            raise ValueError('Changed reciprocal owned projection')
        delegate(owning);delegate(owned,kind!="Disjoining")
        endpoint=None
        if kind!='Disjoining':
            endpoint=declared('Type',target['name'])
            if not endpoint['derived'] or endpoint['upper_bound']!=-1 or not endpoint['ordered'] or not endpoint['unique'] or endpoint['type']!=target['type']:
                raise ValueError('Changed endpoint projection')
            delegate(endpoint)
        rows.append({'kind':kind,'source':source['id'],'source_field':snake(source['name']),
                     'source_derived':source['derived'],'target':target['id'],'target_field':snake(target['name']),
                     'owning_source':owning['id'],'owning_field':snake(owning['name']),
                     'owned_projection':owned['id'],'owned_field':snake(owned['name']),
                     'endpoint_projection':endpoint['id'] if endpoint else None,
                     'endpoint_field':snake(endpoint['name']) if endpoint else None,
                     'cardinality_constraint':'validateTypeOwned'+kind+'NotOne' if endpoint else None,
                     'self_constraint':'validateType'+kind+'TypesNotSelf' if endpoint else None,
                     'grammar_references':[r['cross_reference'] for r in refs],
                     'algorithm':'handwritten canonical owningRelatedElement projection, stored endpoint reads, kind selection and normative count/self predicates'})
    return rows

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    paths=[PROFILE/n for n in ['grammar.structure.extract.json','ecore-effective.extract.json','ecore-semantics.extract.json']]
    rows=derive(*(json.loads(p.read_text(encoding='utf-8')) for p in paths))
    artifact={'schema':'dev.mercurio.type-set-roles.v1','qualification_certificate':False,
              'scope':'Four reviewed relationship roles and seven Type projections; imported dispatch is not implementation.',
              'inputs':{p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},'bindings':rows}
    rust='// Generated from resolved Ecore/Xtext by tools/generate_type_set_roles.py.\n'
    rust+='// Execution of delegates and normative predicates is explicit handwritten Rust.\n'
    rust+='pub struct Binding {pub kind: &\'static str, pub source: &\'static str, pub derived_source: bool, pub target: &\'static str, pub owning: &\'static str, pub owned: &\'static str, pub endpoints: Option<&\'static str>, pub count_constraint: Option<&\'static str>, pub self_constraint: Option<&\'static str>}\n'
    rust+='pub const BINDINGS: &[Binding] = &[\n'
    option=lambda s:'Some('+json.dumps(s)+')' if s else 'None'
    for row in rows:
        rust+='    Binding {kind: '+json.dumps(row['kind'])+', source: '+json.dumps(row['source_field'])+', derived_source: '+str(row['source_derived']).lower()+', target: '+json.dumps(row['target_field'])+', owning: '+json.dumps(row['owning_field'])+', owned: '+json.dumps(row['owned_field'])+', endpoints: '+option(row['endpoint_field'])+', count_constraint: '+option(row['cardinality_constraint'])+', self_constraint: '+option(row['self_constraint'])+'},\n'
    rust+='];\npub fn binding(kind:&str)->Option<&\'static Binding> {let kind=kind.rsplit("::").next().unwrap_or(kind);BINDINGS.iter().find(|b|b.kind==kind)}\n'
    for p,text in [(PROFILE/'type-set-roles.extract.json',json.dumps(artifact,indent=2)+'\n'),(ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/type_set_roles_generated.rs',rust)]:
        if args.check:
            if p.read_text(encoding='utf-8')!=text:raise ValueError('Stale type-set roles '+str(p))
        else:p.write_text(text,encoding='utf-8')
    print('Four resolved type-set roles; seven projections; six named handwritten predicates.')
if __name__=='__main__':main()
