"""Fail-closed mutation controls over the independent frozen Pilot observations."""
import copy
import json
from pathlib import Path
import unittest
from export_pilot_type_set_contributions import validate, OUTPUT


class TypeSetReferenceControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.cache = json.loads(OUTPUT.read_text(encoding="utf-8"))

    def controls(self):
        return copy.deepcopy(self.cache["cases"]), copy.deepcopy(self.cache["boundaries"])

    def test_complete_independent_matrix(self):
        validate(self.cache["cases"], self.cache["boundaries"])

    def test_changed_default_is_not_silent(self):
        cases, boundaries = self.controls()
        cases[0]["observation"]["default_supertype"] = "Invalid::default"
        with self.assertRaises(ValueError):
            validate(cases, boundaries)

    def test_changed_types_are_not_silent(self):
        cases, boundaries = self.controls()
        cases[0]["observation"]["adapter_all_types"].append("constraintTarget")
        with self.assertRaises(ValueError):
            validate(cases, boundaries)

    def test_wrong_derived_source_rejected(self):
        cases, boundaries = self.controls()
        cases[0]["observation"]["source_is_receiver"] = False
        with self.assertRaises(ValueError):
            validate(cases, boundaries)

    def test_duplicate_context_cannot_replace_required_context(self):
        cases, boundaries = self.controls()
        cases[1] = copy.deepcopy(cases[0])
        with self.assertRaises(ValueError):
            validate(cases, boundaries)

    def test_wrong_kind_boundary_cannot_be_accepted(self):
        cases, boundaries = self.controls()
        next(row for row in boundaries if row["boundary"] == "wrong_kind")["observation"]["endpoint_assignment"] = "accepted"
        with self.assertRaises(ValueError):
            validate(cases, boundaries)

    def test_missing_endpoint_cannot_be_filled(self):
        cases, boundaries = self.controls()
        next(row for row in boundaries if row["boundary"] == "missing")["observation"]["endpoint_present"] = True
        with self.assertRaises(ValueError):
            validate(cases, boundaries)


if __name__ == "__main__":
    unittest.main()
