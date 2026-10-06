import copy
import json
import unittest
from generate_ecore_ownership import INPUT, URI, PROFILE, generate

class OwnershipGenerationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = json.loads(INPUT.read_text(encoding='utf-8'))
        cls.grammar = json.loads((PROFILE / 'grammar.structure.extract.json').read_text(encoding='utf-8'))
    def test_contracts_are_derived_from_resolved_endpoints(self):
        text = generate(self.doc, self.grammar).decode()
        self.assertIn('owner_kind: "Element", child_kind: "Relationship"', text)
        self.assertIn('owner_kind: "Relationship", child_kind: "Element"', text)
    def test_verification_metaclass_follows_resolved_grammar(self):
        grammar = copy.deepcopy(self.grammar)
        rule = next(r for g in grammar['grammars'] for r in g['rules']
                    if r['id'] == 'org.omg.sysml.xtext.SysML::RequirementVerificationUsage')
        rule['fields']['type']['fields']['classifier']['$ref'] = URI + 'PartUsage'
        self.assertIn('LEGACY_VERIFY_USAGE_METACLASS: &str = "PartUsage"', generate(self.doc, grammar).decode())
        rule['fields']['type']['fields']['classifier']['$ref'] = URI + 'Missing'
        with self.assertRaisesRegex(ValueError, 'not a resolved model class'):
            generate(self.doc, grammar)
    def test_unsupported_semantics_fail(self):
        for key, value in [('upper_bound', 1), ('ordered', False), ('unique', False), ('derived', True), ('changeable', False), ('containment', False)]:
            with self.subTest(key=key):
                doc = copy.deepcopy(self.doc)
                row = next(r for r in doc['features'] if r['id'] == URI + 'Element/ownedRelationship')
                row[key] = value
                with self.assertRaisesRegex(ValueError, 'Unsupported ownership contract'):
                    generate(doc, self.grammar)
    def test_broken_inverse_and_duplicate_identity_fail(self):
        doc = copy.deepcopy(self.doc)
        row = next(r for r in doc['features'] if r['id'] == URI + 'Relationship/owningRelatedElement')
        row['opposite'] = None
        with self.assertRaisesRegex(ValueError, 'Nonreciprocal'):
            generate(doc, self.grammar)
        doc = copy.deepcopy(self.doc)
        doc['features'].append(doc['features'][0])
        with self.assertRaisesRegex(ValueError, 'Duplicate'):
            generate(doc, self.grammar)

if __name__ == '__main__':
    unittest.main()
