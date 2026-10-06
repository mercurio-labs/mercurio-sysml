"""Translate the reviewed bounded Occurrence default predicate/effect subset.
Canonical Ecore ownership/typing and full Feature lifecycle admission are Rust services.
"""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/occurrence-contribution-pilot-controls.json'
NULL_OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/ecore_nullable_defaults_generated.rs'
ECORE=ROOT/'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/ecore-effective.extract.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/occurrence_contributions_generated.rs'
TREE='31bed30a3692c1b8b1f69c9158ecf15287b0bc579e78566461928d770af9b4a4'
BINDINGS='cb379def947fda9d92d9d7c1cda8cb463e99729daa41943e253d94459c079e73'
PREFIX='org.omg.sysml.adapter.'
CONTEXTS=('detached','Package','Class','Structure','OccurrenceDefinition','OccurrenceUsage','ItemDefinition','ItemUsage','ViewDefinition','ViewUsage','Feature_class','Feature_structure')
def sha(x):return hashlib.sha256(json.dumps(x,sort_keys=True,separators=(',',':')).encode()).hexdigest()
def condition(value):
 while value.startswith('(') and value.endswith(')'):
  depth=0;encloses=True
  for i,char in enumerate(value):
   depth += (char=='(')-(char==')')
   if depth==0 and i<len(value)-1:encloses=False;break
  if not encloses:break
  value=value[1:-1]
 return value
class Translator:
 def __init__(self,data,binding):self.methods=data['methods'];self.binding=binding
 def expression(self,n,locals=None,stack=()):
  env={} if locals is None else locals;k=n['kind']
  if k=='PARENTHESIZED':return self.expression(n['expression'],env,stack)
  if k=='TYPE_CAST':
   require(n['cast_type']=='org.omg.sysml.lang.sysml.Feature' and self.expression(n['expression'],env,stack)=='@owner','Unassessed owner cast');return '@owner'
  if k=='IDENTIFIER':require(n['name'] in env,'Unbound identifier');return env[n['name']]
  if k=='INSTANCE_OF':
   require(self.expression(n['expression'],env,stack)=='@owner','Unassessed instanceof target')
   return 'c.owner_is('+json.dumps(n['test_type'].split('.')[-1])+')'
  if k in ('CONDITIONAL_AND','CONDITIONAL_OR','EQUAL_TO'):
   return '('+self.expression(n['left'],env,stack)+{'CONDITIONAL_AND':' && ','CONDITIONAL_OR':' || ','EQUAL_TO':' == '}[k]+self.expression(n['right'],env,stack)+')'
  if k=='LOGICAL_COMPLEMENT':return '(!'+self.expression(n['expression'],env,stack)+')'
  if k=='MEMBER_SELECT' and n['receiver'].get('name')=='PortionKind':return json.dumps(n['name'].lower())
  if k in ('STRING_LITERAL','BOOLEAN_LITERAL'):return json.dumps(n['value'])
  require(k=='METHOD_INVOCATION','Unassessed predicate node '+k)
  name=n['symbol'].split('#')[-1];args=n['arguments']
  if name=='getTarget':return '@target'
  if name=='getOwningType':return '@owner'
  if name=='isComposite':return 'c.composite'
  if name=='getPortionKind':return 'c.portion'
  if name=='hasDataType':require(not args,'Unassessed datatype arguments');return 'c.data'
  if name in ('hasStructureType','hasClassType'):
   if args:
    require(len(args)==1 and self.expression(args[0],env,stack)=='@owner','Unassessed owner typing')
    return 'c.owner_structure' if name=='hasStructureType' else 'c.owner_class'
   return 'c.structure' if name=='hasStructureType' else 'c.class'
  require(name in ('isSubobject','isSuboccurrence','isSubitem','isSubview'),'Unassessed predicate '+name)
  key=n['symbol'] if n.get('receiver',{}).get('name')=='super' else self.binding['dispatch'].get(name,PREFIX+('ItemUsageAdapter' if name=='isSubitem' else 'ViewUsageAdapter')+'#'+name)
  require(key not in stack,'Cyclic predicate translation')
  return self.return_expression(self.methods[key],(*stack,key))
 def return_expression(self,body,stack=()):
  env={}
  require(body['kind']=='BLOCK','Unassessed method body')
  for statement in body['statements']:
   if statement['kind']=='VARIABLE':env[statement['name']]=self.expression(statement['initializer'],env,stack)
   elif statement['kind']=='RETURN':return self.expression(statement['expression'],env,stack)
   else:raise ValueError('Unassessed return statement')
  raise ValueError('Missing return')
 def selector(self):
  body=self.methods[self.binding['dispatch']['getDefaultSupertype']];n=body['statements'][0]['expression']
  def role(x):
   if x['kind']=='CONDITIONAL_EXPRESSION':return 'if '+condition(self.expression(x['condition']))+' { '+role(x['true'])+' } else { '+role(x['false'])+' }'
   require(x['kind']=='METHOD_INVOCATION' and x['symbol'].endswith('#getDefaultSupertype') and len(x['arguments'])==1,'Unassessed default selector')
   return self.expression(x['arguments'][0])
  return role(n)
 def effects(self,key,indent='    ',env=None):
  env={} if env is None else dict(env)
  # Shared Feature stage is separately guarded by native code. Usage variation
  # is explicitly excluded from this bounded domain; no signature-only support.
  if key==PREFIX+'FeatureAdapter#addDefaultGeneralType':return [indent+'roles.push('+self.selector()+');']
  lines=[]
  def statement(n,indent,env):
   kind=n['kind']
   if kind=='BLOCK':
    for s in n['statements']:statement(s,indent,env)
   elif kind=='VARIABLE':env[n['name']]=self.expression(n['initializer'],env)
   elif kind=='IF':
    lines.append(indent+'if '+condition(self.expression(n['condition'],env))+' {');statement(n['then'],indent+'    ',dict(env));lines.append(indent+'}')
    if 'else' in n:lines[-1]+=' else {';statement(n['else'],indent+'    ',dict(env));lines.append(indent+'}')
   elif kind=='EXPRESSION_STATEMENT':
    x=n['expression'];require(x['kind']=='METHOD_INVOCATION','Unassessed effect')
    name=x['symbol'].split('#')[-1]
    if name=='addVariationTyping':require(key==PREFIX+'UsageAdapter#addDefaultGeneralType','Unassessed variation call');return
    require(name=='addDefaultGeneralType','Unassessed contribution effect '+name)
    if x.get('receiver',{}).get('name')=='super':lines.extend(self.effects(x['symbol'],indent,env))
    else:
     require(len(x['arguments'])==1,'Unassessed contribution arguments');a=x['arguments'][0]
     if a['kind']=='CONDITIONAL_EXPRESSION':value='if '+condition(self.expression(a['condition'],env))+' { '+self.expression(a['true'],env)+' } else { '+self.expression(a['false'],env)+' }'
     else:value=self.expression(a,env)
     lines.append(indent+'roles.push('+value+');')
   else:raise ValueError('Unassessed effect statement '+kind)
  statement(self.methods[key],indent,env);return lines

