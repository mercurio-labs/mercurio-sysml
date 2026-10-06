"""Generate bounded transition-link role dispatch from resolved pinned source."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/reference-link-pilot-controls.json'
ECORE=ROOT/'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/reference_link_generated.rs'
TREE='5676d70d80d87d24e118d43d62a94d5ba48b82dfb32067b1d295090c4c98e7ba'
def generate(data):
 require(data['schema']=='dev.mercurio.reference-link-controls.v1','Changed Reference schema')
 ecore=json.loads(ECORE.read_text(encoding='utf-8'))
 classes={c['name']:c for c in ecore['classes']}
 require([t.split('#//')[-1] for t in classes['ReferenceUsage']['super_types']]==['Usage'] and [t.split('#//')[-1] for t in classes['TransitionUsage']['super_types']]==['ActionUsage'],'Changed adapter context types')
 role=next(f for f in ecore['features'] if f['owner'].endswith('#//Redefinition') and f['name']=='redefinedFeature')
 require(role['kind']=='reference' and role['type'].endswith('#//Feature') and role['lower_bound']==1 and role['upper_bound']==1 and not role['derived'],'Changed redefinition endpoint contract')
 program={'methods':data['methods'],'constants':data['constants']}
 require(hashlib.sha256(json.dumps(canonical(program),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Changed resolved Reference algorithms')
 constant=data['constants']['TRANSITION_LINK_FEATURE']
 require(constant['kind']=='STRING_LITERAL' and constant['type']=='java.lang.String','Unresolved role constant')
 matrix={(m,p,e) for m in ('FeatureMembership','EndFeatureMembership') for p in ('none','parameter','ordinary') for e in ('none','role','other','self')}
 require(len(data['controls'])==24 and {(c['membership'],c['preceding'],c['explicit']) for c in data['controls']}==matrix,'Incomplete controls')
 for c in data['controls']:
  selected=c['preceding']!='ordinary';explicit=[] if c['explicit']=='none' else [{'role':'transitionLink','other':'other','self':'candidate'}[c['explicit']]]
  require(c['selected']==selected and c['redefinitions']==explicit+(['transitionLink'] if selected else []),'Reference observation disagreement')
 return '\n'.join(['// Generated resolved ReferenceUsage role; canonical lookup is handwritten.',f'pub(super) const ROLE: &str = {json.dumps(constant["value"])};',f'pub(super) const PROGRAM_SHA: &str = "{TREE}";',''])
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args();code=generate(json.loads(INPUT.read_text(encoding='utf-8')))
 if a.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale Reference dispatch')
 else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
 print('Reference link: 5 resolved programs, 24 ordering/identity controls')
if __name__=='__main__':main()
