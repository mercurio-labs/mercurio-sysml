"""Resolve effective runtime Ecore metadata with pinned EMF; cross-check native schema."""
import argparse
import json
import os
from pathlib import Path
import subprocess
from export_pilot_grammar_structure import ROOT, JAR_SHA256, digest, encode
from extract_ecore_semantics import verify_pilot, PINNED_COMMIT, MODEL

PROFILE = ROOT / 'crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08'
HELPER = ROOT / 'tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotEcoreExporter.java'
OUTPUT = PROFILE / 'ecore-effective.extract.json'


def cross_check(doc, structural):
    def name(ref):
        if ref is None:
            return None
        if '#//' not in ref:
            raise ValueError('Invalid resolved reference: ' + ref)
        ns, local = ref.split('#//', 1)
        package = {'https://www.omg.org/spec/SysML/20250201': 'SysML',
                   'http://www.eclipse.org/emf/2002/Ecore': 'Ecore'}.get(ns)
        if package is None:
            raise ValueError('Unknown resolved namespace: ' + ns)
        return package + '::' + local.replace('/', '::')
    actual = {name(r['id']): r for r in doc['features']}
    expected = {r['qualified_name']: r for r in structural['structural_features'] if r['owner'].startswith('SysML::')}
    if len(actual) != len(doc['features']) or actual.keys() != expected.keys():
        raise ValueError('Effective feature identity coverage differs')
    fields = ['kind', 'lower_bound', 'upper_bound', 'containment', 'derived', 'transient', 'volatile', 'ordered', 'unique']
    normalizations = []
    for key, row in actual.items():
        old = expected[key]
        for field in fields:
            if row[field] != old[field]:
                raise ValueError(f'{key}: {field} differs')
        for field, target in [('opposite', 'opposite'), ('type', 'target')]:
            resolved = name(row[field])
            if resolved != old[target]:
                # Existing extractor represents Ecore data types without a package prefix.
                if field == 'type' and resolved in ('Ecore::EString', 'Ecore::EBoolean', 'Ecore::EInt', 'Ecore::EDouble') and old[target] == 'SysML::' + resolved.split('::')[1]:
                    normalizations.append({'feature': key, 'legacy': old[target], 'resolved': resolved})
                else:
                    raise ValueError(f'{key}: {field} differs: {resolved} != {old[target]}')
        if row['default_literal'] != old['default_value'] or row['is_id'] != old['id']:
            raise ValueError(f'{key}: declared default or ID differs')
    classes = {name(r['id']): r for r in doc['classes']}
    oldclasses = {r['qualified_name']: r for r in structural['metaclasses'] if r['package'] == 'SysML'}
    if classes.keys() != oldclasses.keys():
        raise ValueError('Effective class coverage differs')
    for key, row in classes.items():
        old = oldclasses[key]
        supers = [r['general'] for r in structural['generalizations'] if r['specific'] == key]
        if row['abstract'] != old['abstract_class'] or row['interface'] != old['interface'] or sorted(map(name, row['super_types'])) != sorted(supers):
            raise ValueError(f'{key}: class flags or inheritance differ')
    return {'legacy_builtin_type_normalizations': normalizations, 'classes': len(classes), 'features': len(actual), 'status': 'matched',
            'scope': 'runtime SysML.ecore only; separate kerml.ecore package is not loaded'}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--java-bin', type=Path, required=True)
    p.add_argument('--check', action='store_true')
    args = p.parse_args()
    pilot = ROOT.parent / 'target/upstream/SysML-v2-Pilot-Implementation'
    jar = ROOT.parent / 'target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar'
    verify_pilot(pilot, PINNED_COMMIT)
    if digest(jar) != JAR_SHA256:
        raise ValueError('Pinned JAR mismatch')
    tracked = [jar, pilot / MODEL, HELPER, Path(__file__), PROFILE / 'metamodel.extract.json']
    before = [digest(path) for path in tracked]
    work = ROOT.parent / 'target/support-2026-08/ecore-effective'
    work.mkdir(parents=True, exist_ok=True)
    suffix = '.exe' if os.name == 'nt' else ''
    subprocess.run([str(args.java_bin / ('javac' + suffix)), '-encoding', 'UTF-8', '-cp', str(jar), '-d', str(work), str(HELPER)], check=True)
    raw = work / 'raw.json'
    subprocess.run([str(args.java_bin / ('java' + suffix)), '-cp', str(work) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotEcoreExporter', str(pilot / MODEL), str(raw)], check=True)
    doc = json.loads(raw.read_text(encoding='utf-8'))
    structural = PROFILE / 'metamodel.extract.json'
    doc['cross_check'] = cross_check(doc, json.loads(structural.read_text(encoding='utf-8')))
    doc['provenance'] = {'pilot_commit': PINNED_COMMIT, 'jar_sha256': digest(jar), 'ecore_sha256': digest(pilot / MODEL), 'helper_sha256': digest(HELPER), 'driver_sha256': digest(Path(__file__)), 'structural_sha256': digest(structural)}
    if [digest(path) for path in tracked] != before:
        raise ValueError('Exporter inputs changed during export')
    result = encode(doc)
    if args.check:
        if OUTPUT.read_bytes() != result:
            raise ValueError('Stale effective Ecore metadata')
    else:
        OUTPUT.write_bytes(result)
    print('Verified effective Ecore:', len(doc['classes']), 'classes,', len(doc['features']), 'features,', len(doc['operations']), 'operations')


if __name__ == '__main__':
    main()