def expected(data,c):
 b=data['bindings'][c['kind']];ctx=c['context'];comp=c['composite'];mask=c['typing_mask'];item=b['dispatch']['isSubobject'].endswith('ItemUsageAdapter#isSubobject')
 owner_class=ctx in ('Class','Structure','OccurrenceDefinition','ItemDefinition','ViewDefinition','Feature_class','Feature_structure')
 owner_structure=ctx in ('Structure','ItemDefinition','ViewDefinition','Feature_structure')
 subitem=comp and ctx in ('ItemDefinition','ItemUsage','ViewDefinition','ViewUsage')
 subobject=comp and owner_structure and not(item and subitem)
 suboccurrence=comp and (owner_class or ctx in ('OccurrenceUsage','ItemUsage','ViewUsage')) and not(item and subitem)
 selector=b['dispatch']['getDefaultSupertype']
 role='subview' if selector.endswith('ViewUsageAdapter#getDefaultSupertype') and ctx in ('ViewDefinition','ViewUsage') else 'subitem' if selector.endswith('ItemUsageAdapter#getDefaultSupertype') and subitem else 'base'
 roles=[role]
 if mask&1:roles.append('dataValue')
 if mask&4:roles.append('subobject' if subobject else 'object')
 elif suboccurrence:roles.append('suboccurrence')
 if c['portion']!='none':roles.append(c['portion'])
 if b['dispatch']['addDefaultGeneralType'].endswith('ViewUsageAdapter#addDefaultGeneralType') and subitem:roles.append('subpart')
 names=[]
 for role in roles:
  name=b['names'][role];require(name is not None,'Missing required role definition')
  if name not in names:names.append(name)
 return [{'kind':'Subsetting','target':n} for n in names]

