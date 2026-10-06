import copy,json,unittest
from generate_type_relationship_scope import derive,PROFILE,walk
class ScopeGenerationTests(unittest.TestCase):
 def setUp(self):
  self.grammar=json.loads((PROFILE/'grammar.structure.extract.json').read_text(encoding='utf-8'))
  self.ecore=json.loads((PROFILE/'ecore-effective.extract.json').read_text(encoding='utf-8'))
 def test_exact_imported_inventory(self):
  rows=derive(self.grammar,self.ecore)
  self.assertEqual(len(rows),6)
  self.assertEqual(len({(r['kind'],r['field']) for r in rows}),5)
  self.assertTrue(all(r['target_type'].endswith('#//Type') for r in rows))
 def test_changed_endpoint_rejected(self):
  feature=next(f for f in self.ecore['features'] if f['owner'].endswith('#//Intersecting') and f['name']=='intersectingType')
  for key,value in [('derived',True),('upper_bound',-1),('type','wrong')]:
   changed=copy.deepcopy(self.ecore)
   next(f for f in changed['features'] if f['id']==feature['id'])[key]=value
   with self.assertRaises(ValueError):derive(self.grammar,changed)
 def test_missing_assignment_rejected(self):
  rule=next(r for r in walk(self.grammar) if r.get('kind')=='ParserRule' and r['fields']['type']['fields']['classifier']['$ref'].endswith('#//Intersecting'))
  rule['fields']['alternatives']={}
  with self.assertRaises(ValueError):derive(self.grammar,self.ecore)
if __name__=='__main__':unittest.main()
