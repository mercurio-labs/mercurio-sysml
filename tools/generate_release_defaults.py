"""Generate native implicit definition defaults from a clean Pilot model export.

The fixture has no explicit specializations. The compiler fills missing defaults
from these observations; curated existing defaults retain precedence.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def derive(document, namespace, kinds=()):
    elements = {e['qualified_name']: e for e in document['elements'] if e['qualified_name'].startswith(namespace + '::') and (e['kind'] in kinds if kinds else e['kind'].endswith('Definition'))}
    defaults = {}
    for name, element in elements.items():
        parents = sorted({r['target'] for r in document['relationships'] if r['source'] == name and r['relation'] == 'specializes' and not r['target'].startswith(namespace + '::')})
        if not parents:
            raise ValueError(f'no observed specialization for {name}')
        kind = element['kind']
        if kind in defaults and defaults[kind] != parents:
            raise ValueError(f'inconsistent observed defaults for {kind}')
        defaults[kind] = parents
    if not defaults:
        raise ValueError('empty default observation set')
    return dict(sorted(defaults.items()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('export', 'fixture', 'pilot-root', 'jar', 'out'):
        parser.add_argument('--' + arg, type=Path, required=True)
    parser.add_argument('--namespace', default='ReleaseDefaults')
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--kind', action='append', default=[])
    args = parser.parse_args()
    def git(*command):
        return subprocess.check_output(['git', '-C', str(args.pilot_root), *command], text=True).strip()
    if git('status', '--porcelain'):
        raise ValueError('Pilot checkout must be clean')
    export = json.loads(args.export.read_text(encoding='utf-8'))
    output = {'schema': 'dev.mercurio.observed-definition-defaults.v1',
              'source': {'pilot_commit': git('rev-parse', 'HEAD'), 'pilot_git_describe': git('describe', '--tags', '--exact-match'), 'pilot_dirty': False,
                         'export_sha256': digest(args.export), 'fixture_sha256': digest(args.fixture), 'jar_sha256': digest(args.jar),
                         'extractor': 'generate_release_defaults.py', 'extractor_version': 1, 'export_metadata': export.get('metadata', {})},
              'definitions': derive(export, args.namespace, args.kind)}
    if args.check:
        assert output == json.loads(args.out.read_text(encoding='utf-8')), 'observed-defaults artifact drift'
    else:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(output, indent=2) + '\n', encoding='utf-8')
    print(f'{len(output["definitions"])} definition defaults verified')


if __name__ == '__main__':
    main()
