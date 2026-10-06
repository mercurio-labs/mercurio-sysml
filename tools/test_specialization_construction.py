import json
import unittest
from export_pilot_specialization_construction import OUTPUT, render
class SpecializationConstructionTests(unittest.TestCase):
    def setUp(self): self.doc=json.loads(OUTPUT.read_text())
    def test_generated_program_contains_all_imported_kinds(self):
        generated=render(self.doc)
        for kind in self.doc["kinds"]: self.assertIn('"'+kind+'"',generated)
        self.assertIn('RESULT_KIND: &str = "Subsetting"',generated)
    def test_modified_resolved_definition_rejected(self):
        self.doc["definitions"]["TypeUtil#insertImplicitSpecializations"]["kind"]="EMPTY_STATEMENT"
        with self.assertRaisesRegex(ValueError,"Changed resolved"): render(self.doc)
    def test_missing_ownership_case_rejected(self):
        self.doc["cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete specialization"): render(self.doc)
    def test_missing_referent_case_rejected(self):
        self.doc["result_cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete result"): render(self.doc)
    def test_duplicate_kind_rejected(self):
        self.doc["kinds"][0]=self.doc["kinds"][1]
        with self.assertRaisesRegex(ValueError,"Changed specialization inventory"): render(self.doc)

    def test_changed_implied_flag_literal_rejected(self):
        def walk(node):
            yield node
            for child in node["children"]: yield from walk(child)
        literal=next(n for n in walk(self.doc["definitions"]["TypeUtil#insertImplicitSpecializations"]) if n["kind"]=="BOOLEAN_LITERAL")
        literal["value"]=False
        with self.assertRaisesRegex(ValueError,"Changed resolved"): render(self.doc)

    def test_changed_self_library_binding_rejected(self):
        self.doc["self_reference_feature"]="Base::Anything::other"
        with self.assertRaisesRegex(ValueError,"Changed self-reference"): render(self.doc)

    def test_missing_self_context_rejected(self):
        self.doc["self_cases"].pop()
        with self.assertRaisesRegex(ValueError,"Incomplete self-reference"): render(self.doc)
