import copy,hashlib,json,unittest
from run_fresh_transition_link_probe import ROOT,OUTPUT,HELPER,validate
from generate_fresh_transition_link import generate
class FreshTransitionLinkTests(unittest.TestCase):
    def setUp(self):self.data=json.loads(OUTPUT.read_text(encoding="utf-8"))
    def test_cache_and_provenance(self):
        validate(self.data)
        for key,path in [("helper_sha256",HELPER),("driver_sha256",ROOT/"tools/run_fresh_transition_link_probe.py")]:self.assertEqual(self.data["provenance"][key],hashlib.sha256(path.read_bytes()).hexdigest())
    def test_missing_context_rejected(self):
        d=copy.deepcopy(self.data);d["controls"].pop()
        with self.assertRaises(ValueError):validate(d)
    def test_partial_output_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["after"]["complete"]=False
        with self.assertRaises(ValueError):validate(d)
    def test_variable_context_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["may_time_vary"]=True
        with self.assertRaises(ValueError):validate(d)
    def test_repeat_drift_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["repeat"]["generals"]=[]
        with self.assertRaises(ValueError):validate(d)
    def test_missing_library_stage_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["library_inputs"].pop("dataValues")
        with self.assertRaises(ValueError):validate(d)
    def test_generation_consumes_resolved_receiver(self):self.assertIn('const KIND: &str = "ReferenceUsage"',generate(self.data))
    def test_resolved_lifecycle_drift_rejected(self):
        d=copy.deepcopy(self.data);d["methods"]["org.omg.sysml.adapter.FeatureAdapter#doTransform"]["kind"]="EMPTY_STATEMENT"
        with self.assertRaises(Exception):generate(d)
if __name__=="__main__":unittest.main()
