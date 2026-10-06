import copy
import json
import unittest
import export_pilot_feature_redefinitions as exporter


class FeatureRedefinitionPrograms(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = json.loads(exporter.OUTPUT.read_text(encoding="utf-8"))

    def test_checked_artifact_reproduces_native_dispatch(self):
        self.assertEqual(exporter.render(self.doc), exporter.GENERATED.read_text(encoding="utf-8"))
        self.assertEqual(len(self.doc["bindings"]), 79)
        self.assertEqual(sum(exporter.bounded(row) for row in self.doc["bindings"]), 30)

    def test_featuring_dispatch_requires_all_resolved_dependencies(self):
        rows = [r for r in self.doc["bindings"] if exporter.featuring_supported(r)]
        self.assertEqual(len(rows), 32)
        for field in ["computeFeaturingType", "isVariableGetter", "owningTypeGetter"]:
            row = copy.deepcopy(rows[0])
            row["methods"][field] = "unreviewed.Implementation#method"
            self.assertFalse(exporter.featuring_supported(row))
            del row["methods"][field]
            self.assertFalse(exporter.featuring_supported(row))
        self.assertFalse(exporter.featuring_supported(next(r for r in self.doc["bindings"] if r["kind"] == "Usage")))

    def test_usage_featuring_requires_resolved_variability_delegate_getter(self):
        rows = [r for r in self.doc["bindings"] if exporter.usage_featuring_supported(r)]
        self.assertEqual(len(rows), 47)
        for key in ["computeFeaturingType", "isVariableGetter", "mayTimeVaryGetter", "owningTypeGetter"]:
            row = copy.deepcopy(rows[0])
            row["methods"][key] = "unreviewed.Implementation#method"
            self.assertFalse(exporter.usage_featuring_supported(row))
        self.assertFalse(exporter.usage_featuring_supported(next(r for r in self.doc["bindings"] if r["kind"] == "Feature")))

    def test_participant_library_binding_is_imported(self):
        doc = copy.deepcopy(self.doc)
        doc["participant_default"] = "Example::DifferentParticipant"
        self.assertIn('PARTICIPANT_DEFAULT: &str = "Example::DifferentParticipant"', exporter.render(doc))
        doc["participant_default"] = None
        with self.assertRaisesRegex(ValueError, "Missing participant default"):
            exporter.render(doc)

    def test_selector_order_is_imported(self):
        doc = copy.deepcopy(self.doc)
        expression = doc["methods"][exporter.PREFIX + "FeatureAdapter#getRelevantFeatures"]["statements"][-1]["expression"]
        expression["true"], expression["false"] = expression["false"], expression["true"]
        self.assertNotEqual(exporter.render(doc), exporter.render(self.doc))
        self.assertIn("if !has_owner { if is_end", exporter.render(doc))

    def test_changed_resolved_call_is_rejected(self):
        doc = copy.deepcopy(self.doc)
        def replace(node):
            if isinstance(node, dict):
                if node.get("symbol") == "org.omg.sysml.lang.sysml.Feature#isEnd":
                    node["symbol"] = "unreviewed.Feature#isEnd"
                for value in node.values(): replace(value)
            elif isinstance(node, list):
                for value in node: replace(value)
        replace(doc["methods"])
        with self.assertRaisesRegex(ValueError, "Unsupported resolved selector call"):
            exporter.render(doc)

    def test_new_effect_does_not_inherit_supported_strategy(self):
        row = copy.deepcopy(next(row for row in self.doc["bindings"] if row["kind"] == "Feature"))
        row["methods"]["addFeatureWriteTypes"] = "unreviewed.Adapter#addFeatureWriteTypes"
        self.assertFalse(exporter.bounded(row))

    def test_nonempty_additional_members_need_an_algorithm(self):
        doc = copy.deepcopy(self.doc)
        doc["methods"][exporter.PREFIX + "NamespaceAdapter#addAdditionalMembers"]["statements"] = [{"kind": "EXPRESSION_STATEMENT"}]
        with self.assertRaisesRegex(ValueError, "Additional member algorithm required"):
            exporter.render(doc)

    def test_selector_effects_are_not_dropped(self):
        doc = copy.deepcopy(self.doc)
        doc["methods"][exporter.PREFIX + "MultiplicityAdapter#getRelevantFeatures"]["statements"].insert(0, {"kind": "EXPRESSION_STATEMENT"})
        with self.assertRaisesRegex(ValueError, "Unassessed selector statements"):
            exporter.render(doc)

    def test_end_source_order_is_imported(self):
        doc = copy.deepcopy(self.doc)
        expression = doc["methods"][exporter.PREFIX + "FeatureAdapter#getEndRelevantFeatures"]["statements"][0]["expression"]
        expression["true"], expression["false"] = expression["false"], expression["true"]
        self.assertIn("if same_owner { EndSource::Effective }", exporter.render(doc))
        self.assertNotEqual(exporter.render(doc), exporter.render(self.doc))

    def test_unknown_end_dependency_is_rejected(self):
        doc = copy.deepcopy(self.doc)
        expression = doc["methods"][exporter.PREFIX + "FeatureAdapter#getEndRelevantFeatures"]["statements"][0]["expression"]
        expression["false"]["symbol"] = "unreviewed.TypeUtil#getEndFeatureOf"
        with self.assertRaisesRegex(ValueError, "Unsupported resolved end selector"):
            exporter.render(doc)

    def test_end_adapter_override_is_not_admitted(self):
        doc = copy.deepcopy(self.doc)
        row = next(row for row in doc["bindings"] if row["kind"] == "Feature")
        row["methods"]["getEndRelevantFeatures"] = "unreviewed.Adapter#getEndRelevantFeatures"
        self.assertNotIn('"Feature"', exporter.render(doc).split("fn end_strategy_supported")[1].split("#[derive")[0])

    def test_parameter_branch_order_drives_generated_code(self):
        doc = copy.deepcopy(self.doc)
        statements = doc["methods"][exporter.PREFIX + "FeatureAdapter#getParameterRelevantFeatures"]["statements"]
        branch = statements[0]["then"]["statements"][0]
        branch["then"], branch["else"] = branch["else"], branch["then"]
        self.assertNotEqual(exporter.render(doc), exporter.render(self.doc))

    def test_changed_parameter_exclusion_is_rejected(self):
        doc = copy.deepcopy(self.doc)
        expression = doc["methods"][exporter.PREFIX + "FeatureAdapter#filterIgnoredParameters"]["statements"][0]["expression"]
        expression["receiver"]["arguments"][0]["body"]["kind"] = "UNARY_PLUS"
        with self.assertRaisesRegex(ValueError, "Changed parameter exclusion"):
            exporter.render(doc)

    def test_unknown_result_dependency_is_rejected(self):
        doc = copy.deepcopy(self.doc)
        statements = doc["methods"][exporter.PREFIX + "FeatureAdapter#getParameterRelevantFeatures"]["statements"]
        variable = statements[0]["then"]["statements"][0]["then"]["statements"][0]
        variable["initializer"]["symbol"] = "unknown.TypeUtil#getResultParameterOf"
        with self.assertRaisesRegex(ValueError, "Unsupported resolved parameter expression"):
            exporter.render(doc)


if __name__ == "__main__": unittest.main()
