"""Bounded contribution dispatch from resolved pinned operator/trigger adapter programs."""
import argparse,hashlib,itertools,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/instantiation-contribution-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/instantiation_contributions_generated.rs'
DEFINITION_SHA='03fb47a21509ae7313192e495b015dacc02e709c3b8e3095becffb8bff93114f'
def generate(d):
 require(d['schema']=='dev.mercurio.instantiation-contribution-controls.v1','Changed contribution schema')
 require(hashlib.sha256(json.dumps(canonical(d['definitions']),sort_keys=True,separators=(',',':')).encode()).hexdigest()==DEFINITION_SHA,'Changed resolved contribution algorithms')
 expected={k:'org.omg.sysml.adapter.'+('TriggerInvocationExpressionAdapter' if k=='TriggerInvocationExpression' else 'OperatorExpressionAdapter')+'#computeImplicitGeneralTypes/0' for k in ['OperatorExpression','CollectExpression','SelectExpression','IndexExpression','FeatureChainExpression','TriggerInvocationExpression']}
 require(d['bindings']==expected,'Changed resolved adapter dispatch')
 matrix={(k,v,t,c) for k in expected for v in (['null','when','at','after'] if k=='TriggerInvocationExpression' else ['default','null','+']) for t in ['absent','Package','Class','Feature','Function','self'] for c in ['detached','package']}
 actual=[(c['kind'],c['variant'],c['value'],c['context']) for c in d['controls']]
 require(len(actual)==len(set(actual)) and set(actual)==matrix,'Incomplete or duplicate contribution controls')
 require({c['shape'] for c in d['equivalence_controls']}=={'identity','empty','one_same','one_different','two_same','two_reverse','different_length','non_feature'} and len(d['equivalence_controls'])==8,'Incomplete equivalence controls')
 return '// Generated adapter admission; canonical effect composition/equivalence are named handwritten Rust.\n'+'pub(super) const DEFINITION_SHA: &str = '+json.dumps(DEFINITION_SHA)+';\n'+'pub(super) fn direct_typing(kind: &str)->bool { matches!(kind, '+ ' | '.join(json.dumps(k) for k in sorted(expected))+') }\n'
def main():
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args();s=generate(json.loads(INPUT.read_text(encoding='utf-8')))
 if a.check:require(OUTPUT.read_text(encoding='utf-8')==s,'Stale contribution dispatch')
 else:OUTPUT.write_text(s,encoding='utf-8',newline='\n')
 print('Six resolved adapter bindings current; complete contexts/lifecycle remain separate')
if __name__=='__main__':main()
