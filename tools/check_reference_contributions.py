"""Validate bounded resolved ReferenceUsage physical contribution observations."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
from generate_reference_link import INPUT as QUERY_INPUT,generate
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/reference-contribution-pilot-controls.json'
TREE='aafd76d62aa3077dcd7ab7fc12f14d24f888e04dacb46ce6a981b880a0480617'
def validate(data):
 require(data['schema']=='dev.mercurio.reference-contribution-controls.v1','Changed contribution schema')
 require(hashlib.sha256(json.dumps(canonical({'methods':data['methods'],'constants':data['constants']}),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Changed resolved insertion/reduction algorithms')
 query=json.loads(QUERY_INPUT.read_text(encoding='utf-8'));generate(query)
 require(data['constants']==query['constants'] and all(data['methods'][k]==v for k,v in query['methods'].items()),'Contribution/query program mismatch')
 matrix={(m,p,e) for m in ('FeatureMembership','EndFeatureMembership') for p in ('none','parameter') for e in ('none','role','other')}
 require(len(data['controls'])==12 and {(c['membership'],c['preceding'],c['explicit']) for c in data['controls']}==matrix,'Incomplete physical controls')
 for c in data['controls']:
  explicit=[] if c['explicit']=='none' else [{'role':'transitionLink','other':'other'}[c['explicit']]]
  expected=[dict(kind='Redefinition',specific='candidate',target=name,implied=False) for name in explicit]
  if c['explicit']!='role':expected.append(dict(kind='Redefinition',specific='candidate',target='transitionLink',implied=True))
  require(c['selected'] and c['complete'] is False and c['stored']==expected,'Physical contribution disagreement')
  require(c['redefinitions']==explicit+['transitionLink'] and c['after']==[r['target'] for r in expected],'Changed adapter-state/query ordering')
 return 12
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.parse_args();print('Reference physical contribution controls:',validate(json.loads(INPUT.read_text(encoding='utf-8'))))
