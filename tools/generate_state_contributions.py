"""Bounded State contributions; Ecore invocation and canonical inputs are handwritten Rust."""
import argparse,json
from pathlib import Path
from generate_action_contributions import ActionTranslator,CONTEXTS
from generate_occurrence_contributions import sha
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/state-contribution-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/state_contributions_generated.rs'
TREE='30ee827a6148d8a33985d7f45fa355f0082b5e00189e4e681f715ad828da3714'
BINDINGS='efe6c1f4482b81777ec3a9e69634287a6aca99eb15c68ac52cef67cf3e78d65c'
OPERATION='a8bc61206fc0f7369ade4506e7937a10ef8f1efd51287d6eef210f4e0eda3af1'
class StateTranslator(ActionTranslator):
 def expression(self,n,locals=None,stack=()):
  if n['kind']=='METHOD_INVOCATION':
   name=n['symbol'].split('#')[-1]
   if name=='isSubstateUsage':
    require(n['symbol']=='org.omg.sysml.lang.sysml.StateUsage#isSubstateUsage' and len(n['arguments'])==1 and n['arguments'][0]['kind']=='BOOLEAN_LITERAL','Changed Ecore invocation')
    return 'c.substate' if n['arguments'][0]['value'] else 'c.exclusive_state'
   if name in ('isExclusiveState','isSubstate'):
    key=self.binding['dispatch'][name];require(key not in stack,'Cyclic State predicate');return self.return_expression(self.methods[key],(*stack,key))
  return super().expression(n,locals,stack)
def generate(data):
 require(data['schema']=='dev.mercurio.state-contribution-controls.v1','Changed State schema')
 require(sha(canonical(data['methods']))==TREE,'Resolved State programs changed')
 require(sha(data['bindings'])==BINDINGS and set(data['bindings'])=={'StateUsage'},'Changed State dispatch/maps')
 require(sha(data['operation_binding'])==OPERATION,'Changed resolved Ecore invocation dispatch')
 matrix={(c,b,m,p,o) for c in (*CONTEXTS,'state_entry','state_do','state_exit') for b in (False,True) for m in range(8) for p in ('none','snapshot','timeslice') for o in (False,True)}
 require(len(data['controls'])==1632 and {(c['context'],c['composite'],c['typing_mask'],c['portion'],c['owner_parallel']) for c in data['controls']}==matrix,'Incomplete State contribution matrix')
 ops={(c,o,b,r,m,a) for c in ('detached','Package','Class','StateDefinition','StateUsage') for o in (False,True) for b in (False,True) for r in (False,True) for m in ('ordinary','entry','do','exit') for a in (False,True)}
 require(len(data['operation_controls'])==320 and {(c['context'],c['owner_parallel'],c['composite'],c['receiver_parallel'],c['membership'],c['argument']) for c in data['operation_controls']}==ops,'Incomplete Ecore invocation controls')
 for c in data['operation_controls']:
  expected=c['composite'] and c['context'] in ('StateDefinition','StateUsage') and c['owner_parallel']==c['argument'] and c['membership']=='ordinary'
  require(c['result']==expected,'Independent Ecore operation disagrees with reviewed algorithm')
 require(len(data['owned_member_controls'])==4 and {c['child_kind'] for c in data['owned_member_controls']}=={'Class','DataType','ReferenceUsage','FeatureReferenceExpression'},'Incomplete nested State declaration controls')
 for c in data['owned_member_controls']:
  require(c['kind']=='StateUsage' and c['generals']==[{'kind':'Subsetting','target':data['bindings']['StateUsage']['names']['base']}],'Nested State declaration changes contributions')
 for c in data['controls']:
  require(c['kind']=='StateUsage' and c['source_kind']=='none','Unassessed State input')
  require(all(r['kind']=='Subsetting' and r['target'] in data['bindings']['StateUsage']['names'].values() for r in c['generals']),'Unresolved State contribution observation')
 b=data['bindings']['StateUsage']
 lines=['// Generated from resolved State/Action inherited predicates and imported default map.', '// Handwritten dependencies: canonical ownership/typing, selected Ecore invocation and entry/exit membership.', "pub(super) struct Inputs<'a> {pub composite:bool,pub structure:bool,pub data:bool,pub owner_kind:&'a str,pub owner_class:bool,pub owner_structure:bool,pub portion:&'a str,pub entry_exit:bool,pub exclusive_state:bool,pub substate:bool}","impl Inputs<'_> {fn owner_is(&self,kind:&str)->bool {super::metaclass_conforms(self.owner_kind,kind)}}", "#[allow(unused_parens)]\npub(super) fn names(c:&Inputs<'_>)->Option<Vec<&'static str>> {let mut roles=Vec::new();"]
 lines.extend(StateTranslator(data,b).effects(b['dispatch']['addDefaultGeneralType']))
 lines+=['let mut result=Vec::new();for role in roles {let name=match role {']
 for role,name in sorted(b['names'].items()):
  if name is not None:lines.append(json.dumps(role)+'=>'+json.dumps(name)+',')
 lines+=['_=>return None,};if !result.contains(&name) {result.push(name);}}Some(result)}','']
 return '\n'.join(lines)
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args();code=generate(json.loads(INPUT.read_text(encoding='utf-8')))
 if a.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale generated State contributions')
 else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
 print('24 resolved programs; 1632 contribution controls; 320 selected Ecore invocation controls')
if __name__=='__main__':main()
