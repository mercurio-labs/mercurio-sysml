"""Integrity checks for independent type-set predicate/projection observations."""
import copy,json,unittest
from export_pilot_type_set_validation import OUTPUT,validate
class TypeSetValidationControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.cache=json.loads(OUTPUT.read_text(encoding="utf-8"))
    def check_changed(self,change):
        d=copy.deepcopy(self.cache);change(d)
        with self.assertRaises(ValueError):validate(d["cases"],d["boundaries"])
    def test_complete(self):validate(self.cache["cases"],self.cache["boundaries"])
    def test_missing_constraint(self):
        self.check_changed(lambda d:next(c for c in d["cases"] if c["count"]==1)["diagnostics"].clear())
    def test_duplicate_context(self):self.check_changed(lambda d:d["cases"].__setitem__(1,copy.deepcopy(d["cases"][0])))
    def test_nonunique_endpoint(self):
        self.check_changed(lambda d:next(c for c in d["cases"] if c["count"]==2 and c["pattern"]=="duplicate")["endpoints"].append("target0"))
    def test_foreign_source_selected(self):
        self.check_changed(lambda d:next(c for c in d["boundaries"] if c["mode"]=="foreign_source").__setitem__("owned_count",1))
    def test_wrong_owned_order(self):
        self.check_changed(lambda d:next(c for c in d["cases"] if c["count"]==2)["owned"].reverse())
if __name__=="__main__":unittest.main()
