"""Select pinned resources from parsed imports and qualified cross-reference syntax.

This is conservative package-prefix dependency selection, not Xtext scoping or
semantic closure. Local member references and implicit dependencies remain open.
"""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]

def select(inventory,roots):
    resources={};packages={}
    for row in inventory['resources']:
        source=row['release_path']
        if source in resources:raise ValueError('Duplicate resource identity')
        resources[source]=row
        for package in row['packages']:
            key=tuple(package)
            if not key or key in packages:raise ValueError('Empty or ambiguous package path')
            packages[key]=source
    def provider(owner,target):
        for depth in range(len(owner),-1,-1):
            qualified=tuple(owner[:depth])+tuple(target)
            for length in range(len(qualified),depth,-1):
                if qualified[:length] in packages:return packages[qualified[:length]]
        return None
    pending=[]
    for root in roots:
        if tuple(root) not in packages:raise ValueError('Missing root package')
        pending.append(packages[tuple(root)])
    included=set();edges=[];unresolved=[];local=[]
    while pending:
        source=pending.pop()
        if source in included:continue
        included.add(source);row=resources[source]
        for origin,items in [('import',row['imports']),('cross_reference',row['references'])]:
            for declaration in items:
                target=declaration['target_segments'];owner=declaration['owner_package']
                if not target:
                    if origin=='import' and declaration.get('implicit_filter_target'):continue
                    raise ValueError('Missing concrete reference target')
                if origin=='cross_reference' and len(target)<2:
                    local.append({'source':source,**declaration});continue
                endpoint=provider(owner,target)
                edge={'source':source,'origin':origin,'target_segments':target,'owner_package':owner,'provider':endpoint}
                for field in ['kind','feature','target_type','offset']:
                    if field in declaration:edge[field]=declaration[field]
                edges.append(edge)
                if endpoint is None:unresolved.append(edge)
                else:pending.append(endpoint)
    return {'resources':sorted(included),'package_prefix_edges':edges,'unresolved_package_prefix_candidates':unresolved,'unqualified_references_not_selected':local}

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--root',action='append',required=True);p.add_argument('--check',action='store_true');a=p.parse_args()
    base=ROOT/'docs/conformance/2026-08-support';inventory_path=base/'library-reference-inventory.json';inventory=json.loads(inventory_path.read_text(encoding='utf-8'))
    result=select(inventory,[root.split('::') for root in a.root]);release=ROOT.parent/'target/upstream/SysML-v2-Release';hashes=inventory['provenance']['library_inputs']
    for source in result['resources']:
        if hashlib.sha256((release/source).read_bytes()).hexdigest()!=hashes[source]:raise ValueError('Changed pinned resource: '+source)
    label='-'.join(a.root).lower();report={'schema':'dev.mercurio.library-reference-selection.v1','scope':__doc__.strip(),'roots':a.root,'inventory_sha256':hashlib.sha256(inventory_path.read_bytes()).hexdigest(),'generator_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'source_sha256':{s:hashes[s] for s in result['resources']},'semantic_qualification':'not_assessed',**result}
    outputs=[(base/(label+'-reference-resource-selection.json'),report),(base/'definition-pipeline-evidence'/(label+'-reference-source-inputs.json'),{'cases':[{'relative_path':label+'-qualified-reference-source-set','input_files':[(release/s).as_posix() for s in result['resources']]}]})]
    for path,value in outputs:
        text=json.dumps(value,indent=2,sort_keys=True)+'\n'
        if a.check:
            if path.read_text(encoding='utf-8')!=text:raise ValueError('Stale reference source set')
        else:path.write_text(text,encoding='utf-8')
    print('Prepared reference source set:',len(result['resources']),'resources;',len(result['unresolved_package_prefix_candidates']),'unresolved prefix candidates; semantics unqualified')
if __name__=='__main__':main()
