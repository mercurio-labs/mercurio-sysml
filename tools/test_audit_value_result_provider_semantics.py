import copy
import unittest

from audit_value_result_provider_semantics import compare_queries, native_identity, validate_direction_controls


class ProviderSemanticComparisonTests(unittest.TestCase):
    def setUp(self):
        resource = "sysml.library/Kernel Libraries/Kernel Semantic Library/Base.kerml"
        fragment = "//@ownedRelationship.0/@ownedRelatedElement.0"
        self.identity = native_identity(resource, fragment)
        self.spec = {"inspection_queries": [{"owner_id": "owner", "field": "owned_feature"}]}
        self.independent = {"observations": [{
            "root": {"owner_id": "owner", "field": "owned_feature",
                     "feature_candidates": ["https://example.invalid#//Type/ownedFeature"]},
            "owner": {"qualified_name": "Base::Anything"},
            "feature_declaring_kind": "Type", "feature": "ownedFeature",
            "status": "reference_observed", "provider_completion_before": False,
            "provider_completion_after": False,
            "endpoints": [{"resource": resource, "emf_fragment": fragment, "kind": "Feature"}]}]}
        self.native = {"status": "read_only_diagnostic", "publication": "not_attempted",
            "semantic_qualification": "not_assessed", "qualification_certificate": False,
            "queries": [{"owner_id": "owner", "field": "owned_feature", "status": "query_evaluated",
                         "targets": [{"id": self.identity, "kind": "SysML::Feature"}]}]}

    def compare(self):
        return compare_queries(self.spec, self.independent, self.native)

    def test_exact_identity_type_and_order_are_required(self):
        self.assertEqual(self.compare()["counts"], {"exact_ordered_match": 1})
        self.native["queries"][0]["targets"][0]["kind"] = "SysML::Classifier"
        self.assertEqual(self.compare()["counts"], {"semantic_mismatch": 1})

    def test_reordered_endpoints_do_not_match(self):
        other = copy.deepcopy(self.independent["observations"][0]["endpoints"][0])
        other["emf_fragment"] = "//@ownedRelationship.1/@ownedRelatedElement.0"
        self.independent["observations"][0]["endpoints"].append(other)
        self.native["queries"][0]["targets"].insert(0, {"id": native_identity(other["resource"],other["emf_fragment"]), "kind": "Feature"})
        self.assertEqual(self.compare()["counts"], {"semantic_mismatch": 1})

    def test_prerequisites_and_unsupported_algorithms_get_no_match_credit(self):
        for status, key, expected in [("dependency_required","dependency","typed_dependency_required"),
                                     ("unavailable","error","native_algorithm_unavailable")]:
            self.native["queries"][0].update(status=status, **{key: "unresolved dependency"})
            self.assertEqual(self.compare()["counts"], {expected: 1})

    def test_pruned_matrices_or_wrong_request_order_reject(self):
        self.native["queries"] = []
        with self.assertRaisesRegex(ValueError, "denominator"):
            self.compare()
        self.native["queries"] = [{"owner_id": "different", "field": "owned_feature"}]
        with self.assertRaisesRegex(ValueError, "order or identity"):
            self.compare()

    def test_prepared_reference_and_qualification_claims_reject(self):
        self.independent["observations"][0]["provider_completion_after"] = True
        with self.assertRaisesRegex(ValueError, "prepared"):
            self.compare()
        self.independent["observations"][0]["provider_completion_after"] = False
        self.native["qualification_certificate"] = True
        with self.assertRaisesRegex(ValueError, "became qualification"):
            self.compare()

    def test_noncanonical_reference_identity_is_explicitly_unavailable(self):
        self.independent["observations"][0]["endpoints"][0]["emf_fragment"] = "//named"
        self.assertEqual(self.compare()["counts"], {"independent_identity_unavailable": 1})


class DirectionControlBoundaryTests(unittest.TestCase):
    def controls(self):
        rows = [{"owner": owner, "feature": feature, "value": None,
                 "supplied_materialized_operation_input": False, "completion_before": False,
                 "completion_after": False}
                for owner in ["G", "H", "C", "D"] for feature in ["i", "o", "b", "u"]]
        rows += [{"owner": owner, "feature": "i", "value": None,
                  "supplied_materialized_operation_input": True, "completion_before": True,
                  "completion_after": True} for owner in ["A", "B"]]
        return {"schema": "dev.mercurio.direction-operation-reference.v1",
            "qualification_certificate": False, "native_context_qualification": "not_assessed",
            "operation": "Type.directionOf(Feature)", "observations": rows}

    def test_materialized_cycle_controls_remain_component_evidence(self):
        data = self.controls()
        self.assertEqual(validate_direction_controls(data)["native_contexts_qualified"], 0)
        data["qualification_certificate"] = True
        with self.assertRaisesRegex(ValueError, "became qualification"):
            validate_direction_controls(data)

    def test_prepared_provider_or_relabelled_cycle_reject(self):
        for index, field in [(0, "completion_after"), (-1, "supplied_materialized_operation_input")]:
            data = self.controls()
            data["observations"][index][field] = not data["observations"][index][field]
            with self.assertRaisesRegex(ValueError, "lifecycle boundary"):
                validate_direction_controls(data)

    def test_missing_duplicate_and_non_enum_observations_reject(self):
        data = self.controls()
        data["observations"][-1] = copy.deepcopy(data["observations"][0])
        with self.assertRaisesRegex(ValueError, "denominator"):
            validate_direction_controls(data)
        data = self.controls()
        data["observations"][0]["value"] = "invalid"
        with self.assertRaisesRegex(ValueError, "enum literal"):
            validate_direction_controls(data)


if __name__ == "__main__":
    unittest.main()
