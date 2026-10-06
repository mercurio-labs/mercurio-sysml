import unittest
from prepare_library_reference_closure import select

def row(name,package,refs=(),imports=()):return {'release_path':name,'packages':[package],'references':list(refs),'imports':list(imports)}
def ref(owner,target,**extra):return {'owner_package':owner,'target_segments':target,**extra}
class ReferenceClosureTests(unittest.TestCase):
 def test_qualified_superclassifier_adds_provider_without_import(self):
  d={'resources':[row('functions',['Functions'],[ref(['Functions'],['BaseFunctions','=='],kind='Subclassification',feature='superclassifier',target_type='Classifier',offset=42)]),row('base',['BaseFunctions'])]}
  r=select(d,[['Functions']]);self.assertEqual(r['resources'],['base','functions']);self.assertEqual(r['package_prefix_edges'][0]['feature'],'superclassifier');self.assertEqual(r['package_prefix_edges'][0]['offset'],42)
 def test_transitive_reference_and_import_cycles_are_order_independent(self):
  rows=[row('p',['P'],[ref(['P'],['Q','x'])]),row('q',['Q'],imports=[ref(['Q'],['R','x'])]),row('r',['R'],[ref(['R'],['P','x'])])]
  a=select({'resources':rows},[['P']]);b=select({'resources':list(reversed(rows))},[['P']]);self.assertEqual(a,b);self.assertEqual(a['resources'],['p','q','r'])
 def test_relative_package_precedes_global(self):
  d={'resources':[row('p',['P'],[ref(['P'],['N','x'])]),row('nested',['P','N']),row('global',['N'])]};self.assertEqual(select(d,[['P']])['resources'],['nested','p'])
 def test_unknown_and_unqualified_are_explicit_not_false_self_edges(self):
  r=select({'resources':[row('p',['P'],[ref(['P'],['Unknown','x']),ref(['P'],['local'])])]},[['P']]);self.assertEqual(r['resources'],['p']);self.assertEqual(len(r['unresolved_package_prefix_candidates']),1);self.assertEqual(len(r['unqualified_references_not_selected']),1)
 def test_filter_import_and_duplicate_or_missing_inputs(self):
  d={'resources':[row('p',['P'],imports=[ref(['P'],[],implicit_filter_target=True)])]};self.assertEqual(select(d,[['P']])['resources'],['p'])
  with self.assertRaisesRegex(ValueError,'Missing root'):select(d,[['Q']])
  with self.assertRaisesRegex(ValueError,'ambiguous'):select({'resources':d['resources']+[row('other',['P'])]},[['P']])
 def test_real_inventory_selects_base_functions_for_states(self):
  import json
  from pathlib import Path
  p=Path(__file__).resolve().parents[1]/'docs/conformance/2026-08-support/library-reference-inventory.json';r=select(json.loads(p.read_text(encoding='utf-8')),[['States']]);self.assertTrue(any(s.endswith('/BaseFunctions.kerml') for s in r['resources']));self.assertTrue(any(e.get('kind')=='Subclassification' and e['target_segments']==['BaseFunctions','=='] and e.get('provider','').endswith('/BaseFunctions.kerml') for e in r['package_prefix_edges']))
