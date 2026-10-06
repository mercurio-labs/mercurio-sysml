import json
import unittest

from audit_library_ecore_observations import ECORE, PIN, audit


class LibraryObservationAuditTests(unittest.TestCase):
    def test_observation_survives_native_import_and_mismatch_fails(self):
        ecore = json.loads(ECORE.read_text(encoding="utf-8"))
        source = {"metadata": {"observed_ecore_defaults_v1": True, "element_count": 1,
                               "relationship_count": 0}, "relationships": [],
                  "elements": [{"qualified_name": "P::e", "kind": "Element",
                                "properties": {"is_implied_included": True}}]}
        kir = {"metadata": {"pilot_commit": PIN}, "elements": [
            {"id": "P::e", "kind": "SysML::Element", "properties": {
                "metadata": {"is_implied_included": True}, "is_implied_included": True}}]}
        result = audit(ecore, source, kir)
        self.assertEqual(result["observations"], 1)
        self.assertEqual(result["different_from_ecore_literal"], {"is_implied_included": 1})
        kir["elements"][0]["properties"]["is_implied_included"] = False
        with self.assertRaisesRegex(ValueError, "differs from Pilot"):
            audit(ecore, source, kir)

    def test_unmarked_or_truncated_export_fails_closed(self):
        ecore = json.loads(ECORE.read_text(encoding="utf-8"))
        source = {"metadata": {"element_count": 0, "relationship_count": 0},
                  "elements": [], "relationships": []}
        kir = {"metadata": {"pilot_commit": PIN}, "elements": []}
        with self.assertRaisesRegex(ValueError, "marked Ecore getter"):
            audit(ecore, source, kir)
        source["metadata"]["observed_ecore_defaults_v1"] = True
        source["metadata"]["element_count"] = 1
        with self.assertRaisesRegex(ValueError, "element count"):
            audit(ecore, source, kir)


if __name__ == "__main__":
    unittest.main()
