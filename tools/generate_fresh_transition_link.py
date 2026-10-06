"""Guard resolved fresh fixed ReferenceUsage lifecycle; native navigation is handwritten."""
import argparse,hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/fresh-transition-link-pilot-controls.json'
OUTPUT=ROOT/'crates/mercurio-sysml/src/language_frontend/lowering/emit/fresh_transition_link_generated.rs'
TREE='0f232ef335466bae253d621b92d3859d2e7eafa0571c0a415c1dadef1ad0ca88'
def generate(data):
    require(data['schema']=='dev.mercurio.fresh-transition-link-controls.v1','Changed lifecycle schema')
    require(hashlib.sha256(json.dumps(canonical(data['methods']),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Changed resolved lifecycle; review required')
    require(len(data['methods'])==8,'Changed lifecycle inventory')
    body=data['methods']['org.omg.sysml.lang.sysml.impl.UsageImpl#isVariable']
    call=body['children'][0]['children'][0]
    require(call['kind']=='METHOD_INVOCATION' and call['symbol']=='isMayTimeVary()' and call['type']=='boolean','Changed resolved variability getter')
    receiver=data['methods']['org.omg.sysml.adapter.ReferenceUsageAdapter#addDefaultGeneralType']['children'][0]['type']
    require(receiver=='org.omg.sysml.lang.sysml.ReferenceUsage','Changed resolved lifecycle receiver')
    kind=receiver.split('.')[-1]
    # Complete resolved method bodies remain guarded; these two services execute
    # the admitted fixed empty-relationship branch, not arbitrary Java methods.
    return '// Generated guarded resolved fresh fixed link lifecycle identity.\n'+f'pub(super) const PROGRAM_SHA: &str = "{TREE}";\npub(super) const KIND: &str = "{kind}";\n'
def main():
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args();code=generate(json.loads(INPUT.read_text(encoding='utf-8')))
    if a.check:require(OUTPUT.read_text(encoding='utf-8')==code,'Stale fresh link lifecycle guard')
    else:OUTPUT.write_text(code,encoding='utf-8',newline='\n')
    print('Eight resolved lifecycle/getter methods guarded; fixed fresh-link admission only')
if __name__=='__main__':main()
