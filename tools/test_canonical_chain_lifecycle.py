"""Adversarial controls for the narrow lifecycle mutation envelope."""
import copy,unittest
from audit_canonical_chain_lifecycle import assess

class ChainLifecycleEnvelope(unittest.TestCase):
    def fixture(self):
        base=dict(id="base",kind="SysML::Feature",properties={})
        context=dict(id="context",kind="SysML::ReferenceSubsetting",properties={})
        wrapper=dict(id="wrapper",kind="SysML::Feature",properties=dict(is_implied_included=False,is_end=False,is_variable=False,is_composite=False,is_portion=False,direction=None,owning_relationship="context",owned_relationship=["a","b"]))
        links=[dict(id=id,kind="SysML::FeatureChaining",properties=dict(chaining_feature=target)) for id,target in [("a","left"),("b","right")]]
        source=dict(id="source",kind="SysML::Feature",properties=dict(is_implied_included=False))
        nodes=[base,context,wrapper,*links,source];after=copy.deepcopy(nodes);after[2]["properties"].update(is_implied_included=True,owned_relationship=["a","b","default"])
        after.append(dict(id="default",kind="SysML::Subsetting",properties=dict(is_implied=True,is_implied_included=False,subsetting_feature="wrapper",subsetted_feature="base",owning_related_element="wrapper",owned_related_element=[])))
        return nodes,dict(element_count=len(after),committed_reference_fields=[],pending_reference_count=0,inspection=dict(constructed_elements=after,pending_references=[]))
    def test_exact_flag_transition_requires_physical_default(self):
        nodes,result=self.fixture();summary=assess(nodes,[],result,["wrapper"],"base");self.assertEqual(summary["canonical_chain_wrappers_completed"],1)
    def test_unrelated_source_completion_is_rejected(self):
        nodes,result=self.fixture();result["inspection"]["constructed_elements"][5]["properties"]["is_implied_included"]=True
        with self.assertRaises(ValueError):assess(nodes,[],result,["wrapper"],"base")
    def test_wrong_default_or_completion_claim_is_rejected(self):
        for field,value in [("subsetted_feature","source"),("is_implied",False),("is_implied_included",True),("subsetting_feature","source")]:
            nodes,result=self.fixture();result["inspection"]["constructed_elements"][-1]["properties"][field]=value
            with self.subTest(field=field),self.assertRaises(ValueError):assess(nodes,[],result,["wrapper"],"base")
    def test_chain_reordering_is_rejected(self):
        nodes,result=self.fixture();result["inspection"]["constructed_elements"][2]["properties"]["owned_relationship"]=["b","a","default"]
        with self.assertRaises(ValueError):assess(nodes,[],result,["wrapper"],"base")
    def test_unexpected_nodes_and_lexical_commits_are_rejected(self):
        for case in ["node","commit"]:
            nodes,result=self.fixture()
            if case=="node":result["inspection"]["constructed_elements"].append(dict(id="extra",kind="SysML::Feature",properties={}));result["element_count"]+=1
            else:result["committed_reference_fields"]=[dict(owner_id="a",field="chaining_feature")]
            with self.subTest(case=case),self.assertRaises(ValueError):assess(nodes,[],result,["wrapper"],"base")
    def test_receiver_flags_and_disappearing_scope_are_rejected(self):
        for case in ["variable","completion","scope"]:
            nodes,result=self.fixture()
            if case=="variable":nodes[2]["properties"]["is_variable"]=True
            elif case=="completion":nodes[2]["properties"]["is_implied_included"]=True
            else:result["inspection"]["constructed_elements"][2]["id"]="other"
            with self.subTest(case=case),self.assertRaises((ValueError,KeyError)):assess(nodes,[],result,["wrapper"],"base")

if __name__=="__main__":unittest.main()
