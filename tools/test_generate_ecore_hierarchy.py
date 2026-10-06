import copy
import json
import unittest
from generate_ecore_hierarchy import INPUT, URI, hierarchy, generate

class HierarchyTests(unittest.TestCase):
    def test_multiple_inheritance_and_shared_ancestor(self):
        doc={'classes':[{'id':URI+k,'super_types':[URI+p for p in ps]} for k,ps in
                        [('Root',[]),('Left',['Root']),('Right',['Root']),('Leaf',['Left','Right'])]]}
        self.assertEqual(hierarchy(doc)['Leaf'],['Leaf','Left','Right','Root'])
        doc['classes'][3]['super_types']=['https://www.omg.org/spec/SysML/20250201#//Right']
        self.assertEqual(hierarchy(doc)['Leaf'],['Leaf','Right','Root'])
    def test_invalid_hierarchies_fail(self):
        for rows in [ [('A',['B'])], [('A',['B']),('B',['A'])], [('A',[]),('A',[])] ]:
            doc={'classes':[{'id':URI+k,'super_types':[URI+p for p in ps]} for k,ps in rows]}
            with self.assertRaises(ValueError):hierarchy(doc)
    def test_pinned_multiple_inheritance_and_unknown_default(self):
        doc=json.loads(INPUT.read_text(encoding='utf-8'));result=hierarchy(doc)
        self.assertEqual(len(result),175)
        self.assertIn('Namespace',result['Connector'])
        self.assertIn('Relationship',result['Connector'])
        self.assertNotIn('Relationship',result['Class'])
        self.assertIn('_ => false',generate(doc).decode())
        self.assertIn('pub(super) const CLASSES: &[&str]',generate(doc).decode())

if __name__=='__main__':unittest.main()
