"""Qualify pinned Ecore defaults against fresh Pilot factory instances.

This is a focused build-time audit, not a sample comparison or benchmark.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from export_pilot_grammar_structure import JAR_SHA256

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotDefaultProbe.java'
JAR = ROOT.parent / 'target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar'
WORK = ROOT.parent / 'target/support-2026-08/ecore-defaults'
EVIDENCE = ROOT / 'docs/conformance/2026-08-support/ecore-defaults-constructor-evidence.json'
URI = 'https://www.omg.org/spec/SysML/20250201#//'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def expected_rows(model):
    classes = {row['id']: row for row in model['classes']}
    direct = {}
    for feature in model['features']:
        if feature['default_literal'] is not None:
            direct.setdefault(feature['owner'], []).append(feature)

    def inherited(class_id, seen):
        if class_id in seen:
            raise ValueError('Ecore inheritance cycle: ' + class_id)
        row = classes[class_id]
        result = {}
        for parent in row['super_types']:
            result.update(inherited(parent, seen | {class_id}))
        for feature in direct.get(class_id, []):
            result[feature['name']] = feature
        return result

    result = {}
    for class_id, row in classes.items():
        if row['abstract'] or row['interface'] or not class_id.startswith(URI):
            continue
        for feature in inherited(class_id, set()).values():
            result[(row['name'], feature['name'])] = (feature['owner'].removeprefix(URI), feature['default_value'])
    return result


def audit(rows, expected):
    observed = {(row['class'], row['feature']): row for row in rows}
    if len(observed) != len(rows) or observed.keys() != expected.keys():
        raise ValueError(f'Pilot constructor coverage differs: {len(observed)} observed, {len(expected)} expected')
    overrides = []
    deferred = []
    for key, row in observed.items():
        owner, default = expected[key]
        if row['owner'] != owner or row['ecore_default'] != default:
            raise ValueError(f'{key}: Pilot Ecore identity or default differs')
        if row.get('is_set') is not False or 'is_set_error' in row:
            raise ValueError(f'{key}: unexpected initial eIsSet result')
        if 'initial_error' in row:
            if row['owner'] != 'Feature' or row['feature'] != 'isVariable' or row['initial_error'] != 'NullPointerException':
                raise ValueError(f'{key}: unexpected detached delegate failure')
            deferred.append(row['class'])
        elif row['initial_value'] != default:
            if row['class'] not in ('MembershipExpose', 'NamespaceExpose') or row['feature'] != 'isImportAll' or row['initial_value'] is not True:
                raise ValueError(f'{key}: unexpected constructor override')
            overrides.append({'class': row['class'], 'feature': row['feature'], 'ecore_default': default,
                              'constructor_value': row['initial_value']})
    if len(overrides) != 2 or len(deferred) != 47:
        raise ValueError(f'Pinned constructor behavior drift: {len(overrides)} overrides, {len(deferred)} deferred')
    return sorted(overrides, key=lambda row: row['class']), sorted(deferred)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--java-bin', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if sha(JAR) != JAR_SHA256:
        raise ValueError('Pinned Pilot JAR mismatch')
    model_file = PROFILE / 'ecore-effective.extract.json'
    model = json.loads(model_file.read_text(encoding='utf-8'))
    WORK.mkdir(parents=True, exist_ok=True)
    suffix = '.exe' if os.name == 'nt' else ''
    subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(JAR),
                    '-d', str(WORK), str(HELPER)], check=True)
    raw = WORK / 'constructor-defaults.json'
    subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(WORK) + os.pathsep + str(JAR),
                    'dev.mercurio.pilot.PilotDefaultProbe', str(raw)], check=True)
    observations = json.loads(raw.read_text(encoding='utf-8'))
    if observations['schema'] != 'dev.mercurio.pilot-constructor-defaults.v1':
        raise ValueError('Unexpected Pilot default probe schema')
    rows = observations['rows']
    overrides, deferred = audit(rows, expected_rows(model))
    evidence = {
        'schema': 'dev.mercurio.ecore-defaults-constructor-evidence.v1',
        'scope': 'Fresh detached Pilot factory instances for all concrete pinned SysML EClasses and inherited explicit Ecore defaults; attached-model delegate behavior remains open.',
        'concrete_classes': len({row['class'] for row in rows}),
        'inherited_default_observations': len(rows),
        'matches': len(rows) - len(overrides) - len(deferred),
        'constructor_overrides': overrides,
        'detached_delegate_failures': {'owner': 'Feature', 'feature': 'isVariable', 'count': len(deferred), 'classes': deferred},
        'native_consumer': 'Native source recompilation applies generated Ecore defaults; expose lowering explicitly emits is_import_all=true. Library import and attached-model derived values need separate qualification.',
        'ecore_sha256': sha(model_file), 'jar_sha256': sha(JAR), 'helper_sha256': sha(HELPER),
    }
    encoded = (json.dumps(evidence, indent=2) + '\n').encode()
    if args.check:
        if EVIDENCE.read_bytes() != encoded:
            raise ValueError('Stale Pilot constructor default evidence')
    else:
        EVIDENCE.write_bytes(encoded)
    print(f'Pilot constructor defaults: {len(rows)} observations, {len(overrides)} overrides, {len(deferred)} deferred delegates')


if __name__ == '__main__':
    main()
