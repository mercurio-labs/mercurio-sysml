"""Guard full resolved succession/connector child programs; generate typed identities."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/transition-owner-recursive-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/transition_child_stages_generated.rs'
TREE='46ac418479d5f5018f7d3faa10b3e62a51a75612f763780857a677c4292b16a2'
def walk(node):
    yield node
    for child in node.get('children',[]):yield from walk(child)
def generate(data):
    require(data['schema']=='dev.mercurio.transition-owner-recursive-controls.v1','Changed child schema')
    require(len(data['methods'])==4,'Changed complete child program inventory')
    require(hashlib.sha256(json.dumps(canonical(data['methods']),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Changed resolved child programs; review required')
    body=data['methods']['org.omg.sysml.adapter.SuccessionAsUsageAdapter#addContextFeaturingType']
    receiver=body['children'][0]['type'].rsplit('.',1)[-1]
    guards=[n for n in walk(body) if n['kind']=='INSTANCE_OF']
    require(len(guards)==1,'Changed context owner guard')
    types=[n['type'] for n in walk(guards[0]) if n['kind']=='IDENTIFIER' and n.get('symbol')==n.get('type')]
    require(len(types)==1,'Unresolved context owner type')
    owner=types[0].rsplit('.',1)[-1]
    return f'// Generated typed child identities; context/navigation execution is handwritten.\npub(super) const PROGRAM_SHA:&str="{TREE}";\npub(super) const SUCCESSION:&str="{receiver}";\npub(super) const OWNER:&str="{owner}";\n'
def main():
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args();code=generate(json.loads(INPUT.read_text(encoding='utf-8')))
    if a.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale generated child identities')
    else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
    print('Four full resolved child bodies guarded; typed receiver/context identities generated')
if __name__=='__main__':main()
