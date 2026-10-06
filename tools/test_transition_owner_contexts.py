import copy,hashlib,json,unittest
from run_transition_owner_context_probe import ROOT,OUTPUT,HELPER,validate
class TransitionOwnerContextsTests(unittest.TestCase):
    def setUp(self):self.data=json.loads(OUTPUT.read_text(encoding="utf-8"))
    def test_cached_inventory_and_provenance(self):
        validate(self.data)
        self.assertEqual(sum(len(c["children_before"]) for c in self.data["controls"]),240)
        for key,path in [("helper_sha256",HELPER),("driver_sha256",ROOT/"tools/run_transition_owner_context_probe.py")]:self.assertEqual(self.data["provenance"][key],hashlib.sha256(path.read_bytes()).hexdigest())
        for n,h in self.data["provenance"]["observation_helpers_sha256"].items():self.assertEqual(h,hashlib.sha256(HELPER.with_name(n+".java").read_bytes()).hexdigest())
    def test_same_resolved_child_programs(self):
        prior=json.loads((ROOT/"docs/conformance/2026-08-support/transition-owner-recursive-pilot-controls.json").read_text(encoding="utf-8"))
        self.assertEqual(prior["methods"],self.data["methods"])
    def test_missing_parent_context_rejected(self):
        d=copy.deepcopy(self.data);d["controls"]=[c for c in d["controls"] if c["parent_kind"]!="StateUsage"]
        with self.assertRaises(ValueError):validate(d)
    def test_duplicate_context_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]=d["controls"][1]
        with self.assertRaises(ValueError):validate(d)
    def test_uncompleted_parent_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["parent_complete"]=False
        with self.assertRaises(ValueError):validate(d)
    def test_uncompleted_performance_role_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["performance_role_input"]["complete"]=False
        with self.assertRaises(ValueError):validate(d)
    def test_missing_child_rejected(self):
        d=copy.deepcopy(self.data);c=next(c for c in d["controls"] if c["children_after"]);c["children_after"].pop(next(iter(c["children_after"])))
        with self.assertRaises(ValueError):validate(d)
    def test_changed_repeat_rejected(self):
        d=copy.deepcopy(self.data);c=next(c for c in d["controls"] if c["children_repeat"]);next(iter(c["children_repeat"].values()))["generals"]=[]
        with self.assertRaises(ValueError):validate(d)
if __name__=="__main__":unittest.main()
