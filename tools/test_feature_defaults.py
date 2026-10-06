import copy
import json
import unittest
from export_pilot_feature_defaults import OUTPUT, render, validate

class FeatureDefaultTests(unittest.TestCase):
    def setUp(self):
        self.doc = json.loads(OUTPUT.read_text(encoding="utf-8"))

    def test_resolved_selector_and_all_controls(self):
        validate(self.doc)
        self.assertEqual(len(self.doc["controls"]),384)
        for name in self.doc["defaults"].values():
            self.assertIn(json.dumps(name),render(self.doc))

    def test_changed_resolved_predicate_rejected(self):
        expression=self.doc["methods"]["selector"]["statements"][0]["expression"]
        expression["arguments"][0]["condition"]["symbol"]="unassessed#hasStructureType"
        with self.assertRaisesRegex(ValueError,"selector AST"): render(self.doc)

    def test_changed_priority_rejected(self):
        tree=self.doc["methods"]["selector"]["statements"][0]["expression"]["arguments"][0]
        tree["true"],tree["false"]=tree["false"],tree["true"]
        with self.assertRaisesRegex(ValueError,"selector AST"): render(self.doc)

    def test_missing_duplicate_disagreeing_controls_rejected(self):
        original=copy.deepcopy(self.doc["controls"])
        for rows in [original[:-1],original[:-1]+original[:1]]:
            self.doc["controls"]=rows
            with self.assertRaisesRegex(ValueError,"inventory"): render(self.doc)
        self.doc["controls"]=original
        self.doc["controls"][0]["default_supertype"]="wrong"
        with self.assertRaisesRegex(ValueError,"disagrees"): render(self.doc)

    def test_changed_dependency_rejected(self):
        self.doc["methods"]["dispatch"]["addDefaultGeneralType"]="unassessed#method"
        with self.assertRaisesRegex(ValueError,"dependencies"): render(self.doc)

    def test_subclass_rejected(self):
        self.doc["binding"]="Usage"
        with self.assertRaisesRegex(ValueError,"binding"): render(self.doc)

    def test_incomplete_owner_matrix_rejected(self):
        self.doc["controls"]=[r for r in self.doc["controls"] if not (r["owner"] == "Feature" and r["owner_typing_mask"] == 7)]
        with self.assertRaisesRegex(ValueError,"inventory"): render(self.doc)

    def test_owner_typing_priority_is_observed(self):
        rows=[r for r in self.doc["controls"] if r["owner"] == "Feature" and r["owner_typing_mask"] == 2 and r["typing_mask"] == 2 and r["composite"]]
        self.assertEqual(len(rows),2)
        self.assertTrue(all(r["default_supertype"] == self.doc["defaults"]["subobject"] for r in rows))

    def test_missing_source_comparison_rejected(self):
        self.doc["source_controls"].pop()
        with self.assertRaisesRegex(ValueError,"source control inventory"): render(self.doc)

    def test_missing_end_general_observation_rejected(self):
        self.doc["end_general_controls"][0]["general_queries"].pop()
        with self.assertRaisesRegex(ValueError,"end-general control inventory"): render(self.doc)

    def test_changed_participant_condition_rejected(self):
        self.doc["methods"]["participant"]["condition"]["expression"]["right"]["kind"]="UNARY_PLUS"
        with self.assertRaisesRegex(ValueError,"participant suppression AST"): render(self.doc)

    def test_changed_participant_effect_rejected(self):
        self.doc["methods"]["participant"]["effect"]["arguments"][0]["value"]="base"
        with self.assertRaisesRegex(ValueError,"participant suppression AST"): render(self.doc)

    def test_missing_connector_participant_case_rejected(self):
        self.doc["connector_participant_controls"].pop()
        with self.assertRaisesRegex(ValueError,"Connector participant inventory"): render(self.doc)

    def test_missing_absent_reference_case_rejected(self):
        self.doc["absent_reference_controls"].pop()
        with self.assertRaisesRegex(ValueError,"absent-reference inventory"): render(self.doc)

    def test_changed_association_selector_rejected(self):
        self.doc["methods"]["association_selector"]["statements"][0]["expression"]["condition"]["right"]["value"]=3
        with self.assertRaisesRegex(ValueError,"Association selector AST"): render(self.doc)

    def test_changed_association_dispatch_rejected(self):
        self.doc["association_dispatch"]["AssociationStructure"]["addDefaultGeneralType"]="unassessed"
        with self.assertRaisesRegex(ValueError,"Association inherited dependencies"): render(self.doc)

    def test_missing_association_control_rejected(self):
        self.doc["association_controls"].pop()
        with self.assertRaisesRegex(ValueError,"Association inventory"): render(self.doc)

    def test_unknown_association_binding_rejected(self):
        self.doc["association_defaults"]["Interaction"]={"base":"wrong","binary":"wrong"}
        with self.assertRaisesRegex(ValueError,"Association default bindings"): render(self.doc)

    def test_missing_feature_typing_case_rejected(self):
        self.doc["feature_type_controls"].pop()
        with self.assertRaisesRegex(ValueError,"Feature type inventory"): render(self.doc)

    def test_missing_association_type_case_rejected(self):
        self.doc["association_type_controls"].pop()
        with self.assertRaisesRegex(ValueError,"Association type inventory"): render(self.doc)

    def test_usage_dispatch_drift_rejected(self):
        self.doc["usage_bindings"]["Usage"]["dispatch"]["isAddMultiplicity"]="unassessed"
        with self.assertRaisesRegex(ValueError,"Usage dispatch"): render(self.doc)

    def test_usage_contribution_order_rejected(self):
        self.doc["methods"]["usage_contributions"].reverse()
        with self.assertRaisesRegex(ValueError,"Usage contributions"): render(self.doc)

    def test_usage_multiplicity_condition_rejected(self):
        self.doc["methods"]["usage_multiplicity"]["statements"][0]["expression"]["symbol"]="unassessed"
        with self.assertRaisesRegex(ValueError,"Usage multiplicity"): render(self.doc)

    def test_usage_missing_control_rejected(self):
        self.doc["usage_controls"].pop()
        with self.assertRaisesRegex(ValueError,"Usage control inventory"): render(self.doc)

    def test_usage_disagreeing_observation_rejected(self):
        self.doc["usage_controls"][0]["default_supertype"]="wrong"
        with self.assertRaisesRegex(ValueError,"Usage observation disagrees"): render(self.doc)

    def test_reference_fallback_guard_drift_rejected(self):
        self.doc["methods"]["reference_redefinition_fallback"]["guard_type"]="org.omg.sysml.lang.sysml.Type"
        with self.assertRaisesRegex(ValueError,"ReferenceUsage fallback"): render(self.doc)

    def test_reference_fallback_dispatch_drift_rejected(self):
        self.doc["reference_dispatch"]["getGeneralTypes"]="unassessed"
        with self.assertRaisesRegex(ValueError,"ReferenceUsage dispatch"): render(self.doc)

    def test_missing_reference_end_case_rejected(self):
        self.doc["reference_end_controls"].pop()
        with self.assertRaisesRegex(ValueError,"ReferenceUsage end inventory"): render(self.doc)

    def test_missing_usage_source_rejected(self):
        self.doc["usage_source_controls"].pop()
        with self.assertRaisesRegex(ValueError,"Usage source inventory"): render(self.doc)

    def test_missing_reference_parameter_case_rejected(self):
        self.doc["reference_parameter_controls"].pop()
        with self.assertRaisesRegex(ValueError,"ReferenceUsage parameter inventory"): render(self.doc)

    def test_reference_parameter_dispatch_drift_rejected(self):
        self.doc["reference_dispatch"]["filterIgnoredParameters"]="unassessed"
        with self.assertRaisesRegex(ValueError,"ReferenceUsage dispatch"): render(self.doc)

    def test_missing_binding_source_control_rejected(self):
        self.doc["binding_connector_controls"].pop()
        with self.assertRaisesRegex(ValueError,"BindingConnector source inventory"):render(self.doc)

    def test_reference_link_identity_drift_rejected(self):
        self.doc["methods"]["reference_redefinition_fallback"]["link_identity"]["kind"]="NOT_EQUAL_TO"
        with self.assertRaisesRegex(ValueError,"ReferenceUsage fallback"): render(self.doc)

    def test_reference_transition_inventory_rejected(self):
        self.doc["reference_transition_controls"].pop()
        with self.assertRaisesRegex(ValueError,"transition link inventory"): render(self.doc)

    def test_reference_transition_observation_rejected(self):
        self.doc["reference_transition_controls"][0]["is_transition_link"]=False
        with self.assertRaisesRegex(ValueError,"Transition link observation"): render(self.doc)

    def test_reference_default_override_drift_rejected(self):
        self.doc["methods"]["reference_default_contributions"][-1]["kind"]="RETURN"
        with self.assertRaisesRegex(ValueError,"ReferenceUsage default override"):render(self.doc)

    def test_reference_default_dispatch_drift_rejected(self):
        self.doc["usage_bindings"]["ReferenceUsage"]["dispatch"]["addDefaultGeneralType"]="org.omg.sysml.adapter.UsageAdapter#addDefaultGeneralType"
        with self.assertRaisesRegex(ValueError,"Usage dispatch"):render(self.doc)

    def test_reference_complete_matrix_required(self):
        rows=[r for r in self.doc["usage_controls"] if r["kind"]=="ReferenceUsage"]
        self.assertEqual(len(rows),64)
        self.doc["usage_controls"]= [r for r in self.doc["usage_controls"] if r["kind"]!="ReferenceUsage"]
        with self.assertRaisesRegex(ValueError,"Usage control inventory"):render(self.doc)

    def test_typing_algorithm_drift_rejected(self):
        self.doc["methods"]["typing_getTypes"][-1]["kind"]="RETURN"
        with self.assertRaisesRegex(ValueError,"Feature typing algorithm"):render(self.doc)

    def test_typing_dispatch_drift_rejected(self):
        self.doc["typing_bindings"]["ReferenceUsage"]["getTypes"]="unassessed"
        with self.assertRaisesRegex(ValueError,"Feature typing dispatch"):render(self.doc)

    def test_inherited_typing_matrix_required(self):
        self.doc["inherited_typing_controls"].pop()
        with self.assertRaisesRegex(ValueError,"inherited typing inventory"):render(self.doc)

    def test_inherited_typing_does_not_drive_default_selector(self):
        self.doc["inherited_typing_controls"][-1]["default_supertype"]="Objects::objects"
        with self.assertRaisesRegex(ValueError,"owned-typing default selector"):render(self.doc)

    def test_inherited_typing_graph_drift_rejected(self):
        self.doc["inherited_typing_controls"][-1]["edges"].pop()
        with self.assertRaisesRegex(ValueError,"inherited typing graph"):render(self.doc)

    def test_usage_definition_algorithm_drift_rejected(self):
        self.doc["methods"]["typing_usage_definition"][-1]["kind"]="RETURN"
        with self.assertRaisesRegex(ValueError,"Feature typing algorithm"):render(self.doc)

    def test_non_classifier_projection_drift_rejected(self):
        row=next(r for r in self.doc["inherited_typing_controls"] if r["kind"]=="ReferenceUsage" and r["typing_mask"]==8)
        row["projected_types"]=["type3"]
        with self.assertRaisesRegex(ValueError,"Classifier projection"):render(self.doc)
