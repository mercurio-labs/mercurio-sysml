import copy,json,unittest
import generate_expression_context_defaults as generator

class ExpressionContextGenerationTests(unittest.TestCase):
    def setUp(self): self.data=json.loads(generator.INPUT.read_text(encoding='utf-8'))
    def test_resolved_predicate_change_rejected(self):
        data=copy.deepcopy(self.data)
        data['methods']['org.omg.sysml.adapter.ExpressionAdapter#addDefaultGeneralType']['statements'][1]['condition']['expression']['symbol']='other#predicate'
        with self.assertRaisesRegex(ValueError,'tree changed'): generator.generate(data)
    def test_order_change_rejected(self):
        data=copy.deepcopy(self.data)
        rows=data['methods']['org.omg.sysml.adapter.ExpressionAdapter#addDefaultGeneralType']['statements']
        rows[1],rows[2]=rows[2],rows[1]
        with self.assertRaisesRegex(ValueError,'tree changed'): generator.generate(data)
    def test_duplicate_cannot_hide_context(self):
        data=copy.deepcopy(self.data);data['controls'][-1]=data['controls'][0]
        with self.assertRaisesRegex(ValueError,'Incomplete or duplicate'): generator.generate(data)
    def test_missing_provider_rejected(self):
        data=copy.deepcopy(self.data);data['bindings'].pop('Invariant')
        with self.assertRaisesRegex(ValueError,'inventory changed'): generator.generate(data)
    def test_generated_dependencies_and_order(self):
        text=generator.generate(self.data)
        self.assertLess(text.index('Objects::Object::ownedPerformances'),text.index('Performances::Performance::subperformances'))
        self.assertLess(text.index('Performances::Performance::subperformances'),text.index('Performances::Performance::enclosedPerformances'))
if __name__=='__main__': unittest.main()
