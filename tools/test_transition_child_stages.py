import copy, hashlib, json, unittest
from run_transition_owner_recursive_probe import ROOT, OUTPUT, HELPER, validate
from generate_transition_child_stages import generate
class TransitionChildStagesTests(unittest.TestCase):
    def setUp(self):self.data=json.loads(OUTPUT.read_text(encoding="utf-8"))
    def test_cache_inventory_and_provenance(self):
        validate(self.data)
        self.assertEqual(sum(len(c["children_before"]) for c in self.data["controls"]),48)
        for key,path in [("helper_sha256",HELPER),("driver_sha256",ROOT/"tools/run_transition_owner_recursive_probe.py")]:self.assertEqual(self.data["provenance"][key],hashlib.sha256(path.read_bytes()).hexdigest())
        for n,h in self.data["provenance"]["observation_helpers_sha256"].items():self.assertEqual(h,hashlib.sha256(HELPER.with_name(n+".java").read_bytes()).hexdigest())
    def test_generated_identity_is_current(self):
        self.assertEqual(generate(self.data),(ROOT/"crates/mercurio-sysml/src/language_frontend/lowering/emit/transition_child_stages_generated.rs").read_text(encoding="utf-8"))
    def test_full_method_drift_rejected(self):
        d=copy.deepcopy(self.data);next(iter(d["methods"].values()))["kind"]="EMPTY_STATEMENT"
        with self.assertRaises(ValueError):generate(d)
    def test_missing_method_rejected(self):
        d=copy.deepcopy(self.data);d["methods"].pop(next(iter(d["methods"])))
        with self.assertRaises(ValueError):generate(d)
    def test_uncompleted_parent_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["parent_complete"]=False
        with self.assertRaises(ValueError):validate(d)
    def test_uncompleted_succession_role_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["succession_role_input"]["complete"]=False
        with self.assertRaises(ValueError):validate(d)
    def test_missing_child_rejected(self):
        d=copy.deepcopy(self.data);c=next(c for c in d["controls"] if c["children_after"]);c["children_after"].pop(next(iter(c["children_after"])))
        with self.assertRaises(ValueError):validate(d)
    def test_changed_repeat_rejected(self):
        d=copy.deepcopy(self.data);c=next(c for c in d["controls"] if c["children_repeat"]);next(iter(c["children_repeat"].values()))["generals"]=[]
        with self.assertRaises(ValueError):validate(d)
if __name__=="__main__":unittest.main()
