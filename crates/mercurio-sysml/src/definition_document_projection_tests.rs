//! Shared default-list and canonical ownership delegate controls.
use super::*;

fn inspect(text: &str) -> DefinitionStructureInspection {
    inspect_source_structure(&[DefinitionSource {uri:"projection.kerml",text,language:SourceLanguage::Kerml}]).unwrap()
}
fn named<'a>(graph: &'a [mercurio_foundation::kir::KirElement], name:&str)-> &'a str {
    &graph.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id
}

#[test]
fn definition_default_subset_lists_preserve_membership_order_filtering_and_pending_endpoints() {
    let inspection=inspect("package P { function f specializes Missing::Prototype { in a; out b; feature c; return result; } }");
    let graph=inspection.elements();
    let f=named(graph,"f");
    let requests=[(f,"owned_feature_membership"),(f,"owned_specialization"),(f,"owned_intersecting"),(f,"owned_unioning")];
    let before=serde_json::to_value(graph).unwrap();
    let results=query_constructed_references(graph,&requests).unwrap();
    assert!(results.iter().all(Result::is_ok),"{results:?}");
    assert_eq!(results[0].as_ref().unwrap().len(),4);
    let fields=results[0].as_ref().unwrap().iter().map(|member|
        graph.iter().find(|e|Some(e.id.as_str())==member.properties["owned_related_element"][0].as_str())
        .unwrap().properties["declared_name"].clone()).collect::<Vec<_>>();
    assert_eq!(fields,vec![json!("a"),json!("b"),json!("c"),json!("result")]);
    assert_eq!(results[1].as_ref().unwrap().len(),1);
    assert!(results[1].as_ref().unwrap()[0].kind.ends_with("Subclassification"));
    assert!(results[2].as_ref().unwrap().is_empty());
    assert!(results[3].as_ref().unwrap().is_empty());
    assert!(!inspection.pending_references().is_empty(),"projection does not erase endpoint dependencies");
    assert_eq!(before,serde_json::to_value(graph).unwrap());
}

#[test]
fn definition_default_subset_lists_ignore_derived_snapshots() {
    let inspection=inspect("package P { function f { in a; return result; } }");
    let mut graph=inspection.elements().to_vec();
    let f=named(&graph,"f").to_owned();
    graph.iter_mut().find(|e|e.id==f).unwrap().properties.insert("owned_feature_membership".into(),json!(["invented"]));
    let result=query_constructed_references(&graph,&[(&f,"owned_feature_membership")]).unwrap();
    assert_eq!(result[0].as_ref().unwrap().len(),2);
    assert!(result[0].as_ref().unwrap().iter().all(|e|e.id!="invented"));
}

#[test]
fn definition_default_subset_lists_preserve_typed_source_prerequisites() {
    let inspection=inspect("package P { classifier G specializes Missing::Prototype; }");
    let graph=inspection.elements();
    let result=query_constructed_references(graph,&[(named(graph,"G"),"feature_membership")]).unwrap();
    assert!(matches!(&result[0],Err(DefinitionReferenceQueryError::Required {
        prerequisite:DefinitionQueryPrerequisite::ReadField {field,..}
    }) if field=="general" || field=="superclassifier"),"{result:?}");
}

#[test]
fn definition_feature_ownership_distinguishes_feature_and_ordinary_memberships() {
    let inspection=inspect("package P { feature free; function f { return result; feature inner { feature nested; } } }");
    let graph=inspection.elements();
    let requests=[(named(graph,"free"),"owning_type"),(named(graph,"free"),"owning_feature_membership"),
        (named(graph,"result"),"owning_type"),(named(graph,"result"),"owning_feature_membership"),
        (named(graph,"nested"),"owning_type")];
    let before=serde_json::to_value(graph).unwrap();
    let results=query_constructed_references(graph,&requests).unwrap();
    assert!(results.iter().all(Result::is_ok),"{results:?}");
    assert!(results[0].as_ref().unwrap().is_empty());
    assert!(results[1].as_ref().unwrap().is_empty());
    assert_eq!(results[2].as_ref().unwrap()[0].id,named(graph,"f"));
    assert!(results[3].as_ref().unwrap()[0].kind.ends_with("ReturnParameterMembership"));
    assert_eq!(results[4].as_ref().unwrap()[0].id,named(graph,"inner"));
    assert_eq!(before,serde_json::to_value(graph).unwrap());
    assert!(graph.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
}


#[test]
fn definition_default_subset_lists_shared_sources_preserve_order_and_specific_predicate() {
    let inspection=inspect("package P { class A; class B specializes Missing::Prototype; feature free; }");
    let graph=inspection.elements();
    let b=named(graph,"B").to_owned();
    let a=named(graph,"A").to_owned();
    let result=query_constructed_references(graph,&[(named(graph,"P"),"owned_element"),(&b,"owned_specialization")]).unwrap();
    assert!(result.iter().all(Result::is_ok),"{result:?}");
    assert_eq!(result[0].as_ref().unwrap().iter().map(|e|e.properties["declared_name"].clone())
        .collect::<Vec<_>>(),vec![json!("A"),json!("B"),json!("free")]);
    assert_eq!(result[1].as_ref().unwrap().len(),1);
    let relationship=result[1].as_ref().unwrap()[0].id.clone();
    let mut edited=graph.to_vec();
    edited.iter_mut().find(|e|e.id==relationship).unwrap().properties.insert("subclassifier".into(),json!(a));
    let result=query_constructed_references(&edited,&[(&b,"owned_specialization")]).unwrap();
    assert!(result[0].as_ref().unwrap().is_empty(),"foreign specific is excluded, not substituted from containment");
    assert!(!inspection.pending_references().is_empty(),"general endpoint still requires a typed read");
}
