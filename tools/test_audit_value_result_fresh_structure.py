"""Adversarial controls for strict construction-batch evidence."""
import copy
import unittest
from audit_value_result_fresh_structure import read, compare_structural, compare_fresh_graph

class FreshStructureEvidenceControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.spec=read("value-result-fresh-structure-pilot-spec.json")
        cls.reference=read("value-result-fresh-structure-pilot-observations.json")
        cls.identities=read("value-result-lifecycle-shared-services-source-identities.json")
        cls.native=read("value-result-lifecycle-shared-services-native-queries.json")
        ports={(p["owner_id"],p["field"]) for p in cls.spec["ports"]}
        # Synthetic successful answers exercise the checker, not native behavior.
        # Rebuild from the preserved request identities without any acceptance credit.
        cls.native=read("value-result-lifecycle-shared-services-native-queries.json")
        for row in cls.native["queries"]:
            if (row["owner_id"],row["field"]) in ports:
                owner,field=row["owner_id"],row["field"]
                row.clear();row.update(owner_id=owner,field=field,status="query_evaluated",targets=[])

    def check(self,spec=None,reference=None,native=None):
        return compare_structural(spec or self.spec,reference or self.reference,native or self.native,self.identities)

    def test_exact_frozen_matrix_is_accepted_as_component(self):
        self.assertEqual(len(self.check()),275)

    def test_pruned_context_or_port_is_rejected(self):
        spec=copy.deepcopy(self.spec);spec["fixed_contexts"]=25
        with self.assertRaisesRegex(ValueError,"contexts"):self.check(spec=spec)
        spec=copy.deepcopy(self.spec);spec["ports"].pop()
        with self.assertRaisesRegex(ValueError,"275"):self.check(spec=spec)

    def test_prepared_or_side_effecting_reference_is_rejected(self):
        ref=copy.deepcopy(self.reference);ref["observations"][0]["provider_completion_after"]=True
        with self.assertRaisesRegex(ValueError,"Prepared"):self.check(reference=ref)
        ref=copy.deepcopy(self.reference);ref["observations"][0]["source_nodes_after"]+=1
        with self.assertRaisesRegex(ValueError,"Prepared"):self.check(reference=ref)

    def test_wrong_resolved_owner_or_contract_is_rejected(self):
        ref=copy.deepcopy(self.reference);ref["observations"][0]["owner"]["kind"]="Feature"
        with self.assertRaisesRegex(ValueError,"owner"):self.check(reference=ref)
        ref=copy.deepcopy(self.reference);ref["observations"][0]["derived"]=True
        with self.assertRaisesRegex(ValueError,"stored"):self.check(reference=ref)

    def test_native_pending_result_or_promotion_is_rejected(self):
        native=copy.deepcopy(self.native)
        port=self.spec["ports"][0]
        row=next(q for q in native["queries"] if q["owner_id"]==port["owner_id"] and q["field"]==port["field"])
        row["status"]="dependency_required"
        with self.assertRaisesRegex(ValueError,"requires"):self.check(native=native)
        native=copy.deepcopy(self.native);native["publication"]="closed"
        with self.assertRaisesRegex(ValueError,"promoted"):self.check(native=native)

    def test_explicit_edge_overwrite_or_fabricated_default_rejects(self):
        old=[{"id":"root","kind":"SysML::Package","layer":2,
            "properties":{"owned_relationship":["child"]}}]
        new=copy.deepcopy(old)
        new[0]["properties"]["owning_relationship"]=None
        self.assertEqual(compare_fresh_graph(old,new),{"owning_relationship":1})
        new[0]["properties"]["owned_relationship"]=[]
        with self.assertRaisesRegex(ValueError,"overwritten"):compare_fresh_graph(old,new)
        new=copy.deepcopy(old);new[0]["properties"]["owning_relationship"]="invented"
        with self.assertRaisesRegex(ValueError,"fabricated"):compare_fresh_graph(old,new)

    def test_missing_or_prepared_native_objects_reject(self):
        old=[{"id":"root","kind":"SysML::Package","layer":2,"properties":{"owned_relationship":[]}}]
        with self.assertRaisesRegex(ValueError,"identities"):compare_fresh_graph(old,[])
        new=copy.deepcopy(old);new[0]["properties"]["owning_relationship"]=None
        new[0]["properties"]["is_implied_included"]=True
        with self.assertRaises(ValueError):compare_fresh_graph(old,new)

if __name__=="__main__":unittest.main()
