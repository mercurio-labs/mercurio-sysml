"""Expand an existing native audit source set using cached Xtext references.

This selects resource providers conservatively. It does not resolve member
references, implicit library dependencies, scoping or semantic lifecycle.
Existing source identities, producer selections and case controls are preserved.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

from prepare_library_reference_closure import select

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def prepare(spec, inventory, release):
    release = release.resolve()
    catalog = {}
    for row in inventory['resources']:
        path = (release / row['release_path']).resolve()
        if not path.is_relative_to(release):
            raise ValueError('Library resource escapes pinned release')
        if path in catalog:
            raise ValueError('Duplicate resolved library resource')
        catalog[path] = row
    if not spec.get('cases'):
        raise ValueError('Empty source specification')
    expanded = copy.deepcopy(spec)
    reports = []
    for case in expanded['cases']:
        original = list(case['input_files'])
        resolved = [Path(p).resolve() for p in original]
        if not resolved or len(set(resolved)) != len(resolved):
            raise ValueError('Source set requires distinct input identities')
        selection = case.get('literal_value_binding_sources', [])
        if len(set(selection)) != len(selection) or any(p not in original for p in selection):
            raise ValueError('Producer sources must be distinct supplied identities')
        roots = [package for path in resolved if path in catalog
                 for package in catalog[path]['packages']]
        if not roots:
            raise ValueError('Source set has no supplied inventory package roots')
        domain = select(inventory, roots)
        supplied = dict(zip(resolved, original))
        added = []
        for source in domain['resources']:
            path = (release / source).resolve()
            if sha256(path) != inventory['provenance']['library_inputs'][source]:
                raise ValueError('Changed pinned library resource: ' + source)
            if path not in supplied:
                added.append(path.as_posix())
        case['input_files'] = original + added
        reports.append({
            'relative_path': case['relative_path'], 'roots': roots,
            'original_input_files': original, 'added_input_files': added,
            'literal_value_binding_sources': selection,
            'input_sha256': {p: sha256(Path(p)) for p in case['input_files']},
            'external_source_reference_dependencies': 'not selected by this inventory',
            'implicit_dependencies': 'not assessed',
            'native_linking': 'not assessed', 'semantic_qualification': 'not assessed',
            **domain,
        })
    return expanded, reports


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-spec', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--report', type=Path, required=True)
    parser.add_argument('--inventory', type=Path, default=ROOT / 'docs/conformance/2026-08-support/library-reference-inventory.json')
    parser.add_argument('--release', type=Path, default=ROOT.parent / 'target/upstream/SysML-v2-Release')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    spec = json.loads(args.source_spec.read_text(encoding='utf-8'))
    inventory = json.loads(args.inventory.read_text(encoding='utf-8'))
    expanded, cases = prepare(spec, inventory, args.release)
    report = {'schema': 'dev.mercurio.native-reference-inputs.v1',
              'meaning': __doc__.strip(), 'source_spec': args.source_spec.as_posix(),
              'source_spec_sha256': sha256(args.source_spec),
              'inventory_sha256': sha256(args.inventory),
              'generator_sha256': sha256(Path(__file__)),
              'selector_sha256': sha256(Path(__file__).with_name('prepare_library_reference_closure.py')),
              'qualification_certificate': False, 'cases': cases}
    for path, data in [(args.output, expanded), (args.report, report)]:
        text = json.dumps(data, indent=2) + '\n'
        if args.check:
            if path.read_text(encoding='utf-8') != text:
                raise ValueError('Stale prepared inputs: ' + path.as_posix())
        else:
            path.write_text(text, encoding='utf-8')
    print('Prepared', len(cases), 'source set(s):',
          sum(len(c['added_input_files']) for c in cases),
          'additional parsed-reference providers; semantics unqualified')


if __name__ == '__main__':
    main()
