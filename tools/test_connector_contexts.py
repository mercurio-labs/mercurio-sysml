import copy,json,unittest
from export_connector_contexts import OUTPUT,render
class ConnectorContextTests(unittest.TestCase):
 def setUp(self):self.doc=json.loads(OUTPUT.read_text(encoding='utf-8'))
 def test_exact_inventory_and_program(self):
  self.assertEqual(len(self.doc['bindings']),12);self.assertEqual(len(self.doc['controls']),132);self.assertIn('SuccessionAsUsage',render(self.doc))
 def test_missing_binding_rejected(self):
  del self.doc['bindings']['Connector']
  with self.assertRaises(Exception):render(self.doc)
 def test_missing_context_program_rejected(self):
  del self.doc['definitions']['org.omg.sysml.adapter.SuccessionAsUsageAdapter#addContextFeaturingType']
  with self.assertRaises(Exception):render(self.doc)
 def test_changed_utility_rejected(self):
  self.doc['definitions']['org.omg.sysml.util.ConnectorUtil#getContextTypeFor']['kind']='EMPTY_STATEMENT'
  with self.assertRaises(Exception):render(self.doc)
 def test_duplicate_replacing_boundary_rejected(self):
  self.doc['controls'][-1]=copy.deepcopy(self.doc['controls'][0])
  with self.assertRaises(Exception):render(self.doc)
 def test_unknown_transform_rejected(self):
  self.doc['bindings']['Connector']['transform']='org.omg.sysml.adapter.Unknown#doTransform'
  with self.assertRaises(Exception):render(self.doc)
if __name__=='__main__':unittest.main()
