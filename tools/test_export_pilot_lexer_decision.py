import copy,json,unittest
from export_pilot_lexer_decision import OUTPUT,validate
class LexerDecisionTests(unittest.TestCase):
    def setUp(self): self.doc=json.loads(OUTPUT.read_text(encoding='utf-8'))
    def test_imported_decision_is_complete(self): validate(self.doc)
    def test_missing_array_entry_rejects(self):
        self.doc['eot'].pop()
        with self.assertRaisesRegex(ValueError,'arrays'): validate(self.doc)
    def test_unknown_target_rejects(self):
        self.doc['eot'][0]=1000000
        with self.assertRaisesRegex(ValueError,'target'): validate(self.doc)
    def test_incomplete_character_domain_rejects(self):
        self.doc['special_ranges']['0'].pop()
        with self.assertRaisesRegex(ValueError,'domain'): validate(self.doc)
    def test_overlapping_character_domain_rejects(self):
        self.doc['special_ranges']['0'][1][0]=-1
        with self.assertRaisesRegex(ValueError,'domain'): validate(self.doc)
    def test_unknown_label_rejects(self):
        self.doc['accept'][0]=len(self.doc['labels'])+1
        with self.assertRaisesRegex(ValueError,'label'): validate(self.doc)
    def test_missing_keyword_identity_rejects(self):
        self.doc['keywords'].pop(next(iter(self.doc['keywords'])))
        with self.assertRaisesRegex(ValueError,'keyword'): validate(self.doc)
    def test_missing_language_rejects(self):
        self.doc.pop('kerml')
        with self.assertRaisesRegex(ValueError,'KerML'): validate(self.doc)
    def test_invalid_language_table_rejects(self):
        self.doc['kerml']['eot'][0]=1000000
        with self.assertRaisesRegex(ValueError,'target'): validate(self.doc)
    def test_changed_pin_rejects(self):
        self.doc['provenance']['pilot_revision']='other'
        with self.assertRaisesRegex(ValueError,'pin'): validate(self.doc)
if __name__=='__main__': unittest.main()
