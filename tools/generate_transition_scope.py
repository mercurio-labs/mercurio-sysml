"""Guard resolved Xtend source-origin rule; native navigation is handwritten.
Target/feature-chaining branches are retained as reference, not implemented here.
"""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/transition-scope.ast.extract.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/transition_scope_generated.rs'
def digest(v):return hashlib.sha256(json.dumps(v,sort_keys=True,separators=(',',':')).encode()).hexdigest()
def generate(d):
 if d['parser']!='org.eclipse.xtend.core.XtendStandaloneSetup' or d['coverage']['unresolved_references']!=0:raise ValueError('Unresolved transition scope AST')
 if {m['name'] for m in d['methods']}!={'getScope','featureRelationship'}:raise ValueError('Changed scope method inventory')
 if digest({'methods':d['methods'],'symbols':d['symbols'],'types':d['types']})!='b0851cbf59e0e0dd1b1c874ce25f2efb42bbd85a62108c492db5df71a4e71f04':raise ValueError('Resolved transition scope rule changed; review native navigation')
 names=[]
 for name in ('Membership','FeatureMembership','TransitionUsage'):
  symbol=d['symbols']['org.omg.sysml.lang.sysml.'+name]
  if symbol['kind']!='JvmGenericType':raise ValueError('Unresolved origin classifier')
  names.append(symbol['simple_name'])
 return '// Generated resolved classifier identities for the reviewed source-origin rule.\n'+''.join('pub(super) const '+key+':&str='+json.dumps(value)+';\n' for key,value in zip(('MEMBERSHIP','EXCLUDED_MEMBERSHIP','TRANSITION'),names))
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');a=p.parse_args();text=generate(json.loads(INPUT.read_text(encoding='utf-8')))
 if a.check:
  if OUTPUT.read_text(encoding='utf-8')!=text:raise ValueError('Stale transition scope classifier bindings')
 else:OUTPUT.write_text(text,encoding='utf-8',newline='\n')
 print('Resolved Xtend scope classifier bindings current; only source origin consumed')
if __name__=='__main__':main()
