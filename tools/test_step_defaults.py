import copy,json,unittest
import generate_step_defaults as generator
class StepGenerationTests(unittest.TestCase):
    def setUp(self):self.data=json.loads(generator.INPUT.read_text(encoding='utf-8'))
    def test_override_rejected(self):
        self.data['bindings']['Step']['dispatch']['getTypes']='other#getTypes'
        with self.assertRaisesRegex(ValueError,'Overridden'):generator.generate(self.data)
    def test_changed_selector_rejected(self):
        self.data['methods']['org.omg.sysml.adapter.StepAdapter#getDefaultSupertype']['statements'][0]['expression']['arguments'][0]['condition']['symbol']='other#predicate'
        with self.assertRaisesRegex(ValueError,'selector/predicate changed'):generator.generate(self.data)
    def test_duplicate_default_cannot_hide_case(self):
        self.data['controls'][-1]=self.data['controls'][0]
        with self.assertRaisesRegex(ValueError,'Incomplete or duplicate'):generator.generate(self.data)
    def test_missing_implicit_typing_case_rejected(self):
        self.data['typing_controls']=self.data['typing_controls'][1:]
        with self.assertRaisesRegex(ValueError,'Incomplete Step typing'):generator.generate(self.data)
    def test_no_generalization_to_overridden_classes(self):
        self.data['bindings']['Expression']=copy.deepcopy(self.data['bindings']['Step'])
        with self.assertRaisesRegex(ValueError,'inventory changed'):generator.generate(self.data)
if __name__=='__main__':unittest.main()
