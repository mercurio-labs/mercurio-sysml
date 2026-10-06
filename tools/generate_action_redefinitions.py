"""Bounded Action/Accept policy generation from resolved trees and imported default maps.

Canonical membership, actual libraries and owner general-type traversal are named
handwritten native dependencies. This is not complete ActionUsage qualification.
"""
import argparse
import hashlib
import json
from pathlib import Path
from generate_expression_redefinitions import canonical, require, BINDINGS
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/action-redefinition-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/action_redefinitions_generated.rs'
PREFIX='org.omg.sysml.adapter.'
TREE_CONTRACT='3f292c9838df88619017ccba06615b1c178f5539540b92e6b97a11bc25c7f066'
CONTEXTS=('detached','ordinary','parameter','state:entry','state:do','state:exit','transition:trigger','transition:guard','transition:effect')
ROLES={'entry','do','exit','trigger','guard','effect'}

def generate(data, inventory):
    require(data['schema']=='dev.mercurio.action-redefinition-controls.v1','Changed Action schema')
    digest=hashlib.sha256(json.dumps(canonical(data['methods']),sort_keys=True,separators=(',',':')).encode()).hexdigest()
    require(digest==TREE_CONTRACT,'Resolved Action policy changed; review required')
    rows=[r for r in inventory['bindings'] if r['methods']['getRelevantFeatures']==PREFIX+'ActionUsageAdapter#getRelevantFeatures']
    kinds=sorted(r['kind'] for r in rows)
    require(len(kinds)==22 and sorted(data['bindings'])==kinds and sorted(data['defaults'])==kinds,'Changed Action binding inventory')
    for row in rows:
        require(all(data['bindings'][row['kind']][m]==v for m,v in row['methods'].items() if m in data['bindings'][row['kind']]),'Changed Action dispatch')
        for m in ('addRedefinitions','addFeatureWriteTypes','getGeneralTypes','getRedefinedFeaturesWithComputed'):
            require(row['methods'][m]==PREFIX+'FeatureAdapter#'+m,'Unsupported Action inherited dependency')
        require(row['methods']['isComputeRedefinitions']==PREFIX+'ActionUsageAdapter#isComputeRedefinitions','Changed Action compute gate')
        names=data['defaults'][row['kind']]
        require(set(names)==ROLES and all(v is None or isinstance(v,str) and '::' in v for v in names.values()),'Changed Action defaults')
    controls=data['controls']
    require(len(controls)==198 and {(c['kind'],c['context']) for c in controls}=={(k,c) for k in kinds for c in CONTEXTS},'Changed Action control inventory')
    for c in controls:
        role=c['context'].split(':',1)[1] if ':' in c['context'] else None
        expected=data['defaults'][c['kind']][role] if role else None
        require(c['default_redefined_feature']==expected and c['same_owner_relevant']==int(expected is not None),'Action selector observation disagrees')
        if c['kind']=='AcceptActionUsage':require(c['is_trigger_action']==(c['context']=='transition:trigger'),'Accept trigger observation disagrees')
    standard=sorted(k for k,d in data['bindings'].items() if d['addComputedRedefinitions']==PREFIX+'FeatureAdapter#addComputedRedefinitions')
    ordinary=data['ordinary_controls']
    require(len(standard)==19 and len(ordinary)==114 and {(c['kind'],c['owner_kind'],c['explicit']) for c in ordinary}=={(k,o,n) for k in standard for o in ('detached','Package','Class') for n in (0,2)},'Changed Action ordinary inventory')
    for c in ordinary:
        require(c['redefined_features']==['general'+str(i) for i in range(c['explicit'])] and c['effective_name']==('General0' if c['explicit'] else None) and c['effective_short_name'] is None,'Action ordinary observation disagrees')
    selector=data['methods'][PREFIX+'ActionUsageAdapter#getRedefinedFeature']['statements'][1]['initializer']
    state=selector['condition']['test_type'].split('.')[-1]
    transition=selector['false']['condition']['test_type'].split('.')[-1]
    trigger=data['methods'][PREFIX+'AcceptActionUsageAdapter#isTriggerAction']['statements'][1]['expression']['right']['right']['name'].lower()
    lines=['// Generated from resolved pinned Action policy and imported default-map bindings.',
           '// Membership projection, owner general types and library lookup are handwritten dependencies.',
           'pub(super) const STATE_MEMBERSHIP: &str = '+json.dumps(state)+';',
           'pub(super) const TRANSITION_MEMBERSHIP: &str = '+json.dumps(transition)+';',
           'pub(super) fn supports(kind: &str) -> bool { matches!(kind, '+ ' | '.join(map(json.dumps,kinds))+') }',
           'pub(super) fn redefined_name(kind: &str, role: &str) -> Option<&\'static str> { match (kind, role) {']
    for kind in kinds:
        for role,value in sorted(data['defaults'][kind].items()):
            if value is not None:lines.append('    ('+json.dumps(kind)+', '+json.dumps(role)+') => Some('+json.dumps(value)+'),')
    lines+=['    _ => None,','} }','pub(super) fn is_trigger_action(is_transition: bool, role: &str) -> bool { is_transition && role == '+json.dumps(trigger)+' }','pub(super) fn lifecycle_supported(kind: &str, trigger_action: bool) -> bool { match kind {']
    for kind in kinds:
        dispatch=data['bindings'][kind]['addComputedRedefinitions']
        if dispatch==PREFIX+'FeatureAdapter#addComputedRedefinitions':lines.append('    '+json.dumps(kind)+' => true,')
        elif dispatch==PREFIX+'AcceptActionUsageAdapter#addComputedRedefinitions':lines.append('    '+json.dumps(kind)+' => trigger_action,')
        else:require(dispatch in (PREFIX+'SendActionUsageAdapter#addComputedRedefinitions',PREFIX+'TerminateActionUsageAdapter#addComputedRedefinitions'),'Changed specialized Action lifecycle')
    lines+=['    _ => false,','} }','']
    return '\n'.join(lines)

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');args=p.parse_args()
    code=generate(json.loads(INPUT.read_text(encoding='utf-8')),json.loads(BINDINGS.read_text(encoding='utf-8')))
    if args.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale generated Action policy')
    else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
    print('22 Action policies, 198 membership controls; generated selection and lifecycle guards checked')
if __name__=='__main__':main()
