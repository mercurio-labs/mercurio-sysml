import copy
import unittest
import audit_value_result_ownership as audit

class OwnershipEvidenceControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.spec=audit.read("value-result-ownership-reference-spec.json")
        cls.reference=audit.read("value-result-ownership-reference-observations.json")
        cls.native=audit.read("value-result-cached-dependency-post-construction-queries.json")
        cls.identities=audit.read("value-result-lifecycle-shared-services-source-identities.json")
        cls.requests=audit.read("value-result-lifecycle-reference-read-spec.json")
    def execute(self,spec=None,reference=None,native=None):
        return audit.compare(spec or self.spec,reference or self.reference,native or self.native,
            self.identities,self.requests)
    def test_baseline_dependencies_and_getter_constructions_remain_explicit(self):
        result=self.execute()
        self.assertEqual(result["counts"],{"unavailable":24,"dependency_required":10})
        self.assertEqual(result["reference_source_nodes_added"],14)
        self.assertEqual(result["complete_native_contexts"],0)
        self.assertEqual(result["strict_families_qualified"],0)
    def test_pruned_ports_rejected(self):
        spec=copy.deepcopy(self.spec);spec["ports"].pop()
        with self.assertRaises(ValueError):self.execute(spec=spec)
    def test_reordered_reference_rejected(self):
        reference=copy.deepcopy(self.reference)
        reference["observations"].reverse()
        with self.assertRaises(ValueError):self.execute(reference=reference)
    def test_prepared_reference_rejected(self):
        reference=copy.deepcopy(self.reference)
        reference["observations"][0]["provider_completion_after"]=True
        with self.assertRaises(ValueError):self.execute(reference=reference)
    def test_pruned_native_matrix_rejected(self):
        native=copy.deepcopy(self.native);native["queries"].pop()
        with self.assertRaises(ValueError):self.execute(native=native)
    def test_publication_promotion_rejected(self):
        native=copy.deepcopy(self.native);native["publication"]="closed"
        with self.assertRaises(ValueError):self.execute(native=native)
    def test_invented_empty_getter_endpoint_is_a_mismatch(self):
        native=copy.deepcopy(self.native)
        observation=next(o for o in self.reference["observations"] if
            o["root"]["strategy_group"]=="multiplicity_range_dispatch_and_featuring"
            and o["root"]["field"]=="featuring_type")
        root=observation["root"]
        answer=next(q for q in native["queries"] if
            (q["owner_id"],q["field"])==(root["owner_id"],root["field"]))
        answer.update(status="query_evaluated",targets=[{"id":"invented","kind":"SysML::Class"}])
        self.assertEqual(self.execute(native=native)["counts"]["semantic_mismatch"],1)
if __name__=="__main__":unittest.main()
