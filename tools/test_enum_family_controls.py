import copy,json,unittest
from run_enum_family_probe import ROOT,OUTPUT,validate
class EnumFamilyControls(unittest.TestCase):
    def setUp(self):self.cases=json.loads(OUTPUT.read_text(encoding='utf-8'))
    def test_complete_reference_matrix(self):validate(self.cases)
    def test_missing_or_duplicated_declaration_rejects(self):
        for missing in (True,False):
            cases=copy.deepcopy(self.cases)
            if missing:cases['controls'].pop()
            else:cases['controls'][-1]=cases['controls'][0]
            with self.assertRaises(ValueError):validate(cases)
    def test_wrong_value_or_keyword_boundary_rejects(self):
        for boundary in (True,False):
            cases=copy.deepcopy(self.cases)
            target=next(c for c in cases['controls'] if c['polarity']==('boundary' if boundary else 'positive'))
            if boundary:target['accepted']=True
            else:target['value']=999
            with self.assertRaises(ValueError):validate(cases)
    def test_missing_caller_or_wrong_model_value_rejects(self):
        for missing in (True,False):
            cases=copy.deepcopy(self.cases)
            if missing:cases['model_controls'].pop()
            else:cases['model_controls'][0]['literal']='unsupported'
            with self.assertRaises(ValueError):validate(cases)
    def test_wrong_caller_identity_rejects(self):
        self.cases['model_controls'][0]['carrier_rule']='UnobservedCaller'
        with self.assertRaises(ValueError):validate(self.cases)
