"""Independent boundary checks for the Xtext/Ecore namespace projection."""
import copy
import json
from pathlib import Path
import unittest

from generate_namespace_grammar_contracts import Grammar, build, render_rust

ROOT = Path(__file__).resolve().parents[1]
PROFILE = ROOT / "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08"


class NamespaceContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.grammar = json.loads((PROFILE / "grammar.structure.extract.json").read_text(encoding="utf-8"))
        cls.model = json.loads((PROFILE / "metamodel.extract.json").read_text(encoding="utf-8"))

    def test_language_specific_ownership_and_annotation_elements(self):
        contract = build(copy.deepcopy(self.grammar), self.model)
        sysml, kerml = (contract["languages"][name] for name in ("sysml", "kerml"))
        self.assertEqual([s["feature"] for s in sysml["relationship_body"]["slots"]], ["ownedRelationship"])
        self.assertEqual([s["feature"] for s in kerml["relationship_body"]["slots"]], ["ownedRelatedElement", "ownedRelationship"])
        self.assertEqual(sysml["owned_annotation"]["metaclass"], "SysML::Annotation")
        self.assertEqual(sysml["owned_annotation"]["assignment"]["feature"], "ownedRelatedElement")
        self.assertEqual([a["metaclass"] for a in sysml["owned_annotation"]["elements"]],
                         ["SysML::Comment", "SysML::Documentation", "SysML::TextualRepresentation", "SysML::MetadataUsage"])
        self.assertEqual(kerml["owned_annotation"]["elements"][-1]["metaclass"], "SysML::MetadataFeature")
        self.assertIsNone(sysml["owned_related_element"])
        families = kerml["owned_related_element"]["families"]
        self.assertEqual([len(family["elements"]) for family in families], [26, 10])
        choices = {element["rule"].rsplit("::", 1)[1] for family in families for element in family["elements"]}
        self.assertTrue({"Package", "Function", "Feature", "Connector"} <= choices)
        self.assertIn('"Connector"', render_rust(contract))

    def test_imports_target_actual_memberships_and_expose_enum_is_protected(self):
        contract = build(copy.deepcopy(self.grammar), self.model)
        row = contract["languages"]["sysml"]
        self.assertEqual(row["expose_visibility"], [{"token": "expose", "value": "protected"}])
        for kind, field, target in [("MembershipImport", "importedMembership", "SysML::Membership"),
                                    ("NamespaceImport", "importedNamespace", "SysML::Namespace")]:
            assignments = [a for a in row["queries"][kind]["assignments"] if a["feature"] == field]
            self.assertEqual(len(assignments), 1)
            self.assertEqual(assignments[0]["terminal"]["kind"], "CrossReference")
            self.assertEqual(assignments[0]["field_contract"]["target"], target)
            self.assertFalse(assignments[0]["field_contract"]["containment"])
        self.assertIn('EXPOSE_VISIBILITY: &str = "protected"', render_rust(contract))

    def test_assignment_shape_and_unknown_reference_fail_closed(self):
        source = copy.deepcopy(self.grammar)
        rule = Grammar(source).rule("org.omg.sysml.xtext.SysML::RelationshipBody")
        rule["fields"]["alternatives"] = {"kind": "Action", "fields": {}}
        with self.assertRaisesRegex(ValueError, "Unsupported namespace projection"):
            build(source, self.model)
        source = copy.deepcopy(self.grammar)
        rule = Grammar(source).rule("org.omg.sysml.xtext.SysML::RelationshipBody")
        rule["fields"]["alternatives"] = {"kind": "RuleCall", "fields": {"rule": {"$ref": "unknown::Body"}}}
        with self.assertRaisesRegex(ValueError, "Unresolved grammar rule"):
            build(source, self.model)

    def test_ecore_cardinality_drift_is_rejected(self):
        model = copy.deepcopy(self.model)
        field = next(f for f in model["structural_features"] if f["qualified_name"] == "SysML::Element::ownedRelationship")
        field["upper_bound"] = 1
        with self.assertRaisesRegex(ValueError, "Additive assignment to singular"):
            build(copy.deepcopy(self.grammar), model)

    def test_constructed_objects_require_compatible_ecore_containment(self):
        for change, message in [({"kind": "attribute", "target": "Ecore::EString", "containment": False}, "requires Ecore containment"),
                                ({"target": "SysML::Package"}, "Constructed type does not conform")]:
            model = copy.deepcopy(self.model)
            field = next(f for f in model["structural_features"] if f["qualified_name"] == "SysML::Element::ownedRelationship")
            field.update(change)
            with self.assertRaisesRegex(ValueError, message):
                build(copy.deepcopy(self.grammar), model)

    def test_same_named_language_rules_are_separate_and_duplicate_identity_fails(self):
        rules = Grammar(self.grammar)
        self.assertNotEqual(rules.rule("org.omg.sysml.xtext.SysML::RelationshipBody"),
                            rules.rule("org.omg.kerml.xtext.KerML::RelationshipBody"))
        source = copy.deepcopy(self.grammar)
        source["grammars"][0]["rules"].append(source["grammars"][0]["rules"][0])
        with self.assertRaisesRegex(ValueError, "Duplicate qualified rule"):
            Grammar(source)

    def test_grammar_and_ecore_must_have_matching_provenance(self):
        for key, value, message in [("revision", "other", "Pilot revision mismatch"),
                                    ("hash", "0" * 64, "Ecore source hash mismatch")]:
            source = copy.deepcopy(self.grammar)
            if key == "revision": source["provenance"]["pilot_revision"] = value
            else: source["metamodel"]["sha256"] = value
            with self.assertRaisesRegex(ValueError, message):
                build(source, self.model)

    def test_derived_enum_changes_reach_native_output(self):
        source = copy.deepcopy(self.grammar)
        rule = Grammar(source).rule("org.omg.sysml.xtext.SysML::ExposeVisibilityKind")
        rule["fields"]["alternatives"]["fields"]["enumLiteral"]["$ref"] = "https://www.omg.org/spec/SysML/20250201#//VisibilityKind/private"
        self.assertIn('EXPOSE_VISIBILITY: &str = "private"', render_rust(build(source, self.model)))


if __name__ == "__main__":
    unittest.main()
