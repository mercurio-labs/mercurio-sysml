import json
import unittest
from export_pilot_featuring_queries import OUTPUT,render
class FeaturingQueryTests(unittest.TestCase):
    def setUp(self): self.doc=json.loads(OUTPUT.read_text())
    def test_resolved_delegate_generates_native_binding(self):
        self.assertIn("Feature_featuringType_SettingDelegate",render(self.doc))
    def test_missing_boundary_context_rejected(self):
        self.doc["cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete featuring"):render(self.doc)
    def test_resolved_traversal_change_rejected(self):
        self.doc["definitions"]["FeatureUtil#getAllFeaturingTypesOf"]["kind"]="EMPTY_STATEMENT"
        with self.assertRaisesRegex(ValueError,"Changed resolved"):render(self.doc)
    def test_duplicate_context_rejected(self):
        self.doc["cases"][0]=self.doc["cases"][1]
        with self.assertRaisesRegex(ValueError,"Incomplete featuring"):render(self.doc)
