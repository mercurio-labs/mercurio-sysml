import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from prepare_native_reference_inputs import prepare


class NativeReferenceInputsTests(unittest.TestCase):
    def fixture(self, root):
        rows = []
        hashes = {}
        for name, refs in [('P', [['Q', 'member']]), ('Q', [])]:
            p = root / (name + '.kerml')
            p.write_text('package ' + name + ';', encoding='utf-8')
            hashes[p.name] = hashlib.sha256(p.read_bytes()).hexdigest()
            rows.append({'release_path': p.name, 'packages': [[name]], 'imports': [],
                         'references': [{'owner_package': [name], 'target_segments': r} for r in refs]})
        caller = root / 'caller.kerml'
        caller.write_text('package Caller { feature x = 1; }', encoding='utf-8')
        spec = {'cases': [{'relative_path': 'original', 'input_files': [(root / 'P.kerml').as_posix(), caller.as_posix()],
                           'literal_value_binding_sources': [caller.as_posix()],
                           'inspection_queries': [{'owner_path': ['Caller'], 'field': 'result'}]}]}
        return spec, {'resources': rows, 'provenance': {'library_inputs': hashes}}

    def test_reference_provider_expansion_preserves_original_case_contract(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); spec, inventory = self.fixture(root)
            original = json.loads(json.dumps(spec))
            expanded, reports = prepare(spec, inventory, root)
            self.assertEqual(spec, original)
            self.assertEqual(expanded['cases'][0]['input_files'], original['cases'][0]['input_files'] + [(root / 'Q.kerml').resolve().as_posix()])
            for field in ['relative_path', 'literal_value_binding_sources', 'inspection_queries']:
                self.assertEqual(expanded['cases'][0][field], original['cases'][0][field])
            self.assertEqual(reports[0]['native_linking'], 'not assessed')
            self.assertEqual(reports[0]['implicit_dependencies'], 'not assessed')

    def test_changed_provider_pin_rejects_before_native_linking(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); spec, inventory = self.fixture(root)
            (root / 'Q.kerml').write_text('package Q { feature altered; }', encoding='utf-8')
            with self.assertRaisesRegex(ValueError, 'Changed pinned'):
                prepare(spec, inventory, root)

    def test_unknown_and_unqualified_reference_dependencies_stay_explicit(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); spec, inventory = self.fixture(root)
            inventory['resources'][0]['references'].extend([
                {'owner_package': ['P'], 'target_segments': ['Unknown', 'value']},
                {'owner_package': ['P'], 'target_segments': ['local']},
            ])
            _, reports = prepare(spec, inventory, root)
            self.assertEqual(len(reports[0]['unresolved_package_prefix_candidates']), 1)
            self.assertEqual(len(reports[0]['unqualified_references_not_selected']), 1)
            self.assertEqual(reports[0]['semantic_qualification'], 'not assessed')

    def test_bad_inputs_and_producer_scope_reject(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); spec, inventory = self.fixture(root)
            for variant in ['duplicate_input', 'foreign_producer', 'escape']:
                s = json.loads(json.dumps(spec)); d = json.loads(json.dumps(inventory))
                if variant == 'duplicate_input': s['cases'][0]['input_files'].append(s['cases'][0]['input_files'][0])
                if variant == 'foreign_producer': s['cases'][0]['literal_value_binding_sources'] = ['unknown.kerml']
                if variant == 'escape': d['resources'][0]['release_path'] = '../escape.kerml'
                with self.subTest(variant=variant), self.assertRaises(ValueError):
                    prepare(s, d, root)

    def test_pinned_value_domain_adds_all_six_missing_reference_providers(self):
        root = Path(__file__).resolve().parents[1]
        base = root / 'docs/conformance/2026-08-support'
        spec = json.loads((base / 'definition-pipeline-evidence/genuine-literal-producer-preflight-inputs.json').read_text(encoding='utf-8'))
        inventory = json.loads((base / 'library-reference-inventory.json').read_text(encoding='utf-8'))
        # Resource selection is independent of any local upstream installation.
        from prepare_library_reference_closure import select
        supplied = {p.split('/SysML-v2-Release/', 1)[1] for p in spec['cases'][0]['input_files'] if '/SysML-v2-Release/' in p}
        roots = [p for r in inventory['resources'] if r['release_path'] in supplied for p in r['packages']]
        result = select(inventory, roots)
        added = {Path(p).name for p in set(result['resources']) - supplied}
        self.assertEqual(added, {'BaseFunctions.kerml', 'ComplexFunctions.kerml', 'DataFunctions.kerml',
                                'NumericalFunctions.kerml', 'RationalFunctions.kerml', 'RealFunctions.kerml'})
        self.assertTrue(any(e.get('kind') == 'Subclassification' and e['target_segments'] == ['BaseFunctions', '==']
                            and e['provider'] and e['provider'].endswith('/BaseFunctions.kerml')
                            for e in result['package_prefix_edges']))