def generate(data):
 require(data['schema']=='dev.mercurio.occurrence-contribution-controls.v1','Changed schema')
 require(sha(canonical(data['methods']))==TREE,'Resolved contribution algorithms changed; review required')
 require(sha(data['bindings'])==BINDINGS,'Changed binding/default definitions')
 require(sha(data['portion_initialization'])=='57f702557878b7d4d8801066b5c3275cd781983fb4553e30de966b551727d666','Changed portion initialization')
 require(data['absent_portion_values']==dict.fromkeys(data['bindings']),'Fresh portion observation disagreement')
 controls=data['controls'];matrix={(k,c,b,m,p) for k in data['bindings'] for c in CONTEXTS for b in (False,True) for m in range(8) for p in ('none','snapshot','timeslice')}
 require(len(controls)==2304 and {(c['kind'],c['context'],c['composite'],c['typing_mask'],c['portion']) for c in controls}==matrix,'Incomplete contribution matrix')
 for c in controls:require(c['generals']==expected(data,c),'Pilot contribution disagreement')
 lines=['// Generated by bounded resolved predicate/effect translation, with imported default maps.','// Canonical inputs and Feature/Usage lifecycle admission are handwritten dependencies.',"pub(super) struct Inputs<'a> {pub composite:bool,pub structure:bool,pub data:bool,pub owner_kind:&'a str,pub owner_class:bool,pub owner_structure:bool,pub portion:&'a str}","impl Inputs<'_> {fn owner_is(&self,kind:&str)->bool {super::metaclass_conforms(self.owner_kind,kind)}}",'pub(super) fn supports(kind:&str)->bool {matches!(kind,'+' | '.join(map(json.dumps,sorted(data['bindings'])))+')}',"pub(super) fn names(kind:&str,c:&Inputs<'_>)->Option<Vec<&'static str>> {",'let mut roles=Vec::new();match kind {']
 for kind,b in sorted(data['bindings'].items()):
  lines.append(json.dumps(kind)+' => {');lines.extend(Translator(data,b).effects(b['dispatch']['addDefaultGeneralType']));lines.append('},')
 lines+=['_ => return None,','}','let mut names=Vec::new();for role in roles {let name=match (kind,role) {']
 for kind,b in sorted(data['bindings'].items()):
  for role,value in sorted(b['names'].items()):
   if value is not None:lines.append('('+json.dumps(kind)+','+json.dumps(role)+') => '+json.dumps(value)+',')
 lines+=['_ => return None,};if !names.contains(&name) {names.push(name);}}Some(names)','}','']
 return '\n'.join(lines)
def nullable_defaults(data,ecore):
 generate(data)
 feature=next(f for f in ecore['features'] if f['id'].endswith('#//OccurrenceUsage/portionKind'))
 require((feature['lower_bound'],feature['upper_bound'],feature['derived'],feature['volatile'],feature['default_literal'],feature['default_value'])==(0,1,False,False,None,'timeslice'),'Changed Ecore nullable enum contract')
 return '\n'.join(['// Generated resolved nullable initialization; Ecore enum default remains separately preserved.','// Only exact concrete bindings with independent fresh-factory observations are admitted.','pub(super) const NULL_DEFAULTS: &[(&str,&str)] = &[',*[f'    ({json.dumps(k)},"portion_kind"),' for k in sorted(data['bindings'])],'];',''])
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');args=p.parse_args();data=json.loads(INPUT.read_text(encoding='utf-8'));code=generate(data);nulls=nullable_defaults(data,json.loads(ECORE.read_text(encoding='utf-8')))
 if args.check:
  require(OUTPUT.read_text(encoding='utf-8')==code,'Stale Occurrence policy');require(NULL_OUTPUT.read_text(encoding='utf-8')==nulls,'Stale nullable initialization')
 else:
  OUTPUT.write_text(code,encoding='utf-8',newline='\n');NULL_OUTPUT.write_text(nulls,encoding='utf-8',newline='\n')
 print('15 resolved methods / 4 bindings / 2304 controls: bounded predicate and effect translation checked')
if __name__=='__main__':main()
