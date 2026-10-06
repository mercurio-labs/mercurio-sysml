import copy
import hashlib
import json
import unittest
from run_accept_trigger_probe import ROOT, OUTPUT, HELPER, validate_controls

class AcceptTriggerCacheTests(unittest.TestCase):
    def setUp(self): self.data=json.loads(OUTPUT.read_text(encoding="utf-8"))
    def test_cached_matrix_and_provenance(self):
        validate_controls(self.data)
        for key,p in [("helper_sha256",HELPER),("driver_sha256",ROOT/"tools/run_accept_trigger_probe.py")]:
            self.assertEqual(self.data["provenance"][key],hashlib.sha256(p.read_bytes()).hexdigest())
    def test_missing_context_rejected(self):
        d=copy.deepcopy(self.data);d["controls"].pop()
        with self.assertRaises(ValueError): validate_controls(d)
    def test_default_leak_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["library_lookups"]=["Actions::acceptActions"]
        with self.assertRaises(ValueError): validate_controls(d)
    def test_dropped_general_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][-1]["general_types"]=[]
        with self.assertRaises(ValueError): validate_controls(d)
    def test_fabricated_completion_rejected(self):
        d=copy.deepcopy(self.data);d["controls"][0]["is_implied_included"]=True
        with self.assertRaises(ValueError): validate_controls(d)

if __name__=="__main__": unittest.main()
