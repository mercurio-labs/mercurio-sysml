import copy,hashlib,json,unittest
from run_transition_members_probe import ROOT,OUTPUT,HELPER,validate
from generate_transition_members import generate,INPUT,ECORE
class TransitionMemberTests(unittest.TestCase):
    def setUp(self):self.data=json.loads(OUTPUT.read_text(encoding="utf-8"))
    def test_cache_and_provenance(self):
        validate(self.data)
        for key,path in [("helper_sha256",HELPER),("driver_sha256",ROOT/"tools/run_transition_members_probe.py")]:self.assertEqual(self.data["provenance"][key],hashlib.sha256(path.read_bytes()).hexdigest())
    def test_missing_branch_rejected(self):
        d=copy.deepcopy(self.data);d["controls"].pop()
        with self.assertRaises(ValueError):validate(d)
    def test_dropped_connector_rejected(self):
        d=copy.deepcopy(self.data);next(c for c in d["controls"] if c["connectors"])["connectors"]=[]
        with self.assertRaises(ValueError):validate(d)
    def test_repeat_drift_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["repeat_connector_count"]=1
        with self.assertRaises(ValueError):validate(d)
    def test_partial_binding_rejected(self):
        d=copy.deepcopy(self.data);next(c for c in d["controls"] if c["connectors"])["connectors"][0]["connector"]["complete"]=False
        with self.assertRaises(ValueError):validate(d)
    def test_resolved_program_generation(self):
        d=json.loads(INPUT.read_text(encoding="utf-8"));ecore=json.loads(ECORE.read_text(encoding="utf-8"))
        self.assertIn('const LINK: &str = "ReferenceUsage"',generate(d,ecore))
    def test_resolved_program_drift_rejected(self):
        d=json.loads(INPUT.read_text(encoding="utf-8"));ecore=json.loads(ECORE.read_text(encoding="utf-8"));d["methods"]["org.omg.sysml.adapter.TransitionUsageAdapter#computeTransitionLinkConnectors"]["kind"]="EMPTY_STATEMENT"
        with self.assertRaises(Exception):generate(d,ecore)
if __name__=="__main__":unittest.main()
