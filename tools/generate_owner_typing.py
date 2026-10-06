"""Guard resolved receiver-typing producer footprint; generate reviewed binding admission.
The Rust absence proof is explicitly handwritten, not a general method translator.
"""
import argparse,json,hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/owner-typing-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/owner_typing_generated.rs'
def digest(value):return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()
def generate(data):
 if data['schema']!='dev.mercurio.owner-typing-controls.v1':raise ValueError('Changed owner typing schema')
 if digest(data['methods'])!='91e2fe1aa9a241cc3542d0b997ae8c44090d86a5d45061e4388566a7e541207d':raise ValueError('Resolved receiver typing producers changed; review proof')
 if digest(data['bindings'])!='e57b3d754f3a12a2748d9a89b1a2c2f9ffeb4e5f0b75bb838360edf9aad06075':raise ValueError('Changed receiver typing dispatch; review proof')
 kinds={'Feature','ReferenceUsage','ActionUsage','StateUsage'}
 if set(data['bindings'])!=kinds:raise ValueError('Changed owner typing binding inventory')
 controls=data['controls'];matrix={(k,c,m,b,r) for k in kinds for c in ('detached','Namespace','Package','Class','ActionDefinition','StateDefinition') for m in range(8) for b in (False,True) for r in ('FeatureMembership','OwningMembership')}
 if len(controls)!=768 or {(c['kind'],c['context'],c['typing_mask'],c['composite'],c['membership']) for c in controls}!=matrix:raise ValueError('Incomplete fresh receiver typing controls')
 for c in controls:
  if c['implicit_before'] or c['implicit_after'] or c['is_implied_included']:raise ValueError('Fresh receiver implicit typing absence no longer holds')
  if c['explicit']!=['type.'+str(i) for i in range(3) if c['typing_mask']&(1<<i)]:raise ValueError('Changed explicit typing projection')
 return '// Generated reviewed dispatch admission; canonical absence proof is handwritten.\n'+'pub(super) fn supports(kind:&str)->bool {matches!(kind,'+'|'.join(json.dumps(k) for k in sorted(kinds))+')}\n'
def nullable(data):
 if data['absent_portion_values']!={'ActionUsage':True,'StateUsage':True,'TransitionUsage':True}:raise ValueError('Changed absent portion values')
 if digest(data['portion_initialization'])!='00de3e6f60027c1ac580473eab1df1eaf9e5c95cc2d3354601700df0c1ad5930':raise ValueError('Changed resolved nullable portion initialization')
 if not all(c['portion_default_is_null'] for c in data['controls']):raise ValueError('Changed fresh receiver portion initialization')
 return '// Generated from resolved Occurrence initialization and fresh exact owner controls.\n'+'pub(super) const NULL_DEFAULTS: &[(&str,&str)] = &[("ActionUsage","portion_kind"),("StateUsage","portion_kind"),("TransitionUsage","portion_kind")];\n'
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
 data=json.loads(INPUT.read_text(encoding='utf-8'));text=generate(data)
 nullpath=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/ecore_owner_nullable_defaults_generated.rs';nulltext=nullable(data)
 if args.check:
  if nullpath.read_text(encoding='utf-8')!=nulltext:raise ValueError('Stale owner nullable defaults')
  if OUTPUT.read_text(encoding='utf-8')!=text:raise ValueError('Stale receiver typing admission')
 else:
  OUTPUT.write_text(text,encoding='utf-8',newline='\n');nullpath.write_text(nulltext,encoding='utf-8',newline='\n')
 print('Resolved receiver typing footprint current: 18 method bodies, 4 bindings, 768 fresh controls')
if __name__=='__main__':main()
