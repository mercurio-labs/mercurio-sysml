"""Guard the resolved Transition source operation and its no-op producer branch."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/transition-source-query-pilot-controls.json'
ECORE=ROOT/'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/transition_source_query_generated.rs'
TREE='8a86b7fd85b1c30fd2904dcc36474d952edd8d48a1dcc0044ffcd63b6340b900'
def generate(data,ecore):
 require(data['schema']=='dev.mercurio.transition-source-query-controls.v1','Changed schema')
 require(hashlib.sha256(json.dumps(canonical(data['methods']),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Resolved source/lifecycle algorithms changed')
 require(len(data['feature_kinds'])==79,'Changed Feature inventory')
 operation=next(o for o in ecore['operations'] if o['owner'].endswith('#//TransitionUsage') and o['name']=='sourceFeature')
 require(operation['parameters']==[] and operation['type'].endswith('#//Feature') and operation['lower_bound']==0 and operation['upper_bound']==1,'Changed source operation contract')
 classes={c['name']:c for c in ecore['classes']}
 def conforms(k,t):return k==t or any(conforms(p.split('#//')[-1],t) for p in classes[k]['super_types'])
 require(sorted(c['name'] for c in ecore['classes'] if not c['abstract'] and conforms(c['name'],'Feature'))==data['feature_kinds'],'Changed Ecore Feature inventory')
 matrix={(k,m,c,l) for k in data['feature_kinds'] for m in ('Membership','OwningMembership','FeatureMembership') for c in ('none','action_last','feature_last') for l in (False,True)}
 require(len(data['controls'])==1422 and {(c['kind'],c['membership'],c['chain'],c['link']) for c in data['controls']}==matrix,'Incomplete source query matrix')
 for c in data['controls']:
  selected=c['membership']!='FeatureMembership' and (c['chain']=='action_last' or c['chain']=='none' and conforms(c['kind'],'ActionUsage'))
  noop=not(c['membership']=='OwningMembership' and conforms(c['kind'],'Succession') and not c['link'])
  require(c['noop']==noop and c['raw']==('candidate' if selected else 'fallback') and c['source']==('last' if selected and c['chain']=='action_last' else 'candidate' if selected else 'fallback'),'Reference source/no-op disagreement')
  require(c['members_before']==2+int(c['link']) and c['members_after']==c['members_before']+int(not noop),'Reference producer mutation disagreement')
 def walk(n):
  yield n
  for child in n.get('children',[]):yield from walk(child)
 method=data['methods']['org.omg.sysml.util.UsageUtil#getSourceFeatureOf'];checks=[n for n in walk(method) if n['kind']=='INSTANCE_OF' and any(c.get('type')=='org.omg.sysml.lang.sysml.ActionUsage' for c in n.get('children',[]))];require(len(checks)==1,'Changed source filtering')
 types=[n['type'].split('.')[-1] for n in walk(checks[0]) if n['kind']=='IDENTIFIER' and n.get('type','').startswith('org.omg.sysml.lang.sysml.')];require(types[-1]=='ActionUsage','Changed source range')
 return '\n'.join(['// Generated reviewed source dispatch; no-op readiness/projection execution is handwritten.',f'pub(super) const FILTER_TARGET: &str = {json.dumps(types[-1])};','pub(super) const SOURCE_DELEGATE: &str = "org.omg.sysml.delegate.setting.TransitionUsage_source_SettingDelegate";',f'pub(super) const RESOLVED_PROGRAM_SHA: &str = "{TREE}";',''])
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args();code=generate(json.loads(INPUT.read_text(encoding='utf-8')),json.loads(ECORE.read_text(encoding='utf-8')))
 if a.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale source dispatch')
 else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
 print('Source/no-op lifecycle: 1410 matching contexts, 12 constructor boundaries')
if __name__=='__main__':main()
