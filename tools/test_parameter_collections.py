"""Focused generation drift controls, independent of Rust implementation."""
import copy,json,unittest
from generate_parameter_collections import INPUT,generate
class ParameterCollections(unittest.TestCase):
    def setUp(self): self.data=json.loads(INPUT.read_text(encoding='utf-8'))
    def test_current(self):
        code=generate(self.data);self.assertIn('"Behavior"',code);self.assertIn('"Step"',code)
    def test_predicate_drift(self):
        self.data['methods']['org.omg.sysml.util.FeatureUtil#isParameter']['statements'][1]['expression']['kind']='CONDITIONAL_OR'
        with self.assertRaises(ValueError): generate(self.data)
    def test_collection_drift(self):
        self.data['methods']['org.omg.sysml.util.TypeUtil#getAllParametersOf']['statements'][0]['expression']['symbol']='org.omg.sysml.util.TypeUtil#getOwnedFeatureOf'
        with self.assertRaises(ValueError): generate(self.data)
    def test_missing_or_duplicate_case(self):
        self.data['controls'][-1]=copy.deepcopy(self.data['controls'][0])
        with self.assertRaises(ValueError): generate(self.data)
    def test_dispatch_override(self):
        self.data['bindings']['Feature']['getRelevantParameters']='other#method'
        with self.assertRaises(ValueError): generate(self.data)
if __name__=='__main__':unittest.main()
