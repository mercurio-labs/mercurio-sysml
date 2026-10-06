"""Failure controls for the V01 dependency/observation audit; no qualification."""
import copy, unittest
from value_result_bundle import BASE, META, build, load, validate_reference, getter_candidates, validator_method_closure

class ValueResultBundleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.bundle = load(BASE / "value-result-obligation-bundle.json")
        cls.reference = load(BASE / "value-result-reference-controls.json.gz")
        cls.artifacts = [load(META / name) for name in ("ecore-effective.extract.json", "ecore-semantics.extract.json",
            "validators.inventory.extract.json", "validation-bindings.json", "validation-rules.extract.json")]

    def changed(self):
        return copy.deepcopy(self.bundle), copy.deepcopy(self.reference)

    def test_complete_observation_matrix_is_reference_only(self):
        result = validate_reference(self.bundle, self.reference)
        self.assertEqual(result, {"contexts_observed":26,"direct_result_queries":62,"canonical_stored_witnesses":580,
            "library_stored_witnesses":161,"native_contexts_verified":0})

    def test_reference_cannot_award_support_or_drop_verification_stages(self):
        for failure in ("reference_support", "bundle_certificate", "missing_stage"):
            bundle, reference = self.changed()
            if failure == "reference_support": reference["native_qualification"] = "qualified"
            elif failure == "bundle_certificate": bundle["qualification_certificate"] = True
            else: bundle["contexts"][0]["required_stages"].remove("persist")
            with self.assertRaises(ValueError): validate_reference(bundle, reference)

    def test_missing_duplicate_and_changed_outcome_contexts_reject(self):
        for failure in ("missing", "duplicate", "outcome"):
            bundle, reference = self.changed()
            if failure == "missing": reference["controls"].pop()
            elif failure == "duplicate": reference["controls"][-1] = reference["controls"][0]
            else: reference["controls"][0]["validation_status"] = "rejected"
            with self.assertRaises(ValueError): validate_reference(bundle, reference)

    def test_missing_target_membership_and_owner_witnesses_reject(self):
        for level in ("target", "membership", "owner"):
            bundle, reference = self.changed()
            observation = reference["controls"][0]["result_observations"]
            records = {w["id"]:w for w in observation["stored_witnesses"]}
            target = observation["queries"][0]["result"]["id"]
            membership = records[target]["reference_sequences"]["owningRelationship"][0]["id"]
            owner = records[membership]["reference_sequences"]["owningRelatedElement"][0]["id"]
            missing = {"target":target, "membership":membership, "owner":owner}[level]
            observation["stored_witnesses"] = [w for w in observation["stored_witnesses"] if w["id"] != missing]
            with self.assertRaises(ValueError): validate_reference(bundle, reference)

    def test_nonreturn_and_nonreciprocal_result_ownership_reject(self):
        for failure in ("kind", "member", "owner"):
            bundle, reference = self.changed()
            observation = reference["controls"][0]["result_observations"]
            records = {w["id"]:w for w in observation["stored_witnesses"]}
            target = records[observation["queries"][0]["result"]["id"]]
            member = records[target["reference_sequences"]["owningRelationship"][0]["id"]]
            if failure == "kind": member["kind"] = "OwningMembership"
            elif failure == "member": member["reference_sequences"]["ownedRelatedElement"] = []
            else:
                owner = records[member["reference_sequences"]["owningRelatedElement"][0]["id"]]
                owner["reference_sequences"]["ownedRelationship"] = []
            with self.assertRaises(ValueError): validate_reference(bundle, reference)

    def test_original_owned_inherited_result_boundaries_cannot_shrink(self):
        for case in ("valuation-source-00", "valuation-source-09"):
            bundle, reference = self.changed()
            row = next(r for r in reference["controls"] if r["id"] == case)
            function = next(q for q in row["result_observations"]["queries"] if q["kind"] == "Function")
            if case.endswith("00"): function["owned_result"] = None
            else: function["owned_return_memberships"] = [function["result"]]
            with self.assertRaises(ValueError): validate_reference(bundle, reference)

    def test_rejected_context_cannot_publish_result_observations(self):
        bundle, reference = self.changed()
        row = next(r for r in reference["controls"] if r["validation_status"] == "rejected")
        row["result_observations"] = {"queries":[{}]}
        with self.assertRaises(ValueError): validate_reference(bundle, reference)

    def test_imported_and_bound_operations_do_not_qualify_native_behavior(self):
        doc = build(self.bundle, self.reference, *self.artifacts)
        self.assertFalse(doc["qualification_certificate"])
        self.assertEqual(doc["metrics"]["native_contexts_verified"], 0)
        for key in ("ecore_features", "ecore_operations", "delegate_bindings",
                    "applicable_check_candidates", "resolved_semantic_dependencies"):
            self.assertTrue(doc[key])
            self.assertTrue(all(row["native_bundle_status"] == "not_verified" for row in doc[key]))
        self.assertTrue(all(not row["native_complete"] for row in doc["context_stage_matrix"]))

    def test_boolean_accessors_and_contract_descriptors_are_classified_faithfully(self):
        effective = self.artifacts[0]
        classes = {c["name"]:c for c in effective["classes"]}
        feature = next(f for f in effective["features"] if f["name"] == "isEnd")
        self.assertIn(feature["id"], getter_candidates("isEnd", "org.omg.sysml.lang.sysml.Feature",
            effective["features"], classes))
        self.assertIn(feature["id"], getter_candidates("isEnd", "org.omg.sysml.lang.sysml.Expression",
            effective["features"], classes))
        self.assertEqual(getter_candidates("isEnd", "java.lang.Feature", effective["features"], classes), [])
        self.assertNotIn(feature["id"], getter_candidates("isEnd", "org.omg.sysml.lang.sysml.Element",
            effective["features"], classes))
        doc = build(self.bundle, self.reference, *self.artifacts)
        rows = {r["symbol"]:r for r in doc["resolved_semantic_dependencies"]}
        self.assertEqual(rows["org.omg.sysml.lang.sysml.Feature.isEnd()"]["algorithm_category"], "ecore_stored_access")
        self.assertEqual(rows["org.omg.sysml.lang.sysml.SysMLPackage.getFeature_Type()"]["algorithm_category"], "ecore_contract_descriptor")
        self.assertEqual(rows["org.omg.sysml.lang.sysml.FeatureDirectionKind.toString()"]["algorithm_category"], "enum_scalar_dependency")
        self.assertEqual(doc["metrics"]["native_contexts_verified"], 0)


    def test_validator_closure_uses_resolved_jvm_overloads_and_handles_recursion(self):
        def method(identity, symbol, calls):
            return {"id":identity,"jvm_ids":[symbol],"resolution_status":"resolved",
                    "resolution_issues":[],"calls":[{"resolved":True,"target":{"kind":"JvmOperation","id":c}} for c in calls]}
        inventory = {"methods":[method("root", "check()", ["helper(java.lang.String)"]),
                                method("right", "helper(java.lang.String)", ["check()", "native(java.lang.String)"]),
                                method("wrong", "helper(int)", ["wrongNative()"])]}
        closure = validator_method_closure("root", inventory)
        self.assertEqual(closure["methods"], ["right", "root"])
        self.assertEqual(closure["external_symbols"], ["native(java.lang.String)"])
        self.assertFalse(closure["native_verified"])
        duplicate = copy.deepcopy(inventory)
        duplicate["methods"][2]["jvm_ids"] = ["helper(java.lang.String)"]
        with self.assertRaises(ValueError): validator_method_closure("root", duplicate)
        with self.assertRaises(ValueError): validator_method_closure("missing", inventory)
        inventory["methods"][1]["resolution_issues"] = ["unresolved type"]
        self.assertTrue(validator_method_closure("root", inventory)["resolution_issues"])

    def test_source_checks_and_provider_closure_remain_separate_unqualified_candidates(self):
        doc = build(self.bundle, self.reference, *self.artifacts)
        checks = doc["applicable_check_candidates"]
        self.assertEqual(doc["validator_closure_metrics"]["check_roots"], len(checks))
        self.assertEqual(doc["validator_closure_metrics"]["native_checks_verified"], 0)
        self.assertTrue(all(not row["imported_validator_method_closure"]["native_verified"] for row in checks))
        self.assertTrue(any(row["provider_context_candidates"] and not row["source_context_candidates"] for row in checks))
        helpers = {identity for row in checks for identity in row["imported_validator_method_closure"]["methods"]}
        roots = {row["id"] for row in checks}
        self.assertGreater(len(helpers), len(roots))
        self.assertTrue(any("conformsTo(" in identity for identity in helpers - roots))
        self.assertTrue(all(row["imported_validator_method_closure"]["boundary"] for row in checks))
        self.assertEqual(len(doc["validator_applicability_boundary"]["unknown_source_contexts"]), 7)
        self.assertTrue(all(row["status"] == "source_check_applicability_not_assessed"
                            for row in doc["validator_applicability_boundary"]["unknown_source_contexts"]))

    def test_unknown_metaclasses_and_cyclic_imported_ancestry_reject(self):
        bundle, reference = self.changed()
        reference["controls"][0]["model"]["elements"][0]["kind"] = "InventedModelKind"
        with self.assertRaises(ValueError): build(bundle, reference, *self.artifacts)
        effective = copy.deepcopy(self.artifacts[0])
        feature = next(c for c in effective["classes"] if c["name"] == "Feature")
        feature["super_types"].append(feature["id"])
        with self.assertRaises(ValueError): build(self.bundle, self.reference, effective, *self.artifacts[1:])

if __name__ == "__main__":
    unittest.main()
