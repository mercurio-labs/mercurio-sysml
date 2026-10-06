"""Generate reviewed ordered getter dispatch from resolved Pilot and imported Ecore."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/transition-projection-pilot-controls.json'
ECORE=ROOT/'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/transition_projections_generated.rs'
TREE='b404c0dd798bacb8aded4e84c8061c5a10e09b14123f5c2f005b494e67e8411e'
def generate(data,ecore):
 require(data['schema']=='dev.mercurio.transition-projection-controls.v1','Changed schema')
 require(hashlib.sha256(json.dumps(canonical(data['methods']),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Resolved projection algorithms changed')
 require(hashlib.sha256(json.dumps(data['feature_kinds'],separators=(',',':')).encode()).hexdigest()=='52782ea83e29a072d58828d18509d418327a57cf9a8d66d2d262cc6a8e5ac714','Changed concrete Feature inventory')
 require(data['fresh_transition_kind'] is None,'Changed unresolved constructor observation')
 classes={c['name']:c for c in ecore['classes']}
 def conforms(kind,target):return kind==target or any(conforms(p.split('#//')[-1],target) for p in classes[kind]['super_types'])
 def walk(n):
  yield n
  for child in n.get('children',[]):yield from walk(child)
 require(sorted(c['name'] for c in ecore['classes'] if not c['abstract'] and conforms(c['name'],'Feature'))==data['feature_kinds'],'Changed Ecore concrete Feature inventory')
 kind=next(f for f in ecore['features'] if f['owner'].endswith('#//TransitionFeatureMembership') and f['name']=='kind')
 require(kind['lower_bound']==1 and kind['upper_bound']==1 and kind['default_value']=='trigger' and not kind['derived'],'Changed required enum contract')
 rows=[]
 for field,name in [('trigger_action','triggerAction'),('guard_expression','guardExpression'),('effect_action','effectAction')]:
  cls='TransitionUsage_'+name+'_SettingDelegate';method=data['methods']['org.omg.sysml.delegate.setting.'+cls+'#basicGet'];nodes=list(walk(method))
  enums=[n for n in nodes if n['kind']=='MEMBER_SELECT' and n.get('type')=='org.omg.sysml.lang.sysml.TransitionFeatureKind'];require(len(enums)==1,'Changed role selection')
  checks=[n for n in nodes if n['kind']=='INSTANCE_OF'];require(len(checks)==1,'Changed range selection')
  ranges=[n['type'].split('.')[-1] for n in walk(checks[0]) if n['kind']=='IDENTIFIER' and n.get('type','').startswith('org.omg.sysml.lang.sysml.')];target=ranges[-1]
  contract=next(f for f in ecore['features'] if f['owner'].endswith('#//TransitionUsage') and f['name']==name)
  require(contract['type'].endswith('#//'+target) and contract['derived'] and contract['upper_bound']==-1 and not contract['ordered'] and contract['unique'],'Changed projection Ecore contract')
  rows.append((field,enums[0]['name'].lower(),target,'org.omg.sysml.delegate.setting.'+cls))
 matrix={(k,m,r,v) for k in data['feature_kinds'] for m in ('TransitionFeatureMembership','FeatureMembership','OwningMembership','Membership') for r in ('trigger','guard','effect','unset') for v in (False,True)}
 controls=data['controls'];require(len(controls)==2528 and {(c['kind'],c['membership'],c['role'],c['reverse']) for c in controls}==matrix,'Incomplete projection matrix')
 for c in controls:
  rejected=c['membership']=='TransitionFeatureMembership' and not conforms(c['kind'],'Step');require(c['rejected']==rejected,'Reference setter disagreement')
  if rejected:require(c['result']=={},'Invalid rejected result');continue
  ids=list('cba' if c['reverse'] else 'abc');expected={field:ids if c['membership']=='TransitionFeatureMembership' and c['role']==role and conforms(c['kind'],target) else [] for field,role,target,_ in rows}
  expected['succession']=ids[:1] if c['membership']!='Membership' and conforms(c['kind'],'Succession') else []
  require(c['result']==expected,'Reference projection disagreement')
 return '\n'.join(['// Generated from guarded resolved getter programs and Ecore ranges.',
  'pub(super) fn rule(field: &str) -> Option<(&\'static str,&\'static str,&\'static str)> { match field {',
  *[f' {json.dumps(f)} => Some(({json.dumps(r)},{json.dumps(t)},{json.dumps(d)})),' for f,r,t,d in rows],
  ' "succession" => Some(("","Succession","org.omg.sysml.delegate.setting.TransitionUsage_succession_SettingDelegate")),',
  ' _ => None,','} }',''])
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args();code=generate(json.loads(INPUT.read_text(encoding='utf-8')),json.loads(ECORE.read_text(encoding='utf-8')))
 if a.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale getter dispatch')
 else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
 print('4 getter dispatches / 79 concrete Features / 2528 controls checked')
if __name__=='__main__':main()
