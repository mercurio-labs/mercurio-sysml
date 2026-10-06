import copy,hashlib,json,unittest
from run_transition_owner_lifecycle_probe import ROOT,OUTPUT,HELPER,validate
class TransitionOwnerStageTests(unittest.TestCase):
    def setUp(self):self.data=json.loads(OUTPUT.read_text(encoding="utf-8"))
    def test_cache_and_provenance(self):
        validate(self.data)
        for key,path in [("helper_sha256",HELPER),("driver_sha256",ROOT/"tools/run_transition_owner_lifecycle_probe.py")]:self.assertEqual(self.data["provenance"][key],hashlib.sha256(path.read_bytes()).hexdigest())
    def test_missing_context_rejected(self):
        d=copy.deepcopy(self.data);d["controls"].pop()
        with self.assertRaises(ValueError):validate(d)
    def test_partial_link_rejected(self):
        d=copy.deepcopy(self.data);next(c for c in d["controls"] if c["link"])["link_snapshot"]["complete"]=False
        with self.assertRaises(ValueError):validate(d)
    def test_constructor_drop_rejected(self):
        d=copy.deepcopy(self.data);next(c for c in d["controls"] if c["connectors"])["connectors"]=[]
        with self.assertRaises(ValueError):validate(d)
    def test_repeat_drift_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["repeat_connector_count"]=1
        with self.assertRaises(ValueError):validate(d)
    def test_owner_completion_required(self):
        d=copy.deepcopy(self.data);d["controls"][0]["owner_after"]["complete"]=False
        with self.assertRaises(ValueError):validate(d)
    def test_owner_repeat_drift_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["owner_repeat"]["relationships"].append("Membership")
        with self.assertRaises(ValueError):validate(d)
    def test_missing_role_stage_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["owner_role_input"]["complete"]=False
        with self.assertRaises(ValueError):validate(d)
if __name__=="__main__":unittest.main()
