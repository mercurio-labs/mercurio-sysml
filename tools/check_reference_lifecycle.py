"""Assess the bounded fresh transition-link cache; never close a whole family."""
import hashlib,json
from pathlib import Path
from generate_reference_link import canonical,TREE,require
from export_pilot_feature_defaults import OUTPUT as DEFAULTS,validate as validate_defaults
ROOT=Path(__file__).resolve().parents[1]
INPUT=ROOT/'docs/conformance/2026-08-support/reference-lifecycle-pilot-controls.json'
def validate(data):
    require(data['schema']=='dev.mercurio.reference-lifecycle-controls.v1','Changed lifecycle schema')
    policy=json.loads(DEFAULTS.read_text(encoding='utf-8'));validate_defaults(policy)
    require(data['provenance']['pilot_commit']=='692170b71867353b8f90341e61556f49a5beb0e5','Changed Pilot pin')
    require(data['provenance']['jar_sha256']==policy['provenance']['runtime_sha256'],'Changed lifecycle runtime')
    tree={'methods':data['methods'],'constants':data['constants']}
    require(hashlib.sha256(json.dumps(canonical(tree),sort_keys=True,separators=(',',':')).encode()).hexdigest()==TREE,'Changed resolved Reference programs')
    rows=data['controls'];expected={(a,b,p,e) for a in range(8) for b in range(8) for p in (False,True) for e in (False,True)}
    require(len(rows)==256 and {(r['owned_mask'],r['role_mask'],r['preceding_parameter'],r['explicit_role']) for r in rows}==expected,'Incomplete lifecycle matrix')
    names=policy['usage_bindings']['ReferenceUsage']['defaults']
    for row in rows:
        require(hashlib.sha256(row['source'].encode()).hexdigest()=='433dfb24920f294daaeb2b30a3483db1dc09980abb9adc797d70564108814b96','Changed lifecycle fixture')
        mask=row['owned_mask'];key='object' if mask & 2 else 'occurrence' if mask & 1 else 'dataValue' if mask & 4 else 'base'
        require(row['default_supertype']==names[key],'Default observation disagreement')
        types=[prefix+str(bit) for prefix,mask in [('own',row['owned_mask']),('role',row['role_mask'])] for bit in range(3) if mask & (1<<bit)]
        require(row['types']==types and row['definitions']==types,'Inherited type/projection disagreement')
        require(row['selected'] and not row['candidate_complete_after_query'],'Changed fresh selected-role lifecycle')
if __name__=='__main__':
    validate(json.loads(INPUT.read_text(encoding='utf-8')));print('Fresh transition link: 256 default/type/projection observations; whole lifecycle remains open')
