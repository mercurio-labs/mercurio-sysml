import copy,json,unittest
from pathlib import Path
from export_pilot_default_generals import validate,PROFILE
class PlainTypeDefaults(unittest.TestCase):
    def setUp(self):self.doc=json.loads((PROFILE/"default-general-pilot-controls.json").read_text(encoding="utf-8"))
    def test_current(self):validate(self.doc)
    def test_override_rejected(self):
        self.doc["bindings"][0]["dispatch"]["addDefaultGeneralType"]="other#addDefaultGeneralType"
        with self.assertRaises(ValueError):validate(self.doc)
    def test_binding_drift_rejected(self):
        self.doc["bindings"][0]["kind"]="Interaction"
        with self.assertRaises(ValueError):validate(self.doc)
    def test_selector_drift_rejected(self):
        self.doc["methods"]["org.omg.sysml.adapter.TypeAdapter#getDefaultSupertype"]["statements"][0]["expression"]["arguments"][0]["value"]="other"
        with self.assertRaises(ValueError):validate(self.doc)
    def test_duplicate_control_rejected(self):
        self.doc["controls"][0]=copy.deepcopy(self.doc["controls"][1])
        with self.assertRaises(ValueError):validate(self.doc)
if __name__=="__main__":unittest.main()
