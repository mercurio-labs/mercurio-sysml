import copy
import unittest
import audit_value_result_shared_services as audit

class SharedServiceEvidenceControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        p=audit.PREFIX
        cls.spec=audit.read(p+"-pilot-spec.json")
        cls.reference=audit.read(p+"-pilot-observations.json")
        cls.native=audit.read(p+"-native-queries.json")
        cls.identities=audit.read(p+"-source-identities.json")
    def execute(self,spec=None,reference=None,native=None,identities=None):
        return audit.compare(spec or self.spec,reference or self.reference,native or self.native,identities or self.identities)
    def test_fixed_complete_read_denominator_does_not_award_lifecycle_credit(self):
        result=self.execute()
        self.assertEqual(result["counts"],{"dependency_required":51,"exact_ordered_match":1007,
            "reference_endpoint_unrepresented":5,"reference_getter_failed":2})
        self.assertEqual(result["reference_initial_source_nodes"],388)
        self.assertEqual(result["reference_final_source_nodes"],398)
        self.assertEqual(result["reference_construction_queries"],5)
        self.assertTrue(all(not row["complete_native_context"] for row in result["ports"]))
    def test_pruned_reference_denominator_rejected(self):
        spec=copy.deepcopy(self.spec);spec["ports"].pop()
        with self.assertRaises(ValueError):self.execute(spec=spec)
    def test_duplicate_reference_port_rejected(self):
        spec=copy.deepcopy(self.spec);spec["ports"][0]=spec["ports"][1]
        with self.assertRaises(ValueError):self.execute(spec=spec)
    def test_prepared_pilot_receiver_rejected(self):
        reference=copy.deepcopy(self.reference);reference["observations"][0]["provider_completion_before"]=True
        with self.assertRaises(ValueError):self.execute(reference=reference)
    def test_reference_reordering_rejected(self):
        reference=copy.deepcopy(self.reference)
        reference["observations"][0],reference["observations"][1]=reference["observations"][1],reference["observations"][0]
        with self.assertRaises(ValueError):self.execute(reference=reference)
    def test_model_promotion_rejected(self):
        native=copy.deepcopy(self.native);native["publication"]="closed"
        with self.assertRaises(ValueError):self.execute(native=native)
    def test_pruned_native_matrix_rejected(self):
        native=copy.deepcopy(self.native);native["queries"].pop()
        with self.assertRaises(ValueError):self.execute(native=native)
    def test_pruned_canonical_source_map_rejected(self):
        identities=copy.deepcopy(self.identities);identities["canonical_resource_fragment_to_native_id"].popitem()
        with self.assertRaises(ValueError):self.execute(identities=identities)
    def test_missing_reference_construction_state_rejected(self):
        reference=copy.deepcopy(self.reference);reference["observations"][0].pop("source_nodes_after")
        with self.assertRaises((KeyError,ValueError)):self.execute(reference=reference)

if __name__=="__main__":unittest.main()
