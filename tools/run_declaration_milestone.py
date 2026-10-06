"""Run declaration milestone gates without depending on disposable helper scripts.

Prepare inputs/library with restore_release_assessment_inputs.py, reconstruct
controls with run_declaration_milestone_controls.py, then run checks, audit,
comparison, benchmark and publish in that order. Comparisons are resumable only
while the frozen source and binary hashes remain unchanged.
"""
import argparse
import hashlib
import json
import os
import shutil
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
ARTIFACTS = ROOT.parent / 'target'
SUPPORT = ARTIFACTS / 'support-2026-08'
ASSESSMENT = ARTIFACTS / 'assessment-2026-08-declaration-m1'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sources():
    result = {}
    for base in [ROOT, ROOT.parent / 'mercurio-foundation']:
        names = subprocess.check_output(['git', '-C', str(base), 'ls-files', '--cached', '--others', '--exclude-standard'], text=True).splitlines()
        for name in names:
            path = base / name
            if path.is_file() and ((name.startswith('crates/') and path.suffix in ['.rs', '.toml', '.json', '.sysml', '.kerml', '.rhai', '.yaml', '.yml', '.txt']) or name in ['Cargo.toml', 'Cargo.lock']):
                result[str(path)] = digest(path)
    return result



def verify_declaration_repairs():
    from summarize_release_assessment import load, pairs
    baseline = load(ROOT / 'docs/conformance/2026-08-declaration-validation-assessment/semantic-differences.json')
    expected = [r for r in baseline if r['property'] == 'metaclass']
    if len(expected) != 17:
        raise ValueError('Expected the 17 original metaclass discrepancies')
    cases = {r['relative_path']: r for r in load(ASSESSMENT / 'semantic-summary.json')['cases']}
    documents, repaired = {}, []
    for row in expected:
        case = cases[row['source']]
        if case['status'] != 'compared':
            raise ValueError('Missing comparison for original declaration: ' + row['source'])
        if row['source'] not in documents:
            documents[row['source']] = load(case['artifact']['path'])
        matches = [(n, p) for n, p in pairs(documents[row['source']])
                   if n['declared_name'] == row['name'] and n['source_span']['start_line'] == row['line']]
        if len(matches) != 1:
            raise ValueError('Original declaration disappeared or became ambiguous: ' + str(row))
        native, pilot = matches[0]
        kinds = [(e.get('metatype') or e['kind']).split('::')[-1] for e in [native, pilot]]
        if kinds != [row['pilot'], row['pilot']]:
            raise ValueError('Original metaclass repair failed: ' + str(row))
        repaired.append({'source': row['source'], 'line': row['line'], 'name': row['name'],
                         'before': row['native'], 'native': kinds[0], 'pilot': kinds[1],
                         'native_id': native['id'], 'pilot_id': pilot['id']})
    (ASSESSMENT / 'declaration-repairs.json').write_text(json.dumps({
        'original_metaclass_repairs': repaired,
        'scope': 'All 17 original declaration pairs retained uniquely and verified against fresh Pilot graphs.'
    }, indent=2) + '\n', encoding='utf-8')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage', choices=['checks', 'comparison', 'benchmark', 'publish'], required=True)
    parser.add_argument('--java-bin', type=Path)
    parser.add_argument('--reuse-oracle-only', action='store_true',
                        help='Wait for exports from an already-running oracle stage')
    args = parser.parse_args()
    env = dict(os.environ, CARGO_INCREMENTAL='0')
    if args.java_bin:
        env['PATH'] = str(args.java_bin.resolve()) + os.pathsep + env['PATH']
    for key in list(env):
        if key.startswith('MERCURIO_') and ('LIBRARY' in key or 'STDLIB' in key):
            env.pop(key)
    SUPPORT.mkdir(parents=True, exist_ok=True)
    def run(name, *command):
        print(name + ' started', flush=True)
        with (SUPPORT / ('declaration-m1-' + name + '.log')).open('w', encoding='utf-8') as log:
            result = subprocess.run([str(c) for c in command], env=env, cwd=ROOT,
                                    stdout=log, stderr=subprocess.STDOUT)
        print(name + ' exit ' + str(result.returncode), flush=True)
        if result.returncode:
            raise SystemExit(result.returncode)
    library = SUPPORT / 'stdlib.inherited-types.kir.json'
    if args.stage == 'checks':
        checked_sources = sources()
        # Reuse release dependencies across both component workspaces after a
        # clean recovery, while retaining each workspace's locked manifest.
        run('foundation-tests', 'cargo', 'test', '--manifest-path', '../mercurio-foundation/Cargo.toml',
            '--target-dir', ROOT / 'target', '--release', '--locked', '-j1', '-p', 'mercurio-foundation', '--lib')
        run('sysml-tests', 'cargo', 'test', '--release', '--locked', '-j1', '-p', 'mercurio-sysml', '--lib')
        env['MERCURIO_STDLIB_PATH'] = str(library)
        env['MERCURIO_KERNEL_LIBRARY_PATH'] = str(library)
        run('candidate-tests', 'cargo', 'test', '--release', '--locked', '-j1', '-p', 'mercurio-sysml', '--lib', 'release_2026_08')
        env.pop('MERCURIO_STDLIB_PATH'); env.pop('MERCURIO_KERNEL_LIBRARY_PATH')
        run('sysml-wasm', 'cargo', 'check', '--locked', '-j1', '-p', 'mercurio-sysml', '--target', 'wasm32-unknown-unknown')
        run('workspace-check', 'cargo', 'check', '--locked', '-j1', '--workspace', '--all-targets')
        run('field-generator', sys.executable, '-B', 'tools/generate_release_fields.py',
            '--metamodel', 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/metamodel.extract.json',
            '--selection', 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/mappings/candidate-fields.selection.json',
            '--out', 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/mappings/candidate-fields.extract.json', '--check')
        if sources() != checked_sources:
            raise ValueError('Sources changed while checks ran')
        (SUPPORT / 'declaration-m1-checks-provenance.json').write_text(json.dumps({
            'source_files': checked_sources, 'library_sha256': digest(library),
            'test_profile': 'release', 'checks': ['foundation-tests', 'sysml-tests', 'candidate-tests', 'sysml-wasm', 'workspace-check', 'field-generator']
        }, indent=2) + '\n', encoding='utf-8')
        return
    common = ['--out', ASSESSMENT, '--stdlib', library]
    freeze = [sys.executable, '-B', 'tools/capture_release_assessment.py', '--root', ASSESSMENT]
    if args.stage == 'comparison':
        checked = json.loads((SUPPORT / 'declaration-m1-checks-provenance.json').read_text(encoding='utf-8'))
        if checked['source_files'] != sources() or checked['library_sha256'] != digest(library):
            raise ValueError('Sources/library changed since the check gates')
        required = ['foundation-tests', 'sysml-tests', 'candidate-tests', 'sysml-wasm', 'workspace-check']
        for name in required:
            log = SUPPORT / ('declaration-m1-' + name + '.log')
            if not log.exists() or ('test result: ok.' if 'tests' in name else 'Finished') not in log.read_text(encoding='utf-8'):
                raise ValueError('Missing successful gate: ' + name)
        acceptance = json.loads((SUPPORT / 'declaration-m1-acceptance.json').read_text(encoding='utf-8'))
        if acceptance['library_sha256'] != digest(library):
            raise ValueError('Library changed since the acceptance audit')
        for name, expected_hash in acceptance['binary_sha256'].items():
            if digest(ROOT / 'target/release' / (name + '.exe')) != expected_hash:
                raise ValueError('Binary changed since the acceptance audit: ' + name)
        if (acceptance['samples_passed'], acceptance['controls_passed'], acceptance['original_properties_matched']) != (310, 114, 240):
            raise ValueError('Incomplete acceptance audit')
        fresh = json.loads((SUPPORT / 'declaration-m1-fresh-control-oracle.json').read_text(encoding='utf-8'))
        if len(fresh['cases']) != 21:
            raise ValueError('Expected 21 refreshed control oracles after recovery')
        jar = ARTIFACTS / 'upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar'
        helper = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java'
        if fresh['jar_sha256'] != digest(jar) or fresh['helper_sha256'] != digest(helper):
            raise ValueError('Pilot runtime/helper changed since the control oracle')
        for case in fresh['cases']:
            for name, expected_hash in case['source_sha256'].items():
                if digest(ROOT / name) != expected_hash:
                    raise ValueError('Control changed since its Pilot check: ' + name)
        run('prepare', sys.executable, '-B', 'tools/run_release_assessment.py', '--stage', 'prepare', *common)
        jar = ARTIFACTS / 'upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar'
        (ASSESSMENT / 'classes').mkdir(parents=True, exist_ok=True)
        if args.reuse_oracle_only:
            lock = json.loads((ASSESSMENT / 'source-lock.json').read_text(encoding='utf-8'))
            helper_class = ASSESSMENT / 'classes/dev/mercurio/pilot/PilotModelExporter.class'
            if digest(helper_class) != lock['oracle_sha256']:
                raise ValueError('Active oracle class differs from its recorded identity')
        else:
            run('oracle-compile', 'javac', '-cp', jar, '-d', ASSESSMENT / 'classes',
                'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java')
        run('freeze', *freeze, *(['--verify'] if (ASSESSMENT / 'build-provenance.json').exists() else []))
        run('semantics', sys.executable, '-B', 'tools/run_release_assessment.py', '--stage', 'semantics', *common,
            *(['--reuse-oracle-only'] if args.reuse_oracle_only else []))
        verify_declaration_repairs()
    elif args.stage == 'benchmark':
        semantic = json.loads((ASSESSMENT / 'semantic-summary.json').read_text(encoding='utf-8'))
        if len(semantic['cases']) != 310 or any(c['status'] != 'compared' or c.get('pilot_validation_status') != 'ok' for c in semantic['cases']):
            raise ValueError('Complete all 310 comparisons and Pilot acceptance checks before benchmarking')
        run('verify-before-benchmark', *freeze, '--verify')
        run('performance', sys.executable, '-B', 'tools/run_release_assessment.py', '--stage', 'performance', *common)
    else:
        run('verify-before-publication', *freeze, '--verify')
        run('summarize', sys.executable, '-B', 'tools/summarize_release_assessment.py', '--root', ASSESSMENT)
        run('publish', sys.executable, '-B', 'tools/publish_release_assessment.py', '--root', ASSESSMENT,
            '--dest', 'docs/conformance/2026-08-declaration-m1-assessment')
        destination = ROOT / 'docs/conformance/2026-08-declaration-m1-assessment'
        for name in ['declaration-m1-acceptance.json', 'declaration-m1-fresh-control-oracle.json',
                     'declaration-m1-checks-provenance.json', 'recovered-library.json']:
            shutil.copyfile(SUPPORT / name, destination / name)
        shutil.copyfile(ASSESSMENT / 'declaration-repairs.json', destination / 'declaration-repairs.json')
        run('verify', *freeze, '--verify')


if __name__ == '__main__':
    main()
