"""Isolated failure controls for native provider-port evidence; no qualification."""
import copy
import unittest
from audit_value_result_provider_plan import compare_ports, native_identity, read


class ProviderPortEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.spec = read("value-result-provider-plan-actual-spec.json")["cases"]
        cls.reference_ports = read("value-result-provider-plan-pilot-spec.json")["ports"]
        cls.observations = read("value-result-provider-plan-pilot-observations.json")["observations"]
        nodes = {"definition.resource." + path.encode("utf-8").hex():
                 {"id": "definition.resource." + path.encode("utf-8").hex()}
                 for path in cls.spec[0]["input_files"]}
        for observation in cls.observations:
            root = observation["root"]
            owner = nodes.setdefault(root["owner_id"], {
                "id": root["owner_id"], "kind": observation["owner"]["kind"],
                "properties": {"is_implied_included": False}})
            endpoint = native_identity(observation["endpoint"]["resource"], observation["endpoint"]["emf_fragment"])
            owner["properties"][root["field"]] = endpoint
            nodes[endpoint] = {"id": endpoint, "kind": observation["endpoint"]["kind"], "properties": {}}
        cls.row = {"inspection": {"constructed_elements": list(nodes.values()), "pending_references": []},
                   "element_count": len(nodes), "pending_reference_count": 0,
                   "committed_reference_fields": cls.spec[0]["dependency_roots"]}

    def fixture(self):
        return copy.deepcopy(self.spec), copy.deepcopy(self.observations), copy.deepcopy(self.row), copy.deepcopy(self.reference_ports)

    def test_complete_isolated_port_matrix_matches_without_context_credit(self):
        result = compare_ports(*self.fixture())
        self.assertEqual(len(result["ports"]), 9)
        self.assertTrue(all(not port["native_context_complete"] for port in result["ports"]))

    def test_resource_roots_and_fixed_denominators_cannot_shrink(self):
        for failure in ("resource", "port", "observation", "duplicate_resource"):
            spec, observations, row, ports = self.fixture()
            if failure == "resource":
                row["inspection"]["constructed_elements"].pop(0)
                row["element_count"] -= 1
            elif failure == "port":
                spec[0]["dependency_roots"].pop()
            elif failure == "observation":
                observations.pop()
            else:
                spec[0]["input_files"][-1] = spec[0]["input_files"][0]
            with self.subTest(failure=failure), self.assertRaises(ValueError):
                compare_ports(spec, observations, row, ports)

    def test_owner_endpoint_and_imported_feature_mismatches_reject(self):
        for failure in ("owner_kind", "endpoint_kind", "endpoint_identity", "feature", "duplicate_node"):
            spec, observations, row, ports = self.fixture()
            root = observations[0]["root"]
            nodes = row["inspection"]["constructed_elements"]
            owner = next(n for n in nodes if n["id"] == root["owner_id"])
            endpoint = next(n for n in nodes if n["id"] == owner["properties"][root["field"]])
            if failure == "owner_kind":
                owner["kind"] = "Package"
            elif failure == "endpoint_kind":
                endpoint["kind"] = "Package"
            elif failure == "endpoint_identity":
                owner["properties"][root["field"]] = "invented.endpoint"
            elif failure == "feature":
                observations[0]["root"]["feature_id"] = "invented.feature"
            else:
                nodes.append(copy.deepcopy(owner))
                row["element_count"] += 1
            with self.subTest(failure=failure), self.assertRaises(ValueError):
                compare_ports(spec, observations, row, ports)

    def test_prepared_provider_flags_reject_for_both_implementations(self):
        for failure in ("native", "reference_before", "reference_after"):
            spec, observations, row, ports = self.fixture()
            if failure == "native":
                owner = next(n for n in row["inspection"]["constructed_elements"]
                             if n["id"] == observations[0]["root"]["owner_id"])
                owner["properties"]["is_implied_included"] = True
            elif failure == "reference_before":
                observations[0]["provider_completion_before"] = True
            else:
                observations[0]["provider_completion_after"] = True
            with self.subTest(failure=failure), self.assertRaises(ValueError):
                compare_ports(spec, observations, row, ports)

    def test_pending_or_uncommitted_requested_ports_reject(self):
        for failure in ("uncommitted", "pending", "pending_count"):
            spec, observations, row, ports = self.fixture()
            if failure == "uncommitted":
                row["committed_reference_fields"].pop()
            else:
                row["inspection"]["pending_references"].append(copy.deepcopy(observations[0]["root"]))
                if failure == "pending":
                    row["pending_reference_count"] = 1
            with self.subTest(failure=failure), self.assertRaises(ValueError):
                compare_ports(spec, observations, row, ports)

    def test_noncanonical_or_nonownership_fragments_reject(self):
        for fragment in ("//@ownedRelationship.0/garbage", "//@ownedRelatedElement.00",
                         "//@type.0", "/@ownedRelationship.0"):
            # Decimal spelling must be canonical, including no leading zero.
            with self.subTest(fragment=fragment), self.assertRaises(ValueError):
                native_identity("sysml.library/Performances.kerml", fragment)


if __name__ == "__main__":
    unittest.main()
