"""Bounded native dispatch/default recipes from resolved instantiation definitions."""
import argparse,json,hashlib
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/instantiation-strategy-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/instantiation_strategy_generated.rs'
DEFINITION_SHA='79b4b89853a9274a3b37de3676134010b4294d82335738b667e9447075b22bbc'
BINDINGS_SHA='251e26dd678662882a5362745684b3ec633bb411d09f65fc6a66ab35fa60f697'
def generate(d):
 require(d['schema']=='dev.mercurio.instantiation-strategy-controls.v1','Changed instantiation schema')
 descriptor={k:d[k] for k in ['bindings','generalization_classifier_ids']}
 require(hashlib.sha256(json.dumps(descriptor,sort_keys=True,separators=(',',':')).encode()).hexdigest()==BINDINGS_SHA,'Changed resolved dispatch, role mapping or classifier ordering')
 require(hashlib.sha256(json.dumps(canonical(d['definitions']),sort_keys=True,separators=(',',':')).encode()).hexdigest()==DEFINITION_SHA,'Changed resolved instantiation algorithms; review required')
 require(set(d['generalization_classifier_ids'])=={'FeatureTyping','Subsetting'} and all(isinstance(x,int) for x in d['generalization_classifier_ids'].values()),'Changed classifier ordering inventory')
 require(len(d['bindings'])==8 and len(d['membership_controls'])==20,'Changed instantiation inventory')
 admitted={k for k,v in d['bindings'].items() if v['selected_delegate'].endswith('.InstantiationExpression_instantiatedType_InvocationDelegate')}
 shapes={'empty','empty_then_class','feature_membership_then_class','class','feature','package_then_class','class_then_feature','feature_then_class','owning_class','empty_only'}
 require({(c['kind'],c['shape']) for c in d['membership_controls']}=={(k,s) for k in admitted for s in shapes},'Incomplete or duplicate membership control matrix')
 lines=['// Generated bounded dispatch/default recipes; algorithms and prerequisites are handwritten Rust.', '#[derive(Clone,Copy,Debug,PartialEq,Eq)]','pub(super) enum Selector { Membership, Operator, Trigger }', 'pub(super) const DEFINITION_SHA: &str = '+json.dumps(DEFINITION_SHA)+';', 'pub(super) fn selector(kind: &str)->Option<Selector> { match kind {']
 for k,v in sorted(d['bindings'].items()):
  name=v['selected_delegate'].rsplit('.',1)[-1]; category={'InstantiationExpression_instantiatedType_InvocationDelegate':'Membership','OperatorExpression_instantiatedType_InvocationDelegate':'Operator','TriggerInvocationExpression_instantiatedType_InvocationDelegate':'Trigger'}.get(name)
  require(category is not None,'Unreviewed instantiation delegate')
  lines.append(json.dumps(k)+' => Some(Selector::'+category+'),')
 lines+=['_ => None, }}']
 lines.append('pub(super) const TYPE_BEFORE_SUBSETTING: bool = '+str(d['generalization_classifier_ids']['FeatureTyping'] < d['generalization_classifier_ids']['Subsetting']).lower()+';')
 lines+=['pub(super) fn base(kind: &str)->Option<&\'static str> { match kind {']
 for k,v in sorted(d['bindings'].items()):lines.append(json.dumps(k)+' => Some('+json.dumps(v['role_bindings']['base'])+'),')
 lines+=['_ => None, }}','pub(super) fn adds_instantiated_general(kind: &str)->bool { matches!(kind, '+ ' | '.join(json.dumps(k) for k,v in sorted(d['bindings'].items()) if v['add_default_general_type']=='org.omg.sysml.adapter.InvocationExpressionAdapter#addDefaultGeneralType')+') }']
 # All imported contexts share ExpressionAdapter's resolved conditional effects.
 lines+=['pub(super) fn names(kind: &str, composite: bool, structure_owned: bool, behavior_owned: bool)->Option<Vec<&\'static str>> {','let mut result=vec![base(kind)?];','match kind {']
 for k,v in sorted(d['bindings'].items()):
  n=v['role_bindings'];lines +=[json.dumps(k)+' => {','if composite && structure_owned { result.push('+json.dumps(n['ownedPerformance'])+'); }','if composite && behavior_owned { result.push('+json.dumps(n['subperformance'])+'); }','if behavior_owned { result.push('+json.dumps(n['enclosedPerformance'])+'); }','},']
 lines+=['_ => return None, }','Some(result)','}']
 return '\n'.join(lines)+'\n'
def main():
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args();s=generate(json.loads(INPUT.read_text(encoding='utf-8')))
 if a.check:require(OUTPUT.read_text(encoding='utf-8')==s,'Stale instantiation recipe')
 else:OUTPUT.write_text(s,encoding='utf-8')
 print('Eight resolved instantiation recipes current; external consumers remain separately qualified')
if __name__=='__main__':main()
