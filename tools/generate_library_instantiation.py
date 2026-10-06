"""Bounded resolved operator/trigger library-selector recipes; graph algorithms remain Rust."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/library-instantiation-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/library_instantiation_generated.rs'
DEFINITION_SHA='fa6ac52ab37c27ce65df5ff1084c92d66b03d57c4fd6df479d9d0d90a5a0be51'
MAPPING_SHA='7ecf568d0ae5feef5ab68bb7c096d5c5ea87a61bce1071d333bae773e3397e8f'
def sha(v):return hashlib.sha256(json.dumps(v,sort_keys=True,separators=(',',':')).encode()).hexdigest()
def generate(d):
 require(d['schema']=='dev.mercurio.library-instantiation-controls.v1','Changed library selector schema')
 require(sha(canonical(d['definitions']))==DEFINITION_SHA,'Changed resolved library selection programs')
 require(sha({k:d[k] for k in ['defaults','trigger_roles','operator_packages']})==MAPPING_SHA,'Changed resolved library mappings/defaults')
 require(len(d['definitions'])==25 and len(d['controls'])==1236,'Incomplete library selector inventory')
 keys=[]
 for c in d['controls']:
  require(c['kind'] in d['defaults'],'Unknown control binding')
  keys.append((c['kind'],c['variant'],tuple(c['values'])))
 require(len(keys)==len(set(keys)),'Duplicate library observation')
 import itertools
 expected=set()
 for kind,default in d['defaults'].items():
  for variant in (['default','null','when','at','after'] if kind=='TriggerInvocationExpression' else ['default','null','+','a::b','a b']):
   selector=default if variant=='default' else None if variant=='null' else variant
   count=0 if selector is None else 1 if kind=='TriggerInvocationExpression' else 3
   expected.update((kind,variant,values) for values in itertools.product(['absent','Package','Class','Function'],repeat=count))
 require(set(keys)==expected,'Incomplete library observation matrix')
 lines=['// Generated from resolved pinned utility/delegate programs and constructor defaults.',
  'pub(super) const DEFINITION_SHA: &str = '+json.dumps(DEFINITION_SHA)+';',
  "pub(super) fn operator_default(kind: &str)->Option<Option<&'static str>> { match kind {"]
 for k,v in sorted(d['defaults'].items()):
  if k!='TriggerInvocationExpression':lines.append(json.dumps(k)+' => Some('+('None' if v is None else 'Some('+json.dumps(v)+')')+'),')
 lines+=['_ => None, }}','pub(super) fn operator_names(operator: &str)->Vec<String> {',
  'vec!['+', '.join('format!("'+p+'::\'{}\'",operator)' for p in d['operator_packages'])+']','}',
  "pub(super) fn trigger_name(kind: &str)->Option<&'static str> { match kind {"]
 for k,v in sorted(d['trigger_roles'].items()):lines.append(json.dumps(k)+' => Some('+json.dumps(v)+'),')
 lines+=['_ => None, }}']
 return '\n'.join(lines)+'\n'
def main():
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args();s=generate(json.loads(INPUT.read_text(encoding='utf-8')))
 if a.check:require(OUTPUT.read_text(encoding='utf-8')==s,'Stale library selector recipes')
 else:OUTPUT.write_text(s,encoding='utf-8',newline='\n')
 print('Six resolved library selectors current; default transformation is separate')
if __name__=='__main__':main()
