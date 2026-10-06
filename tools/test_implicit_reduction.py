import json
import unittest
from export_pilot_implicit_reduction import OUTPUT,CONSTRUCTION,render
class ImplicitReductionTests(unittest.TestCase):
    def setUp(self):self.doc=json.loads(OUTPUT.read_text())
    def test_pinned_reduction_and_order(self):self.assertIn("Redefinition",render(self.doc))
    def test_missing_case_rejected(self):
        self.doc["cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete reduction"):render(self.doc)
    def test_changed_semantics_rejected(self):
        self.doc["definitions"].pop("TypeAdapter#specializesExcludingTarget/2")
        with self.assertRaisesRegex(ValueError,"Changed resolved"):render(self.doc)
    def test_unknown_category_rejected(self):
        self.doc["kinds"]["Conjugation"]=999
        with self.assertRaisesRegex(ValueError,"Unassessed reduction"):render(self.doc)
    def test_closed_queue_required(self):
        self.doc["cases"][0]["after_closed"]=[["Subsetting","c"]]
        with self.assertRaisesRegex(ValueError,"remains open"):render(self.doc)
    def test_lossy_pilot_cycle_is_recorded(self):
        cycle=next(r for r in self.doc["cases"] if r["shape"]=="general_cycle")
        self.assertEqual(len(cycle["pending"]),2)
        self.assertEqual(cycle["remaining"],[])
    def test_duplicate_kind_rank_rejected(self):
        self.doc["kinds"]["Subsetting"]=self.doc["kinds"]["Redefinition"]
        with self.assertRaisesRegex(ValueError,"Invalid reduction kind ordering"):render(self.doc)

    def test_all_concrete_construction_categories_have_reduction_controls(self):
        kinds=set(json.loads(CONSTRUCTION.read_text(encoding="utf-8"))["kinds"])
        self.assertEqual(set(self.doc["kinds"]),kinds)
        observed={item[0] for case in self.doc["cases"] for item in case["pending"]}
        self.assertEqual(observed,kinds)
