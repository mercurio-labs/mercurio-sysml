import unittest,json
from check_succession_variation import validate,INPUT,ECORE,PDF
class NormativeVariationTests(unittest.TestCase):
 def setUp(self):self.data=json.loads(INPUT.read_text(encoding='utf-8'));self.ecore=json.loads(ECORE.read_text(encoding='utf-8'));self.pdf=PDF.read_bytes()
 def test_valid(self):self.assertEqual(len(validate(self.data,self.ecore,self.pdf)),73)
 def test_spec_drift(self):
  with self.assertRaises(ValueError):validate(self.data,self.ecore,b'other specification')
 def test_inheritance_drift(self):
  next(c for c in self.ecore['classes'] if c['name']=='SuccessionAsUsage')['super_types']=[]
  with self.assertRaises(ValueError):validate(self.data,self.ecore,self.pdf)
 def test_upstream_dispatch_drift(self):
  next(c for c in self.data['controls'] if c['kind']=='SuccessionAsUsage')['bound']=True
  with self.assertRaises(ValueError):validate(self.data,self.ecore,self.pdf)
if __name__=='__main__':unittest.main()
