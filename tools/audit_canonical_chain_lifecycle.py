"""Strict mutation envelope for the reviewed canonical-chain lifecycle stage.

The generic wave checker remains unchanged. This adapter admits only an explicit
false-to-true wrapper flag transition paired with its physical normative default.
It does not certify the producer, semantic validation or an enclosing lifecycle.
"""
import copy
from audit_cached_native_dependencies import assess_successful_wave,exact_json,require

def assess(nodes,pending,result,owners,default_target):
    require(owners and owners==sorted(set(owners)),"Lifecycle owners must be fixed, sorted and distinct")
    before={node["id"]:node for node in nodes};after={node["id"]:node for node in result["inspection"]["constructed_elements"]}
    require(default_target in before and before[default_target]["kind"]=="SysML::Feature","Required default target absent")
    require(result["committed_reference_fields"]==[],"Lifecycle batch unexpectedly committed a lexical read")
    adjusted=copy.deepcopy(nodes);allowed=set(owners);transitions=[]
    for node in adjusted:
        if node["id"] not in allowed:continue
        prop=node["properties"];current=after[node["id"]]["properties"]
        require(node["kind"]=="SysML::Feature" and prop["is_implied_included"] is False and current["is_implied_included"] is True,"Only reviewed incomplete Feature wrappers may complete")
        require(all(prop[field] is False for field in ["is_end","is_variable","is_composite","is_portion"]) and prop.get("direction") is None,"Lifecycle wrapper flags changed")
        children=[before[id] for id in prop["owned_relationship"]]
        require(len(children)>=2 and all(child["kind"]=="SysML::FeatureChaining" for child in children),"Initial wrapper has unreviewed contributions")
        require(before[prop["owning_relationship"]]["kind"] in ["SysML::ReferenceSubsetting","SysML::Subsetting","SysML::CrossSubsetting","SysML::TypeFeaturing"],"Lifecycle lacks a reviewed ownership role")
        require(exact_json(current["owned_relationship"][:len(children)],prop["owned_relationship"]),"Chain order changed")
        additions=current["owned_relationship"][len(children):];require(len(additions)==1,"One physical default per wrapper required")
        edge=after[additions[0]];ep=edge["properties"]
        require(edge["id"] not in before and edge["kind"]=="SysML::Subsetting" and ep["is_implied"] is True and ep["is_implied_included"] is False,"Unexpected default shape/flags")
        require(ep["subsetting_feature"]==node["id"] and ep["subsetted_feature"]==default_target and ep["owning_related_element"]==node["id"] and ep["owned_related_element"]==[],"Default endpoints/containment changed")
        prop["is_implied_included"]=True;transitions.append(node["id"])
    require(set(transitions)==allowed,"A lifecycle receiver disappeared")
    summary=assess_successful_wave(adjusted,pending,result,[dict(kind="chain_lifecycle_batch",owner_ids=owners)])
    require(summary["native_nodes_added"]==len(owners),"Unexpected native constructions")
    summary.update(canonical_chain_wrappers_completed=len(transitions),approved_flag_transitions=transitions)
    return summary
