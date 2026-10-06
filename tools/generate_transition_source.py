"""Generate guarded source-stage policy; graph algorithms are explicitly handwritten."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/transition-source-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/transition_source_generated.rs'
TREE='0fef28311e016939ac037c38f4f5f8d081320190e313e17f284320a318cfb3cc'
def generate(data):
 require(data['schema']=='dev.mercurio.transition-source-controls.v1','Changed source schema')
 methods=data['methods']
 require(hashlib.sha256(json.dumps(canonical(methods),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Resolved source algorithms changed; review required')
 matrix={(o,p,f) for o in ('detached','Package','Class','ActionDefinition','StateDefinition') for p in ('none','Feature','StateUsage','TransitionUsage','BindingConnector','message','end_flow','directed','nonfeature') for f in ('none','parameter','empty_alias','explicit_alias')}
 controls=data['controls'];require(len(controls)==180 and {(c['owner'],c['previous'],c['first']) for c in controls}==matrix,'Incomplete source matrix')
 for c in controls:
  eligible=c['owner'] not in ('detached','Package') and c['previous'] not in ('none','TransitionUsage','BindingConnector','end_flow','nonfeature') and not(c['previous']=='directed' and c['owner'] in ('ActionDefinition','StateDefinition'))
  source='previous' if eligible else None
  expected=[{'membership':'Membership','member':'explicit' if c['first']=='explicit_alias' else source}]
  if c['first']=='parameter':expected.append({'membership':'ParameterMembership','member':'parameter'})
  before=[] if c['first']=='none' else [{'membership':'ParameterMembership','member':'parameter'}] if c['first']=='parameter' else [{'membership':'Membership','member':'explicit' if c['first']=='explicit_alias' else None}]
  require(c['before']==before and c['after']==expected and c['replay']==expected,'Reference source disagreement')
 def walk(n):
  yield n
  for child in n.get('children',[]):yield from walk(child)
 source=methods['org.omg.sysml.adapter.TransitionUsageAdapter#computeSource']
 factory=[n for n in walk(source) if n['kind']=='METHOD_INVOCATION' and n.get('symbol')=='org.omg.sysml.lang.sysml.SysMLFactory#createMembership']
 require(len(factory)==1,'Changed source factory')
 # These predicates execute the reviewed resolved body. No general translator is claimed.
 return '\n'.join(['// Generated reviewed policy. Ownership projection/execution are handwritten.',
  'pub(super) const MEMBERSHIP: &str = '+json.dumps(factory[0]['type'].split('.')[-1])+';',
  'pub(super) fn insert(empty: bool, first_parameter: bool) -> bool { empty || first_parameter }',
  'pub(super) fn eligible(feature: bool, parameter: bool, transition: bool, connector: bool, message: bool) -> bool { feature && !parameter && !transition && (!connector || message) }',''])
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args();code=generate(json.loads(INPUT.read_text(encoding='utf-8')))
 if a.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale source policy')
 else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
 print('Transition source policy: 4 resolved bodies / 180 controls checked')
if __name__=='__main__':main()
