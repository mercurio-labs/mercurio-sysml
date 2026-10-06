"""Guard resolved bounded multiplicity/default programs; graph execution is Rust."""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/result-multiplicity-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/result_multiplicity_generated.rs'
EXPECTED={'org.omg.sysml.adapter.FeatureAdapter#addBoundValueSubsetting/0': '08bbdf46073ced89ec01a0fa819e5b4200d79c78af2195c3eb3948baece9d544', 'org.omg.sysml.adapter.FeatureAdapter#addDefaultGeneralType/0': '3977ff5149a6ee5575a36b0d336cae99639d8eb10da277ee2643aafcc4ff77f1', 'org.omg.sysml.adapter.FeatureAdapter#getDefaultSupertype/0': '4476bc8db57b334b7cef55bfbce6a85d4cc88bf2b053ceed701b30d148ef99ef', 'org.omg.sysml.delegate.setting.Type_multiplicity_SettingDelegate#basicGet/1': '0a58a3c4764619bfb53da6c47e1f42e3ad92989869f052aea4ed9aa25fe81553', 'org.omg.sysml.delegate.setting.Type_multiplicity_SettingDelegate#getMultiplicityOf/2': '073af4f06b53a1a9d95495cb977e6ea487e9888459f5bb8d44dc8d492d96ace2'}
def generate(data):
 if data.get('schema')!='dev.mercurio.result-multiplicity-controls.v1':raise ValueError('Wrong result multiplicity schema')
 actual={k:hashlib.sha256(json.dumps(v,sort_keys=True,separators=(',',':')).encode()).hexdigest() for k,v in data['methods'].items()}
 if actual!=EXPECTED:raise ValueError('Unassessed result/multiplicity programs')
 def walk(n):
  yield n
  for c in n.get('children',[]):yield from walk(c)
 nodes=list(walk(data['methods']['org.omg.sysml.delegate.setting.Type_multiplicity_SettingDelegate#getMultiplicityOf/2']))
 types={n.get('symbol') for n in nodes if n.get('source')=='Multiplicity' and n.get('kind')=='IDENTIFIER'}
 if types!={'org.omg.sysml.lang.sysml.Multiplicity'}:raise ValueError('Changed multiplicity target')
 target=next(iter(types)).rsplit('.',1)[1]
 sha=hashlib.sha256(json.dumps(data['methods'],sort_keys=True,separators=(',',':')).encode()).hexdigest()
 return '// Generated bounded owned-multiplicity projection guard; graph execution is Rust.\n'+f'pub(super) const PROGRAM_SHA:&str="{sha}";\npub(super) const TARGET:&str="{target}";\n'
def main():
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args();s=generate(json.loads(INPUT.read_text()))
 if a.check:
  if OUTPUT.read_text()!=s:raise ValueError('Stale generated result multiplicity guard')
 else:OUTPUT.write_text(s,encoding='utf-8',newline='\n')
if __name__=='__main__':main()
