"""Guard the explicit normative supplement to Pilot's missing succession rule."""
import json,hashlib
from pathlib import Path
from generate_expression_redefinitions import require
from generate_variation import INPUT,ECORE,generate
ROOT=Path(__file__).resolve().parents[1]
PDF=ROOT.parent/'target/upstream/SysML-v2-Release/doc/2a-OMG_Systems_Modeling_Language.pdf'
PDF_SHA='d73a916885f564300bd1bd0cc7b7d6ca37c32e1553909399d7b1445153a210d5'
def validate(data,ecore,pdf_bytes):
 generate(data)
 require(hashlib.sha256(pdf_bytes).hexdigest()==PDF_SHA,'Changed normative SysML pin')
 classes={c['name']:c for c in ecore['classes']}
 def conforms(k,t):return k==t or any(conforms(p.split('#//')[-1],t) for p in classes[k]['super_types'])
 require(conforms('SuccessionAsUsage','Usage') and not classes['SuccessionAsUsage']['abstract'],'Changed normative Usage inheritance')
 rows=[c for c in data['controls'] if c['kind']=='SuccessionAsUsage' and c['variation'] and c['membership']=='VariantMembership' and (conforms(c['owner'],'Definition') or conforms(c['owner'],'Usage'))]
 require(len(rows)==73 and all(c['bound'] is False and c['pending']==[] for c in rows),'Changed independent adapter disagreement')
 return rows
if __name__=='__main__':
 data=json.loads(INPUT.read_text(encoding='utf-8'));ecore=json.loads(ECORE.read_text(encoding='utf-8'));print('Pinned normative succession supplement:',len(validate(data,ecore,PDF.read_bytes())),'required contexts')
