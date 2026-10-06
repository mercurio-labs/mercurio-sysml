"""Validate reference-only expression context controls; this does not claim native support."""
import hashlib,json
from pathlib import Path
from generate_expression_redefinitions import canonical,require
from generate_instantiation_contributions import DEFINITION_SHA
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/expression-context-contribution-pilot-controls.json'
def validate(d):
 require(d['provenance']['pilot_commit']=='692170b71867353b8f90341e61556f49a5beb0e5','Wrong pin')
 require(d['provenance']['jar_sha256']=='4512852638fc799505fe676955e8c78fedef1f3ce44ec90b57b34c05c31e6945','Wrong jar')
 require(hashlib.sha256(json.dumps(canonical(d['definitions']),sort_keys=True,separators=(',',':')).encode()).hexdigest()==DEFINITION_SHA,'Contribution program changed')
 kinds=json.loads((ROOT/'docs/conformance/2026-08-support/expression-parameter-pilot-controls.json').read_text())['bindings']
 require(set(d['bindings'])==set(kinds),'Expression binding inventory changed')
 expected={(k,c,v,b) for k in kinds for c in ['detached','package','feature','parameter','result','value'] for v in ['null','in','out','inout'] for b in [False,True]}
 keys=[(c['kind'],c['context'],c['direction'],c['composite']) for c in d['controls']]
 require(len(keys)==960 and len(set(keys))==960 and set(keys)==expected,'Incomplete or duplicate context matrix')
 require(all(not c['is_end'] and not c['is_implied_included'] for c in d['controls']),'Changed receiver preconditions')
 require(all(c['status']=='ready' for c in d['controls']),'Unassessed reference outcome')
if __name__=='__main__':
 validate(json.loads(INPUT.read_text()));print('Reference context matrix current; native consumption/qualification not assessed')
