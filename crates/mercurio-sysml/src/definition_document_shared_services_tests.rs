//! Whole shared ownership/selection/union batch; no complete lifecycle credit.
use super::*;
use mercurio_foundation::kir::KirElement;
use std::collections::BTreeMap;

fn node(id:&str, kind:&str, properties:serde_json::Value)->KirElement {
    KirElement {id:id.into(),layer:2,kind:format!("SysML::{kind}"),
        properties:properties.as_object().unwrap().iter().map(|(k,v)|(k.clone(),v.clone())).collect::<BTreeMap<_,_>>()}
}
fn fixture()->Vec<KirElement> {
    vec![
        node("P","Package",json!({"owned_relationship":["mg","mh","ma","w"]})),
        node("mg","OwningMembership",json!({"owning_related_element":"P","owned_related_element":["G"]})),
        node("mh","OwningMembership",json!({"owning_related_element":"P","owned_related_element":["H"]})),
        node("ma","OwningMembership",json!({"owning_related_element":"P","owned_related_element":["A"]})),
        node("G","Classifier",json!({"owning_relationship":"mg","owned_relationship":["mf","mt","mm1","mm2"]})),
        node("H","Classifier",json!({"owning_relationship":"mh","owned_relationship":["hg"]})),
        node("A","Classifier",json!({"owning_relationship":"ma","owned_relationship":["alias"]})),
        node("mf","FeatureMembership",json!({"owning_related_element":"G","owned_related_element":["f"]})),
        node("mt","FeatureMembership",json!({"owning_related_element":"G","owned_related_element":["g"]})),
        node("mm1","OwningMembership",json!({"owning_related_element":"G","owned_related_element":["m1"]})),
        node("mm2","OwningMembership",json!({"owning_related_element":"G","owned_related_element":["m2"]})),
        node("m1","MultiplicityRange",json!({"owning_relationship":"mm1","owned_relationship":[]})),
        node("m2","MultiplicityRange",json!({"owning_relationship":"mm2","owned_relationship":[]})),
        node("f","Feature",json!({"owning_relationship":"mf","owned_relationship":["c1","c2","v"]})),
        node("g","Feature",json!({"owning_relationship":"mt","owned_relationship":[]})),
        node("z","Feature",json!({"owned_relationship":[]})),
        node("hg","Subclassification",json!({"owning_related_element":"H","subclassifier":"H","superclassifier":"G"})),
        node("alias","Membership",json!({"owning_related_element":"A","member_element":"m1"})),
        node("c1","CrossSubsetting",json!({"owning_related_element":"f","crossed_feature":"g"})),
        node("c2","CrossSubsetting",json!({"owning_related_element":"f","crossed_feature":"f"})),
        node("v","FeatureValue",json!({"owning_related_element":"f","owned_related_element":["e"]})),
        node("e","LiteralInteger",json!({"owning_relationship":"v","owned_relationship":[],"value":1})),
        node("w","FeatureValue",json!({"owning_related_element":"P","owned_related_element":["we"]})),
        node("we","LiteralInteger",json!({"owning_relationship":"w","owned_relationship":[],"value":2})),
        node("D","Dependency",json!({"client":["f","g"],"supplier":["g","f"]})),
    ]
}
fn ids<'a>(results:&[&'a KirElement])->Vec<&'a str> { results.iter().map(|e|e.id.as_str()).collect() }
fn query<'a>(graph:&'a [KirElement],owner:&str,field:&str)->Result<Vec<&'a KirElement>,DefinitionReferenceQueryError> {
    query_constructed_references(graph,&[(owner,field)]).unwrap().remove(0)
}

