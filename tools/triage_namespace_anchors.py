"""Inventory every unmatched named declaration from a completed assessment.

Evidence only: source spelling and same-name candidates do not prove equivalence.
This uses the assessment's exact unique (declared name, start line) pairing rule.
"""
import argparse
import collections
import json
from pathlib import Path

from summarize_release_assessment import direct, load

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assessment', type=Path, required=True)
    parser.add_argument('--pilot-root', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    summary = load(args.assessment / 'semantic-summary.json')
    if len(summary['cases']) != 310 or any(r['status'] != 'compared' for r in summary['cases']):
        raise ValueError('A completed 310-target assessment is required')
    rows = []
    kinds = collections.defaultdict(collections.Counter)
    for row in summary['cases']:
        document = load(row['artifact']['path'])
        lines = (args.pilot_root / row['relative_path']).read_text(encoding='utf-8').splitlines()
        indexes = []
        for side in ['mercurio_snapshot', 'pilot_snapshot']:
            index = collections.defaultdict(list)
            for element in document[side]['elements']:
                line = (element.get('source_span') or {}).get('start_line')
                if element.get('declared_name') and line is not None:
                    index[(element['declared_name'], line)].append(element)
            indexes.append(index)
        def describe(element):
            properties = direct(element)
            return {'id': element['id'], 'kind': element['kind'],
                    'metatype': element.get('metatype'),
                    'line': element['source_span']['start_line'],
                    'properties': {k: properties[k] for k in ['owner', 'owning_namespace',
                        'owning_membership', 'owning_feature_membership', 'type', 'specializes',
                        'source', 'target', 'is_implied', 'is_implied_included'] if k in properties}}
        for name, line in sorted(indexes[0].keys() | indexes[1].keys()):
            native = indexes[0].get((name, line), [])
            pilot = indexes[1].get((name, line), [])
            if len(native) > 1 or len(pilot) > 1:
                status = 'ambiguous'
            elif native and not pilot:
                status = 'native_only'
            elif pilot and not native:
                status = 'pilot_only'
            else:
                continue
            source_line = lines[line - 1] if 0 < line <= len(lines) else None
            candidates = []
            if status != 'ambiguous':
                other = indexes[1] if native else indexes[0]
                candidates = [describe(e) for (other_name, other_line), elements in other.items()
                              if other_name == name and other_line != line for e in elements]
            rows.append({'source': row['relative_path'], 'line': line, 'name': name,
                         'status': status, 'source_line': source_line,
                         'native': [describe(e) for e in native], 'pilot': [describe(e) for e in pilot],
                         'same_name_other_line_candidates': candidates,
                         'classification': 'unassessed'})
            for element in native if status == 'native_only' else pilot:
                kinds[status][element['kind']] += 1
    counts = collections.Counter(r['status'] for r in rows)
    measured = load(args.assessment / 'semantic-analysis.json')['named_declaration_projection']
    expected = {'native_only': measured['native_named_anchors_without_pilot_pair'],
                'pilot_only': measured['pilot_named_anchors_without_native_pair'],
                'ambiguous': measured['ambiguous_named_anchors']}
    if dict(counts) != expected:
        raise ValueError(f'Inventory does not reconcile with the published projection: {counts} != {expected}')
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps({'scope': 'M1 named declaration anchors; every row remains unassessed until evidence-backed triage.',
        'source_fingerprint': load(args.assessment / 'build-provenance.json')['source_fingerprint'],
        'counts': dict(counts), 'kinds': {k: dict(v.most_common()) for k, v in kinds.items()},
        'rows': rows}, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'counts': dict(counts), 'kinds': {k: dict(v.most_common()) for k, v in kinds.items()}}, indent=2))


if __name__ == '__main__':
    main()
