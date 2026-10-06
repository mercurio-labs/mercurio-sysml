import copy
import json
import unittest

from generate_ecore_model import INPUT, SEMANTICS, SYSML, generate


class EcoreModelGenerationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = json.loads(INPUT.read_text(encoding="utf-8"))
        cls.semantics = json.loads(SEMANTICS.read_text(encoding="utf-8"))

    def test_pinned_table_includes_every_resolved_feature(self):
        output = generate(self.doc, self.semantics).decode()
        self.assertEqual(output.count("    FeatureContract {"), len(self.doc["features"]))
        self.assertIn('owner: "Element", field: "owned_relationship"', output)
        self.assertIn('owner: "MultiplicityRange", field: "bound"', output)

    def test_library_namespace_operation_preserves_resolved_dynamic_contract(self):
        output = generate(self.doc, self.semantics).decode()
        self.assertIn('owner: "Element", target: "Namespace"', output)
        self.assertIn('("LibraryPackage", "org.omg.sysml.delegate.invocation.LibraryPackage_libraryNamespace_InvocationDelegate")', output)
        for change in ({"parameters": [{"name": "newArgument"}]}, {"type": SYSML + "Feature"}, {"ordered": True}):
            doc = copy.deepcopy(self.doc)
            next(o for o in doc["operations"] if o["name"] == "libraryNamespace").update(change)
            with self.assertRaisesRegex(ValueError, "Changed libraryNamespace signature"):
                generate(doc, self.semantics)
        semantics = copy.deepcopy(self.semantics)
        binding = next(b for b in semantics["delegate_bindings"] if b["element"].endswith("operation:libraryNamespace()"))
        binding["candidate_source_classes"].append("org.omg.sysml.delegate.invocation.Missing_libraryNamespace_InvocationDelegate")
        with self.assertRaisesRegex(ValueError, "Unresolved libraryNamespace invocation branch"):
            generate(self.doc, semantics)

    def test_evaluability_operation_preserves_signature_and_dynamic_branches(self):
        output = generate(self.doc, self.semantics).decode()
        self.assertIn('name: "modelLevelEvaluable", owner: "Expression", target: "Ecore::EBoolean"', output)
        self.assertIn('name: "visited", target: "Feature", lower: 0, upper: -1', output)
        bindings = next(b for b in self.semantics["delegate_bindings"] if "#//Expression/operation:modelLevelEvaluable(" in b["element"])
        for candidate in bindings["candidate_source_classes"]:
            self.assertIn(candidate, output)
        for change in ({"parameters": []}, {"lower_bound": 0}, {"type": SYSML + "Feature"}):
            doc = copy.deepcopy(self.doc)
            next(o for o in doc["operations"] if o["name"] == "modelLevelEvaluable").update(change)
            with self.assertRaisesRegex(ValueError, "Changed modelLevelEvaluable signature"):
                generate(doc, self.semantics)

    def test_evaluability_unresolved_or_changed_selector_rejects(self):
        for key, value, message in [
            ("dispatch_source_class", "changed.Selector", "Changed modelLevelEvaluable invocation selector"),
            ("candidate_source_classes", ["org.omg.sysml.delegate.invocation.Missing_modelLevelEvaluable_InvocationDelegate"], "Unresolved modelLevelEvaluable invocation branch"),
        ]:
            semantics = copy.deepcopy(self.semantics)
            next(b for b in semantics["delegate_bindings"] if "#//Expression/operation:modelLevelEvaluable(" in b["element"])[key] = value
            with self.assertRaisesRegex(ValueError, message):
                generate(self.doc, semantics)

    def test_storage_flags_preserve_effective_ecore_and_changes(self):
        output = generate(self.doc, self.semantics).decode()
        for feature in self.doc["features"]:
            start = output.index('id: ' + json.dumps(feature["id"]))
            contract = output[start:output.index("    },", start)]
            for flag in ("changeable", "unsettable"):
                self.assertIn(f'{flag}: {str(feature[flag]).lower()}', contract)
            proxies = "None" if feature["resolve_proxies"] is None else f'Some({str(feature["resolve_proxies"]).lower()})'
            self.assertIn(f'resolve_proxies: {proxies}', contract)
        doc = copy.deepcopy(self.doc)
        feature = next(f for f in doc["features"] if not f["derived"] and f["kind"] == "reference")
        feature.update(changeable=False, unsettable=True, resolve_proxies=False)
        output = generate(doc, self.semantics).decode()
        start = output.index('id: ' + json.dumps(feature["id"]))
        contract = output[start:output.index("    },", start)]
        self.assertIn('changeable: false, unsettable: true, resolve_proxies: Some(false)', contract)

    def test_setting_delegate_evidence_is_preserved_without_execution_credit(self):
        output = generate(self.doc, self.semantics).decode()
        count = sum(b["kind"] == "setting" for b in self.semantics["delegate_bindings"])
        self.assertEqual(output.count("setting_delegate: Some("), count)
        self.assertIn("InvocationExpression_operand_SettingDelegate", output)
        semantics = copy.deepcopy(self.semantics)
        binding = next(b for b in semantics["delegate_bindings"] if b["element"].endswith("/InvocationExpression/operand"))
        binding["candidate_source_classes"] = ["changed.Dispatch"]
        self.assertIn('"changed.Dispatch"', generate(self.doc, semantics).decode())
        semantics["delegate_bindings"].append(copy.deepcopy(binding))
        with self.assertRaisesRegex(ValueError, "Unknown or duplicate setting delegate"):
            generate(self.doc, semantics)

    def test_unresolved_reference_and_invalid_bounds_fail_closed(self):
        doc = copy.deepcopy(self.doc)
        reference = next(row for row in doc["features"] if row["kind"] == "reference")
        reference["type"] = SYSML + "MissingClass"
        with self.assertRaisesRegex(ValueError, "Unresolved reference type"):
            generate(doc, self.semantics)
        reference["type"] = self.doc["features"][0]["type"]
        reference["lower_bound"] = 3
        reference["upper_bound"] = 2
        with self.assertRaisesRegex(ValueError, "Invalid Ecore bounds"):
            generate(doc, self.semantics)

    def test_redefinitions_resolve_chains_and_preserve_ambiguity(self):
        output = generate(self.doc, self.semantics).decode()
        self.assertIn('("Subclassification", "general", "superclassifier")', output)
        self.assertIn('("Redefinition", "general", "redefined_feature")', output)
        self.assertIn('("AllocationUsage", "type")', output)
        semantics = copy.deepcopy(self.semantics)
        feature = next(e for e in semantics["elements"] if e["id"] == SYSML + "Subclassification/superclassifier")
        annotation = next(a for a in feature["annotations"] if a["attributes"].get("source") == "redefines")
        annotation["attributes"]["references"] = "#//Missing/feature"
        with self.assertRaisesRegex(ValueError, "Unresolved redefined feature"):
            generate(self.doc, semantics)
        annotation["attributes"]["references"] = "#//FeatureTyping/type"
        with self.assertRaisesRegex(ValueError, "Invalid redefinition ancestry"):
            generate(self.doc, semantics)
        annotation["attributes"]["references"] = "#//Subclassification/superclassifier"
        with self.assertRaisesRegex(ValueError, "Invalid redefinition ancestry"):
            generate(self.doc, semantics)

    def test_subsets_resolve_transitive_contracts_and_reject_bad_edges(self):
        output = generate(self.doc, self.semantics).decode()
        self.assertIn('("Relationship", "source", "related_element")', output)
        self.assertIn('("OwningMembership", "owned_member_element", "related_element")', output)
        for ref, message in [("#//Missing/feature", "Unresolved subset"),
                             ("#//Relationship/source", "Cyclic property subset"),
                             ("#//Feature/type", "Invalid subset property ancestry")]:
            semantics = copy.deepcopy(self.semantics)
            feature = next(e for e in semantics["elements"] if e["id"] == SYSML + "Relationship/source")
            annotation = next(a for a in feature["annotations"] if a["attributes"].get("source") == "subsets")
            annotation["attributes"]["references"] = ref
            with self.assertRaisesRegex(ValueError, message): generate(self.doc, semantics)

    def test_union_inputs_include_subtype_declared_subsets(self):
        output = generate(self.doc, self.semantics).decode()
        self.assertIn('("Namespace", "membership", &[("Namespace", "imported_membership"), ("Namespace", "owned_membership"), ("Type", "inherited_membership")])', output)
        doc = copy.deepcopy(self.doc)
        feature = next(f for f in doc["features"] if f["id"] == SYSML + "Namespace/membership")
        feature["derived"] = False
        with self.assertRaisesRegex(ValueError, "Union annotation requires a derived"):
            generate(doc, self.semantics)

    def test_enum_values_come_from_ecore_and_unknown_types_fail(self):
        semantics = copy.deepcopy(self.semantics)
        literal = next(r for r in semantics["elements"] if r["kind"] == "EEnumLiteral")
        literal["attributes"]["literal"] = "changed_literal"
        self.assertIn('"changed_literal"', generate(self.doc, semantics).decode())
        doc = copy.deepcopy(self.doc)
        attribute = next(r for r in doc["features"] if r["kind"] == "attribute")
        attribute["type"] = SYSML + "UnknownDataType"
        with self.assertRaisesRegex(ValueError, "Unsupported attribute type"):
            generate(doc, self.semantics)

    def test_enum_default_and_literal_owner_fail_closed(self):
        doc = copy.deepcopy(self.doc)
        attribute = next(r for r in doc["features"] if r["type"] == SYSML + "VisibilityKind")
        attribute["default_literal"] = "not_a_visibility"
        with self.assertRaisesRegex(ValueError, "Invalid enum default"):
            generate(doc, self.semantics)
        semantics = copy.deepcopy(self.semantics)
        literal = next(r for r in semantics["elements"] if r["kind"] == "EEnumLiteral")
        literal["owner"] = SYSML + "MissingEnum"
        with self.assertRaisesRegex(ValueError, "Unresolved enum literal owner"):
            generate(self.doc, semantics)


    def test_direct_subset_sources_preserve_annotation_order_for_all_features(self):
        output = generate(self.doc, self.semantics).decode()
        expected = {}
        for element in self.semantics["elements"]:
            for annotation in element.get("annotations", []):
                attributes = annotation.get("attributes", {})
                if attributes.get("source") == "subsets":
                    expected.setdefault(element["id"], []).extend(
                        SYSML + ref[3:] if ref.startswith("#//") else ref
                        for ref in attributes["references"].split())
        for feature in self.doc["features"]:
            start = output.index("id: " + json.dumps(feature["id"]))
            contract = output[start:output.index("    },", start)]
            values = ", ".join(json.dumps(v) for v in expected.get(feature["id"], []))
            self.assertIn("subset_sources: &[" + values + "]", contract)

    def test_direct_subset_order_is_distinct_from_transitive_membership(self):
        semantics = copy.deepcopy(self.semantics)
        annotation = next(
            annotation for element in semantics["elements"]
            for annotation in element.get("annotations", [])
            if annotation.get("attributes", {}).get("source") == "subsets"
            and len(annotation["attributes"].get("references", "").split()) > 1)
        references = annotation["attributes"]["references"].split()
        before = generate(self.doc, semantics).decode()
        annotation["attributes"]["references"] = " ".join(reversed(references))
        after = generate(self.doc, semantics).decode()
        self.assertNotEqual(before, after)
        # Transitive closure is unchanged; only ordered direct-source data changes.
        self.assertEqual(
            before[before.index("pub(super) const SUBSET_FIELDS"):],
            after[after.index("pub(super) const SUBSET_FIELDS"):])

    def test_default_delegate_class_is_preserved_as_evidence(self):
        output = generate(self.doc, self.semantics).decode()
        bindings = [binding for binding in self.semantics["delegate_bindings"]
                    if binding["kind"] == "setting"
                    and binding["binding_status"] == "default_setting_delegate_fallback_source"]
        self.assertEqual(len(bindings), 78)
        for binding in bindings:
            start = output.index("id: " + json.dumps(binding["element"]))
            contract = output[start:output.index("    },", start)]
            self.assertIn("fallback: Some(" + json.dumps(binding["fallback_class"]) + ")", contract)
        semantics = copy.deepcopy(self.semantics)
        changed = next(b for b in semantics["delegate_bindings"] if b["element"] == bindings[0]["element"])
        changed["fallback_class"] = "changed.UnreviewedFallback"
        self.assertIn('fallback: Some("changed.UnreviewedFallback")', generate(self.doc, semantics).decode())

    def test_direction_operation_preserves_resolved_signature_and_branch(self):
        output = generate(self.doc, self.semantics).decode()
        self.assertIn('name: "directionOf", owner: "Type", target: "FeatureDirectionKind", lower: 0, upper: 1', output)
        self.assertIn('name: "feature", target: "Feature", lower: 1, upper: 1', output)
        self.assertIn('("Type", "org.omg.sysml.delegate.invocation.Type_directionOf_InvocationDelegate")', output)
        for change in ({"parameters": []}, {"upper_bound": -1}, {"type": SYSML + "Feature"}, {"ordered": True}):
            doc = copy.deepcopy(self.doc)
            next(o for o in doc["operations"] if o["name"] == "directionOf").update(change)
            with self.assertRaisesRegex(ValueError, "Changed directionOf signature"):
                generate(doc, self.semantics)

    def test_direction_operation_unreviewed_selector_and_branch_fail_closed(self):
        for key, value, message in [
            ("dispatch_source_class", "changed.Selector", "Changed directionOf invocation selector"),
            ("candidate_source_classes", ["changed.directionOf"], "Unresolved directionOf invocation branch")]:
            semantics = copy.deepcopy(self.semantics)
            next(b for b in semantics["delegate_bindings"] if "#//Type/operation:directionOf(" in b["element"])[key] = value
            with self.assertRaisesRegex(ValueError, message):
                generate(self.doc, semantics)


    def test_resolved_emf_defaults_preserved_separately_from_literals_and_constructors(self):
        output = generate(self.doc, self.semantics).decode()
        for feature in self.doc["features"]:
            start = output.index("id: " + json.dumps(feature["id"]))
            contract = output[start:output.index("    },", start)]
            self.assertIn("emf_default_json: " + json.dumps(json.dumps(feature["default_value"], ensure_ascii=False), ensure_ascii=False), contract)
        direction = next(f for f in self.doc["features"] if f["id"] == SYSML + "Feature/direction")
        self.assertIsNone(direction["default_literal"])
        self.assertEqual(direction["default_value"], "in")
        doc = copy.deepcopy(self.doc)
        next(f for f in doc["features"] if f["id"] == direction["id"])["default_value"] = "out"
        self.assertNotEqual(output, generate(doc, self.semantics).decode())


    def test_parameter_direction_resolved_operation_and_inherited_dispatch(self):
        output = generate(self.doc, self.semantics).decode()
        self.assertIn('name: "parameterDirection", owner: "ParameterMembership", target: "FeatureDirectionKind", lower: 1, upper: 1', output)
        for owner in ("ParameterMembership", "ReturnParameterMembership"):
            self.assertIn('("' + owner + '", "org.omg.sysml.delegate.invocation.' + owner + '_parameterDirection_InvocationDelegate")', output)
        for change in ({"parameters": [{"name": "newArgument"}]}, {"type": SYSML + "Feature"}, {"upper_bound": -1}):
            doc = copy.deepcopy(self.doc)
            next(o for o in doc["operations"] if o["name"] == "parameterDirection").update(change)
            with self.assertRaisesRegex(ValueError, "Changed parameterDirection signature"):
                generate(doc, self.semantics)

    def test_parameter_direction_rejects_unresolved_and_changed_selector(self):
        for key, value, message in [
            ("dispatch_source_class", "changed.Selector", "Changed parameterDirection invocation selector"),
            ("candidate_source_classes", ["changed.Unknown_parameterDirection_InvocationDelegate"], "Unresolved parameterDirection invocation branch"),
            ("candidate_source_classes", [], "Missing or duplicate parameterDirection invocation branch"),
        ]:
            semantics = copy.deepcopy(self.semantics)
            next(b for b in semantics["delegate_bindings"] if "operation:parameterDirection()" in b["element"])[key] = value
            with self.assertRaisesRegex(ValueError, message):
                generate(self.doc, semantics)



    def test_source_target_operation_preserves_signature_and_rejects_unreviewed_dispatch(self):
        output = generate(self.doc, self.semantics).decode()
        self.assertIn('name: "sourceTargetFeature", owner: "FeatureChainExpression", target: "Feature", lower: 0, upper: 1', output)
        for change in ({"parameters": [{"name": "newArgument"}]}, {"lower_bound": 1}, {"ordered": True}):
            doc = copy.deepcopy(self.doc)
            next(o for o in doc["operations"] if o["name"] == "sourceTargetFeature").update(change)
            with self.assertRaisesRegex(ValueError, "Changed sourceTargetFeature signature"):
                generate(doc, self.semantics)
        for key, value, message in [
            ("dispatch_source_class", "changed.Selector", "Changed sourceTargetFeature invocation selector"),
            ("candidate_source_classes", [], "Unreviewed sourceTargetFeature invocation branch"),
        ]:
            semantics = copy.deepcopy(self.semantics)
            next(b for b in semantics["delegate_bindings"] if "operation:sourceTargetFeature()" in b["element"])[key] = value
            with self.assertRaisesRegex(ValueError, message):
                generate(self.doc, semantics)

if __name__ == "__main__":
    unittest.main()
