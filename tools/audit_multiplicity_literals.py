"""Verify focused Pilot diagnostics for grammar-valid non-Natural range literals.

Use --refresh-pilot to rerun the pinned Java oracle; --check is source-only.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from export_pilot_grammar_structure import JAR_SHA256

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'crates/mercurio-tools/corpus/release-2026-08/multiplicity-literal-bounds/invalid-nonnatural.kerml'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java'
JAR = ROOT.parent / 'target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar'
LIBRARY = ROOT.parent / 'target/upstream/SysML-v2-Pilot-Implementation/sysml.library'
EVIDENCE = ROOT / 'docs/conformance/2026-08-support/multiplicity-literal-bounds-evidence.json'
WORK = ROOT.parent / 'target/support-2026-08/multiplicity-literal-bounds'
EXPECTED = [(2, 32, 'Must have a Natural value'), (3, 31, 'Must have a Natural value'),
            (4, 29, 'Must have a Natural value')]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def record(raw):
    if raw['status'] != 'error' or raw['failure_stage'] != 'resolve_transform':
        raise ValueError('Pilot no longer rejects non-Natural multiplicity literals during transformation')
    diagnostics = [(row['line'], row['column'], row['message']) for row in raw['diagnostics']]
    if diagnostics != EXPECTED:
        raise ValueError(f'Pilot non-Natural diagnostics differ: {diagnostics}')
    return {
        'schema': 'dev.mercurio.multiplicity-literal-bounds-evidence.v1',
        'scope': 'Pinned KerML grammar parses Boolean, string and real MultiplicityExpressionMember literals; Pilot transformation rejects their non-Natural values before successful model construction.',
        'fixture_sha256': digest(FIXTURE),
        'jar_sha256': digest(JAR),
        'helper_sha256': digest(HELPER),
        'pilot_status': raw['status'], 'pilot_failure_stage': raw['failure_stage'],
        'pilot_diagnostics': [{'line': line, 'column': column, 'message': message}
                              for line, column, message in diagnostics],
        'native_control': 'multiplicity_literal_syntax_reaches_pilot_natural_value_check',
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-bin', type=Path)
    parser.add_argument('--raw', type=Path)
    parser.add_argument('--refresh-pilot', action='store_true')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if digest(JAR) != JAR_SHA256:
        raise ValueError('Pinned Pilot JAR mismatch')
    if args.refresh_pilot:
        if args.java_bin is None:
            parser.error('--refresh-pilot requires --java-bin')
        WORK.mkdir(parents=True, exist_ok=True)
        suffix = '.exe' if os.name == 'nt' else ''
        subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(JAR),
                        '-d', str(WORK), str(HELPER)], check=True)
        raw_path = WORK / 'pilot.json'
        with (WORK / 'pilot.log').open('w', encoding='utf-8') as log:
            subprocess.run([str(args.java_bin / ('java' + suffix)), '-Xmx3g', '-cp', str(WORK) + os.pathsep + str(JAR),
                            'dev.mercurio.pilot.PilotModelExporter', '--diagnostics', str(LIBRARY),
                            str(raw_path), str(FIXTURE)], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=600)
    elif args.raw:
        raw_path = args.raw
    else:
        raw_path = None
    if raw_path is not None:
        encoded = (json.dumps(record(json.loads(raw_path.read_text(encoding='utf-8'))), indent=2) + '\n').encode()
        if args.check:
            if EVIDENCE.read_bytes() != encoded:
                raise ValueError('Stale multiplicity literal Pilot evidence')
        else:
            EVIDENCE.write_bytes(encoded)
    else:
        evidence = json.loads(EVIDENCE.read_text(encoding='utf-8'))
        if evidence['schema'] != 'dev.mercurio.multiplicity-literal-bounds-evidence.v1' \
                or evidence['pilot_status'] != 'error' or evidence['pilot_failure_stage'] != 'resolve_transform':
            raise ValueError('Multiplicity literal Pilot evidence status changed')
        if evidence['fixture_sha256'] != digest(FIXTURE) or evidence['jar_sha256'] != digest(JAR) or evidence['helper_sha256'] != digest(HELPER):
            raise ValueError('Multiplicity literal Pilot evidence provenance drift')
        if [(row['line'], row['column'], row['message']) for row in evidence['pilot_diagnostics']] != EXPECTED:
            raise ValueError('Multiplicity literal Pilot evidence changed')
    print('Pilot multiplicity literal diagnostic control current')


if __name__ == '__main__':
    main()
