"""Prepare a source set from parsed Xtext imports; not semantic/global closure proof."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = ROOT / 'docs/conformance/2026-08-support/library-import-inventory.json'
OUTPUT = ROOT / 'docs/conformance/2026-08-support/definition-pipeline-evidence/performances-import-source-inputs.json'
REPORT = ROOT / 'docs/conformance/2026-08-support/performances-import-closure.json'

def select(inventory, root_package):
    resources, packages = {}, {}
    for row in inventory['resources']:
        name = row['release_path']
        if name in resources:
            raise ValueError('Duplicate resource identity')
        resources[name] = row
        for package in row['packages']:
            key = tuple(package)
            if not key or key in packages:
                raise ValueError('Empty or ambiguous package path')
            packages[key] = name
    root = packages.get(tuple(root_package))
    if root is None:
        raise ValueError('Missing root package')
    pending, included, edges, unresolved = [root], set(), [], []
    while pending:
        source = pending.pop()
        if source in included:
            continue
        included.add(source)
        for declaration in resources[source]['imports']:
            target = tuple(declaration['target_segments'])
            if not target:
                if not declaration.get('implicit_filter_target'):
                    raise ValueError('Missing explicit import target')
                continue  # Nested filter imports have their own declarations.
            owner = tuple(declaration['owner_package'])
            provider = None
            for depth in range(len(owner), -1, -1):
                qualified = owner[:depth] + target
                # A prefix must consume a target segment: the owner package alone
                # must never turn every external import into a false self edge.
                for length in range(len(qualified), depth, -1):
                    if qualified[:length] in packages:
                        provider = packages[qualified[:length]]
                        break
                if provider is not None:
                    break
            edge = {'source': source, 'target_segments': list(target), 'owner_package': list(owner), 'provider': provider}
            edges.append(edge)
            if provider is None:
                unresolved.append(edge)
            else:
                pending.append(provider)
    return {'resources': sorted(included), 'package_prefix_edges': edges, 'unresolved_import_prefixes': unresolved}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    inventory = json.loads(INVENTORY.read_text(encoding='utf-8'))
    result = select(inventory, ['Performances'])
    if result['unresolved_import_prefixes']:
        raise ValueError('Unresolved explicit import package prefixes')
    release = ROOT.parent / 'target/upstream/SysML-v2-Release'
    hashes = inventory['provenance']['library_inputs']
    for source in result['resources']:
        data = (release / source).read_bytes()
        if hashlib.sha256(data).hexdigest() != hashes[source]:
            raise ValueError('Changed pinned resource: ' + source)
    spec = {'cases': [{'relative_path': 'Performances-explicit-import-source-set.kerml', 'input_files': [(release / p).as_posix() for p in result['resources']]}]}
    report = {'schema': 'dev.mercurio.performances-import-closure.v1',
              'scope': 'Parsed explicit-import package-prefix resource selection only. Full import target linking, implicit/global library dependencies, construction and semantics require native assessment.',
              'inventory_sha256': hashlib.sha256(INVENTORY.read_bytes()).hexdigest(),
              'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'source_sha256': {p: hashes[p] for p in result['resources']}, **result}
    for path, value in [(OUTPUT, spec), (REPORT, report)]:
        text = json.dumps(value, indent=2, sort_keys=True) + '\n'
        if args.check:
            if path.read_text(encoding='utf-8') != text:
                raise ValueError('Stale import source set: ' + str(path))
        else:
            path.write_text(text, encoding='utf-8', newline='\n')
    print('Prepared explicit-import source set:', len(result['resources']), 'resources; semantic/global closure unqualified')

if __name__ == '__main__':
    main()
