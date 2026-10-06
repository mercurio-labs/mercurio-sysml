"""Bounded generation of pinned multiplicity admission and storage policy."""
import argparse, hashlib, json
from pathlib import Path
from generate_expression_redefinitions import canonical, require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/multiplicity-construction-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/multiplicity_construction_generated.rs'
TREE='012989c3194933d58a3622e61745bc824cdc27a29d0498f56a6fb4c124446475'
def generate(data):
 require(data['schema']=='dev.mercurio.multiplicity-construction-controls.v1','Changed schema')
 require(hashlib.sha256(json.dumps(canonical(data['methods']),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Resolved algorithms changed; review required')
 require(data['excluded_bindings']=={'SuccessionAsUsage':'org.omg.sysml.adapter.NamespaceAdapter#addAdditionalMembers','TransitionUsage':'org.omg.sysml.adapter.TransitionUsageAdapter#addAdditionalMembers'},'Changed excluded dispatch inventory')
 bindings=data['bindings'];require(len(bindings)==45 and hashlib.sha256(json.dumps(bindings,sort_keys=True,separators=(',',':')).encode()).hexdigest()=='3f36aa043cbf2316f97662de26cbe42d7b4e7077067f07fa949da74656e0c816','Changed dispatch inventory')
 modes={k:v.split('.')[-1].split('#')[0] in ('ItemUsageAdapter','PortUsageAdapter','AttributeUsageAdapter') for k,v in bindings.items()}
 require(all(v.split('.')[-1].split('#')[0] in ('UsageAdapter','ConnectionUsageAdapter','ItemUsageAdapter','PortUsageAdapter','AttributeUsageAdapter') for v in bindings.values()),'Unassessed dispatch')
 matrix={(k,e,o,s,x) for k in bindings for e in (False,True) for o in ('detached','Package','Class') for s in ('none','detached','owned','chain_detached','chain_owned') for x in ('none','owned_range','alias')}
 controls=data['controls'];require(len(controls)==4050 and {(c['kind'],c['is_end'],c['owner'],c['subset'],c['existing']) for c in controls}==matrix,'Incomplete controls')
 for c in controls:
  admitted=c['is_end'] or modes[c['kind']] and c['owner']=='Class' and c['subset'] not in ('owned','chain_owned')
  before=int(c['existing']=='owned_range');after=before or int(admitted)
  require((c['admitted'],c['before'],c['after'],c['replay'])==(admitted,before,after,after),'Reference disagreement')
 statements=data['methods']['org.omg.sysml.util.TypeUtil#addMultiplicityTo']['statements'][0]['then']['statements']
 kinds=[s['initializer']['type'].split('.')[-1] for s in statements[:2]]
 return '\n'.join(['// Generated bounded policy; canonical graph projection and transactions are handwritten.',f'pub(super) const MULTIPLICITY: &str = {json.dumps(kinds[0])};',f'pub(super) const MEMBERSHIP: &str = {json.dumps(kinds[1])};','pub(super) fn uses_default_admission(kind: &str) -> Option<bool> { match kind {',*[f'    {json.dumps(k)} => Some({str(v).lower()}),' for k,v in sorted(modes.items())],'    _ => None,','} }','pub(super) fn admitted(default: bool, end: bool, owning_type: bool, subset_basic_owned: bool) -> bool { end || default && owning_type && !subset_basic_owned }',''])
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');args=p.parse_args();code=generate(json.loads(INPUT.read_text()))
 if args.check:require(OUTPUT.read_text()==code,'Stale native policy')
 else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
 print('45 bindings / 4050 controls: resolved multiplicity policy checked')
if __name__=='__main__':main()
