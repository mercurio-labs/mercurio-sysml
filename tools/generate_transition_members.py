"""Generate guarded Transition member constructor identities from resolved Pilot definitions."""
import argparse,json
from pathlib import Path
from generate_transition_source_queries import generate as validate_source,ECORE,INPUT
ROOT=Path(__file__).resolve().parents[1]
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/transition_members_generated.rs'
def walk(node):
    yield node
    for child in node.get('children',[]):yield from walk(child)
def generate(data,ecore):
    validate_source(data,ecore)
    method=data['methods']['org.omg.sysml.adapter.TransitionUsageAdapter#computeTransitionLinkConnectors']
    creators=[n for n in walk(method) if n.get('kind')=='METHOD_INVOCATION' and n.get('symbol','').endswith('#createReferenceUsage')]
    if len(creators)!=1: raise ValueError('Changed Transition link factory')
    link=creators[0]['type'].split('.')[-1]
    membership=next(c for c in ecore['classes'] if c['name']=='FeatureMembership')['name']
    return '// Generated guarded resolved Transition constructor identities; traversal is handwritten.\n'+f'pub(super) const OWNER: &str = "TransitionUsage";\npub(super) const LINK: &str = "{link}";\npub(super) const MEMBERSHIP: &str = "{membership}";\n'
def main():
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args()
    code=generate(json.loads(INPUT.read_text(encoding='utf-8')),json.loads(ECORE.read_text(encoding='utf-8')))
    if a.check:
        if OUTPUT.read_text(encoding='utf-8')!=code: raise ValueError('Stale Transition constructor identities')
    else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
    print('Resolved Transition constructor identities checked; lifecycle and resource qualification separate')
if __name__=='__main__':main()
