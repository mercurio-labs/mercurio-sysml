"""Mutation checks for the resolved role-to-consumer generation contract."""
import copy,json,unittest
from generate_type_set_roles import PROFILE,derive

class TypeSetRoles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs=[json.loads((PROFILE/n).read_text(encoding="utf-8")) for n in ["grammar.structure.extract.json","ecore-effective.extract.json","ecore-semantics.extract.json"]]
    def test_complete_roles(self):
        rows=derive(*self.inputs)
        self.assertEqual(len(rows),4)
        self.assertEqual(sum(r["endpoint_field"] is not None for r in rows),3)
        self.assertEqual(next(r for r in rows if r["kind"]=="Disjoining")["source_derived"],False)
    def changed(self,field,value,owner,name):
        inputs=copy.deepcopy(self.inputs)
        f=next(f for f in inputs[1]["features"] if f["owner"].endswith("#//"+owner) and f["name"]==name)
        f[field]=value
        with self.assertRaises(ValueError):derive(*inputs)
    def test_endpoint_type(self):self.changed("type","wrong#//Package","Intersecting","intersectingType")
    def test_source_storage(self):self.changed("derived",True,"Disjoining","typeDisjoined")
    def test_endpoint_multiplicity(self):self.changed("lower_bound",0,"Unioning","unioningType")
    def test_opposite(self):self.changed("opposite",None,"Type","ownedDifferencing")
    def test_endpoint_uniqueness(self):self.changed("unique",False,"Type","intersectingType")
    def test_delegate_dispatch(self):
        inputs=copy.deepcopy(self.inputs)
        next(b for b in inputs[2]["delegate_bindings"] if b["element"].endswith("#//Type/ownedIntersecting"))["binding_status"]="custom_setting_delegate_source"
        with self.assertRaises(ValueError):derive(*inputs)

if __name__=="__main__":unittest.main()
