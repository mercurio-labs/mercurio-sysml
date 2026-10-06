import copy,json,unittest
from pathlib import Path
from run_literal_library_result_probe import validate_observations

class LiteralLibraryResultControls(unittest.TestCase):
    def setUp(self):
        self.controls=json.loads((Path(__file__).resolve().parents[1]/'docs/conformance/2026-08-support/literal-library-result-pilot-controls.json').read_text(encoding='utf-8'))
    def test_complete_actual_library_observations(self):
        validate_observations(self.controls)
    def test_rejects_missing_case_and_diagnostics(self):
        for mutation in ['missing','diagnostic']:
            data=copy.deepcopy(self.controls)
            if mutation=='missing':data['controls'].pop()
            else:data['resource_errors']=[{'message':'unresolved'}]
            with self.assertRaises(ValueError):validate_observations(data)
    def test_rejects_wrong_result_identity_and_ownership(self):
        for field,value in [('result_path',['Performances','Other']),('result_membership_kind','FeatureMembership'),('result_direction','in'),('result_present',False)]:
            data=copy.deepcopy(self.controls);data['controls'][0][field]=value
            with self.assertRaises(ValueError):validate_observations(data)
    def test_rejects_substitute_resource_and_provider(self):
        data=copy.deepcopy(self.controls);data['controls'][0]['result_uri']='memory:/substitute#result'
        with self.assertRaises(ValueError):validate_observations(data)
        data=copy.deepcopy(self.controls);data['provider']='standin'
        with self.assertRaises(ValueError):validate_observations(data)

if __name__=='__main__':unittest.main()