#[test]
fn definition_shared_services_independent_cached_boundaries_and_normative_alias_disposition() {
    let graph=fixture();let before=serde_json::to_value(&graph).unwrap();
    let controls:serde_json::Value=serde_json::from_str(include_str!(
        "../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-result-lifecycle-shared-services-pilot-controls.json")).unwrap();
    assert_eq!(controls["qualification_certificate"],false);
    assert_eq!(controls["observations"].as_array().unwrap().len(),19);
    for control in controls["observations"].as_array().unwrap() {
        assert_eq!(control["completion_before"],false);
        assert_eq!(control["completion_after"],false);
        let owner=control["owner"].as_str().unwrap();
        let field=control["native_field"].as_str().unwrap();
        let result=query(&graph,owner,field);
        let actual=serde_json::to_value(ids(result.as_ref().unwrap())).unwrap();
        if owner=="A" && field=="multiplicity" {
            assert_eq!(control["targets"],json!(["m1"]));
            assert_eq!(actual,json!([]),"formal ownedMember rule excludes a non-owning alias");
        } else { assert_eq!(actual,control["targets"],"{owner}.{field}: {result:?}"); }
    }
    assert_eq!(before,serde_json::to_value(&graph).unwrap());
    assert!(graph.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
}

#[test]
fn definition_shared_services_pending_redefined_endpoints_are_exact_typed_reads() {
    let mut graph=fixture();
    graph.iter_mut().find(|e|e.id=="D").unwrap().properties.remove("supplier");
    let result=query(&graph,"D","related_element");
    assert!(matches!(result,Err(DefinitionReferenceQueryError::Required {
        prerequisite:DefinitionQueryPrerequisite::ReadField{ref owner_id,ref field}
    }) if owner_id=="D" && field=="supplier"));
    let mut graph=fixture();
    graph.iter_mut().find(|e|e.id=="c1").unwrap().properties.remove("crossed_feature");
    // Selection needs canonical subsetting ownership, not the general endpoint.
    assert_eq!(ids(&query(&graph,"f","owned_cross_subsetting").unwrap()),vec!["c1"]);
}

#[test]
fn definition_shared_services_alias_pending_endpoint_does_not_become_owned_member() {
    let mut graph=fixture();
    graph.iter_mut().find(|e|e.id=="alias").unwrap().properties.remove("member_element");
    assert!(query(&graph,"A","owned_member").unwrap().is_empty());
    assert!(query(&graph,"A","multiplicity").unwrap().is_empty());
}

#[test]
fn definition_shared_services_broken_reciprocals_and_duplicate_graph_reject() {
    for (owner,field,broken) in [("f","owning_namespace","mf"),("G","owned_member","mf"),
        ("v","feature_with_value","f"),("f","owned_cross_subsetting","c1")] {
        let mut graph=fixture();
        let node=graph.iter_mut().find(|e|e.id==broken).unwrap();
        if broken=="f" {node.properties.insert("owned_relationship".into(),json!(["c1","c2"]));}
        else { node.properties.insert("owning_related_element".into(),json!("A")); }
        assert!(matches!(query(&graph,owner,field),Err(DefinitionReferenceQueryError::Rejected{..})),
            "{owner}.{field} accepted broken reciprocal");
    }
    let mut graph=fixture();graph.push(graph[0].clone());
    assert!(query_constructed_references(&graph,&[("P","owned_member")]).is_err());
}

#[test]
fn definition_shared_services_definition_driven_source_to_owned_model_behavior() {
    let source=DefinitionSource {uri:"shared-services.kerml",language:SourceLanguage::Kerml,
        text:"package P { classifier G { feature f = 1; feature g; } }"};
    let inspection=inspect_source_structure(&[source]).unwrap();
    let graph=inspection.elements();
    let named=|name:&str|graph.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id.as_str();
    assert_eq!(ids(&query(graph,named("G"),"owned_member").unwrap()),vec![named("f"),named("g")]);
    assert_eq!(ids(&query(graph,named("f"),"owning_namespace").unwrap()),vec![named("G")]);
    let value=graph.iter().find(|e|e.kind=="SysML::FeatureValue").unwrap();
    assert_eq!(ids(&query(graph,&value.id,"feature_with_value").unwrap()),vec![named("f")]);
    assert!(query(graph,named("g"),"owned_cross_subsetting").unwrap().is_empty());
    assert!(query(graph,named("G"),"multiplicity").unwrap().is_empty());
}
