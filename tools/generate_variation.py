"""Generate resolved Usage variation policy and actual adapter admission."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/variation-pilot-controls.json'
ECORE=ROOT/'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/variation_generated.rs'
TREE='ece1e5fd22cbe91bb5e76ecd36f8cbcd53e5f650c33bd97b0ab5b7df0bd9c521'
def generate(data):
 require(data['schema']=='dev.mercurio.variation-controls.v1','Changed variation schema')
 require(hashlib.sha256(json.dumps(canonical(data['methods']),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Changed resolved variation algorithm')
 ecore=json.loads(ECORE.read_text(encoding='utf-8'));classes={c['name']:c for c in ecore['classes']}
 def conforms(k,t):return k==t or any(conforms(p.split('#//')[-1],t) for p in classes[k]['super_types'])
 usages=sorted(k for k,c in classes.items() if not c['abstract'] and conforms(k,'Usage'))
 owners=sorted(k for k,c in classes.items() if not c['abstract'] and (conforms(k,'Usage') or conforms(k,'Definition')))+['Class','Package'];owners.sort()
 require(data['usage_kinds']==usages and data['owner_kinds']==owners,'Changed concrete binding inventory')
 matrix={(k,o,m,v) for k in usages for o in owners for m in ('VariantMembership','FeatureMembership','OwningMembership') for v in (False,True)}
 require(len(data['controls'])==len(matrix) and {(c['kind'],c['owner'],c['membership'],c['variation']) for c in data['controls']}==matrix,'Incomplete variation matrix')
 bound={};
 for c in data['controls']:
  require(c['kind'] not in bound or bound[c['kind']]==c['bound'],'Unstable adapter dispatch');bound[c['kind']]=c['bound']
  rel='FeatureTyping' if conforms(c['owner'],'Definition') else 'Subsetting' if conforms(c['owner'],'Usage') else None
  expected=[dict(kind=rel,target='owner')] if c['bound'] and c['membership']=='VariantMembership' and c['variation'] and rel else []
  require(c['pending']==expected,'Variation contribution disagreement')
  physical=c['bound'] and c['variation'] and c['membership']=='VariantMembership' and c['owner'] in ('Definition','Usage')
  require(c['physical_observed']==physical and c['physical']==([dict(kind=rel,target='owner',specific='candidate',implied=True,owned_related=0)] if physical else []),'Physical variation observation disagreement')
 def walk(n):
  yield n
  for c in n.get('children',[]):yield from walk(c)
 def context(method):
  types=[c.get('type','').split('.')[-1] for n in walk(data['methods']['org.omg.sysml.util.UsageUtil#'+method]) if n['kind']=='INSTANCE_OF' for c in n.get('children',[]) if c.get('type','').startswith('org.omg.sysml.lang.sysml.') and c.get('name')==c['type'].split('.')[-1]]
  require(len(types)==1,'Changed variation context predicate');return types[0]
 definition=context('getOwningVariationDefinitionFor');usage=context('getOwningVariationUsageFor');membership=context('isVariant')
 require((definition,usage,membership)==('Definition','Usage','VariantMembership'),'Changed variation types')
 names=[json.dumps(k) for k in usages if bound[k]]
 return '\n'.join(['// Generated bounded resolved variation policy; canonical projection/insertion is handwritten.',f'pub(super) const DEFINITION: &str = {json.dumps(definition)};',f'pub(super) const USAGE: &str = {json.dumps(usage)};',f'pub(super) const MEMBERSHIP: &str = {json.dumps(membership)};','pub(super) fn supports(kind: &str) -> bool { matches!(kind, '+ ' | '.join(names)+') }','pub(super) fn relationship(definition: bool, usage: bool) -> Option<&\'static str> { if definition { Some("FeatureTyping") } else if usage { Some("Subsetting") } else { None } }',''])
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args();code=generate(json.loads(INPUT.read_text(encoding='utf-8')))
 if a.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale variation policy')
 else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
 print('Variation policy: 46 inherited bindings; 21,150 independent dispatch/context controls')
if __name__=='__main__':main()
