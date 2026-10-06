"""Reconstruct M1 control/property manifests from retained source and Pilot evidence.
Run from mercurio-sysml after restore_release_assessment_inputs.py --stage inputs.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import os

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'crates/mercurio-tools/corpus/release-2026-08'
EVIDENCE = ROOT / 'docs/conformance/2026-08-support'


def read(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifests(root):
    support = root / 'support-2026-08'
    records = read(EVIDENCE / 'declaration-validation-evidence.json')['validation_controls']
    if len(records) != 92:
        raise ValueError('Historical control inventory changed')
    for filename in ['requirement-constraint', 'relationship-declarations', 'textual', 'flag-name']:
        for case in read(FIXTURES / (filename + '-pilot-results.json'))['cases']:
            records.append({'batch': filename, 'relative_path': case['relative_path'],
                'expected_status': case['status'], 'expected_issue': None, 'pilot': case})
    escaped = read(EVIDENCE / 'textual-declarations-evidence.json')['fresh_pilot_controls']['escaped_language']
    records.append({'batch': 'textual-escaped', 'relative_path': 'textual-escaped.sysml',
        'expected_status': 'ok', 'expected_issue': None, 'source_sha256': escaped['fixture_sha256'],
        'pilot': escaped['cases'][0]})
    if len(records) != 114:
        raise ValueError('Expected 114 accumulated controls')
    cases, expectations = [], []
    historical_sources = read(ROOT / 'docs/conformance/2026-08-declaration-validation-assessment/build-provenance.json')['source_files']
    for row in records:
        relative = Path(row['relative_path'])
        matches = [p for p in [FIXTURES / relative, FIXTURES / row['batch'] / relative.name] if p.is_file()]
        matches = sorted(set(matches)) or sorted(FIXTURES.rglob(relative.name))
        if len(matches) != 1:
            raise ValueError(f'Ambiguous or missing control source: {row["batch"]}/{relative}')
        target = matches[0]
        expected_hash = row.get('source_sha256') or historical_sources.get(target.relative_to(ROOT).as_posix())
        if expected_hash and digest(target) != expected_hash:
            raise ValueError('Retained control source changed: ' + str(target))
        # These two retained fixture groups intentionally cross source files.
        # All other controls are isolated; sibling negative tests must not leak in.
        inputs = sorted(target.parent.glob('*.sysml')) if row['batch'] in ['referenced-port-types', 'semantic-metadata'] else [target]
        inputs = [p for p in inputs if p != target] + [target]
        name = row['batch'] + '/' + relative.as_posix()
        cases.append({'relative_path': name, 'input_files': [str(p.resolve()) for p in inputs]})
        expectations.append({'relative_path': name, 'expected_status': row['expected_status'],
            'expected_issue': row.get('expected_issue'), 'pilot': row['pilot'],
            'source_sha256': {p.relative_to(ROOT).as_posix(): digest(p) for p in inputs},
            'historical_fixture_hash_verified': expected_hash is not None})
    write(support / 'declaration-m1-controls-spec.json', {'cases': cases})
    write(support / 'declaration-m1-controls-expected.json', expectations)
    corpus = read(root / 'release-audit-2026-08-final/corpus.json')['cases']
    rows = read(EVIDENCE / 'flag-name-repairs.json')['rows']
    if len(rows) != 240:
        raise ValueError('Expected all 240 original property rows')
    sources = {r['source'] for r in rows}
    selected = [c for c in corpus if c['relative_path'] in sources]
    if len(selected) != 54:
        raise ValueError('Expected 54 original property contexts')
    write(support / 'flag-name-audit-spec.json', {'cases': selected, 'expectations': [
        {k: r[k] for k in ['source', 'line', 'name', 'property']} | {'pilot': r['expected']} for r in rows]})
    print('Reconstructed 114 controls and 240 property expectations across 54 original contexts', flush=True)


def audit(root):
    support = root / 'support-2026-08'
    env = dict(os.environ)
    env['MERCURIO_STDLIB_PATH'] = str(support / 'stdlib.inherited-types.kir.json')
    env['MERCURIO_KERNEL_LIBRARY_PATH'] = env['MERCURIO_STDLIB_PATH']
    def run(name, binary, spec, expected_exit):
        output = support / (name + '.jsonl')
        with (support / (name + '.log')).open('w', encoding='utf-8') as log:
            result = subprocess.run([str(ROOT / 'target/release' / (binary + '.exe')), str(spec), str(output)],
                env=env, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode != expected_exit:
            raise ValueError(f'{name}: unexpected exit {result.returncode}; inspect log')
        return [json.loads(line) for line in output.read_text(encoding='utf-8').splitlines()]
    samples = run('declaration-m1-corpus', 'audit_release_compile', root / 'release-audit-2026-08-final/corpus.json', 0)
    if len(samples) != 310 or any(r['status'] != 'ok' for r in samples):
        raise ValueError('Corpus gate failed')
    controls = run('declaration-m1-controls', 'audit_release_compile', support / 'declaration-m1-controls-spec.json', 1)
    expected = {r['relative_path']: r for r in read(support / 'declaration-m1-controls-expected.json')}
    if len(controls) != len(expected) or {r['relative_path'] for r in controls} != set(expected):
        raise ValueError('Incomplete or duplicated control results')
    for row in controls:
        oracle = expected[row['relative_path']]
        if row['status'] != oracle['expected_status']:
            raise ValueError('Control acceptance mismatch: ' + str(row))
        issue = oracle['expected_issue']
        if issue and issue not in json.dumps(row.get('diagnostics')):
            raise ValueError('Control diagnostic mismatch: ' + str(row))
    properties = run('declaration-m1-properties', 'audit_release_properties', support / 'flag-name-audit-spec.json', 0)
    if len(properties) != 240 or any(r['status'] != 'match' for r in properties):
        raise ValueError('Property repair gate failed')
    write(support / 'declaration-m1-acceptance.json', {'samples_passed': len(samples),
        'controls_passed': len(controls), 'original_properties_matched': len(properties),
        'library_sha256': digest(support / 'stdlib.inherited-types.kir.json'),
        'binary_sha256': {name: digest(ROOT / 'target/release' / (name + '.exe')) for name in ['audit_release_compile', 'audit_release_properties']},
        'control_oracle': '93 hash-verified historical Pilot controls and 21 refreshed Pilot controls; fresh native compilation with recorded source contexts.'})
    print('Passed: 310 sample contexts, 114 controls, 240 original property rows', flush=True)


def refresh_oracle(root, java_bin):
    from concurrent.futures import ThreadPoolExecutor
    support = root / 'support-2026-08'
    pilot = root / 'upstream/SysML-v2-Pilot-Implementation'
    jar = root / 'upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar'
    classes = support / 'control-oracle-classes'
    classes.mkdir(exist_ok=True)
    java = str(java_bin / 'java.exe') if java_bin else 'java'
    javac = str(java_bin / 'javac.exe') if java_bin else 'javac'
    helper = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java'
    subprocess.run([javac, '-cp', str(jar), '-d', str(classes), str(helper)], check=True)
    cases = {c['relative_path']: c for c in read(support / 'declaration-m1-controls-spec.json')['cases']}
    expected = read(support / 'declaration-m1-controls-expected.json')
    pending = [r for r in expected if not r['historical_fixture_hash_verified']]
    def run(row):
        folder = support / 'control-oracle' / row['relative_path'].replace('/', '__')
        folder.mkdir(parents=True, exist_ok=True)
        spec, output = folder / 'spec.json', folder / 'timings.json'
        write(spec, {'cases': [cases[row['relative_path']]]})
        with (folder / 'oracle.log').open('w', encoding='utf-8') as log:
            subprocess.run([java, '-Xmx3g', '-cp', str(classes) + os.pathsep + str(jar),
                'dev.mercurio.pilot.PilotModelExporter', '--assessment', str(pilot / 'sysml.library'),
                str(spec), '-', str(output)], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=600)
        result = read(output)['cases']
        if len(result) != 1 or result[0]['status'] != row['expected_status']:
            raise ValueError('Refreshed Pilot control disagrees: ' + row['relative_path'])
        print('Pilot control verified: ' + row['relative_path'], flush=True)
        return {'relative_path': row['relative_path'], 'pilot': result[0], 'source_sha256': row['source_sha256']}
    with ThreadPoolExecutor(max_workers=2) as pool:
        results = list(pool.map(run, pending))
    write(support / 'declaration-m1-fresh-control-oracle.json', {'cases': results,
        'jar_sha256': digest(jar), 'helper_sha256': digest(helper),
        'scope': 'Fresh isolated CheckMode.ALL acceptance controls; not benchmark measurements.'})
    print(f'Refreshed {len(results)} controls with current source digests', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage', choices=['manifests', 'oracle', 'audit'], required=True)
    parser.add_argument('--artifacts', type=Path, default=ROOT.parent / 'target')
    parser.add_argument('--java-bin', type=Path)
    args = parser.parse_args()
    if args.stage == 'oracle':
        refresh_oracle(args.artifacts.resolve(), args.java_bin)
        return
    (manifests if args.stage == 'manifests' else audit)(args.artifacts.resolve())


if __name__ == '__main__':
    main()
