import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from export_pilot_ecore_effective import PROFILE, ROOT, cross_check


class EffectiveEcoreTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = json.loads((PROFILE / 'ecore-effective.extract.json').read_text(encoding='utf-8'))
        cls.structural = json.loads((PROFILE / 'metamodel.extract.json').read_text(encoding='utf-8'))

    def test_pinned_cross_check(self):
        report = cross_check(self.doc, self.structural)
        self.assertEqual(report, self.doc['cross_check'])
        self.assertEqual(report['features'], 415)
        self.assertTrue(report['legacy_builtin_type_normalizations'])

    def test_structural_mutations_fail(self):
        for key, value in [('upper_bound', 99), ('ordered', False), ('type', 'urn:unknown#//T'), ('opposite', 'https://www.omg.org/spec/SysML/20250201#//Element/bogus')]:
            with self.subTest(field=key):
                doc = copy.deepcopy(self.doc)
                row = next(r for r in doc['features'] if r['ordered'])
                row[key] = value
                with self.assertRaises(ValueError):
                    cross_check(doc, self.structural)

    def test_missing_and_duplicate_features_fail(self):
        for duplicate in [False, True]:
            doc = copy.deepcopy(self.doc)
            if duplicate:
                doc['features'].append(doc['features'][0])
            else:
                doc['features'].pop()
            with self.assertRaises(ValueError):
                cross_check(doc, self.structural)

    @unittest.skipUnless(os.environ.get('MERCURIO_TEST_JAVA_BIN'), 'explicit build-time Java fixture')
    def test_actual_emf_defaults_explicit_overrides_and_unresolved_reference(self):
        java = Path(os.environ['MERCURIO_TEST_JAVA_BIN']) / ('java.exe' if os.name == 'nt' else 'java')
        classes = ROOT.parent / 'target/support-2026-08/ecore-effective'
        jar = ROOT.parent / 'target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar'
        def invoke(text):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'model.ecore').write_text(text, encoding='utf-8')
                result = subprocess.run([str(java), '-cp', str(classes) + os.pathsep + str(jar), 'dev.mercurio.pilot.PilotEcoreExporter', str(root / 'model.ecore'), str(root / 'out.json')], capture_output=True, text=True)
                return result, json.loads((root / 'out.json').read_text(encoding='utf-8')) if result.returncode == 0 else None
        template = '''<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="test" nsURI="urn:test" nsPrefix="t">
          <eClassifiers xsi:type="ecore:EClass" name="A"><eStructuralFeatures xsi:type="ecore:EAttribute" name="flag" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EBoolean" EXTRA/></eClassifiers></ecore:EPackage>'''
        result, doc = invoke(template.replace('EXTRA', ''))
        self.assertEqual(result.returncode, 0, result.stderr)
        row = doc['features'][0]
        self.assertEqual((row['lower_bound'], row['upper_bound']), (0, 1))
        self.assertTrue(row['ordered'] and row['unique'] and row['changeable'])
        self.assertIsNone(row['default_literal'])
        self.assertFalse(row['default_value'])
        result, doc = invoke(template.replace('EXTRA', 'defaultValueLiteral="true" ordered="false" upperBound="-1"'))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(doc['features'][0]['default_value'])
        self.assertFalse(doc['features'][0]['ordered'])
        self.assertEqual(doc['features'][0]['upper_bound'], -1)
        result, _ = invoke(template.replace('EXTRA', '').replace('http://www.eclipse.org/emf/2002/Ecore#//EBoolean', '#//Missing'))
        self.assertNotEqual(result.returncode, 0)


if __name__ == '__main__':
    unittest.main()
