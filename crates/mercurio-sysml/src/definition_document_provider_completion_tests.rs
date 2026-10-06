//! Focused dependency-bundle controls, separate from actual-library qualification.
use super::*;
use mercurio_foundation::kir::KirElement;

fn inspect(text:&str)->DefinitionStructureInspection {
    inspect_source_structure(&[DefinitionSource{uri:"provider-controls.kerml",text,language:SourceLanguage::Kerml}]).unwrap()
}
fn linked(text:&str)->DefinitionStructureInspection {
    let raw=inspect(text);
    let roots=raw.pending_references().iter().map(|port|DefinitionQueryPrerequisite::ReadField {
        owner_id:port.owner_id.clone(),field:port.field.clone()}).collect::<Vec<_>>();
    if roots.is_empty() {return raw;}
    inspect_source_dependencies(&[DefinitionSource{uri:"provider-controls.kerml",text,language:SourceLanguage::Kerml}],&roots).unwrap()
}
fn named<'a>(graph:&'a [KirElement],name:&str)-> &'a str {
    &graph.iter().find(|e|e.properties.get("declared_name")==Some(&json!(name))).unwrap().id
}
fn names(targets:&[&KirElement])->Vec<serde_json::Value> {
    targets.iter().map(|target|target.properties["declared_name"].clone()).collect()
}
fn reference()->serde_json::Value {
    serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-result-provider-completion-direction-controls.json"))).unwrap()
}
const BASE:&str="standard library package Base { abstract classifier Anything; }";

#[test]
fn definition_provider_completion_owned_input_output_use_context_direction_and_preserve_graph() {
    let inspection=linked(&format!("{BASE} package P {{ classifier G {{ in i; out o; inout b; feature u; }} }}"));
    let graph=inspection.elements();let before=serde_json::to_value(graph).unwrap();let g=named(graph,"G");
    for (name,direction) in [("i",json!("in")),("o",json!("out")),("b",json!("inout")),("u",json!(null))] {
        assert_eq!(query_constructed_feature_direction(graph,g,named(graph,name)).unwrap(),direction);
    }
    let queries=query_constructed_references(graph,&[(g,"input"),(g,"output"),(g,"membership")]).unwrap();
    assert!(queries.iter().all(Result::is_ok),"{queries:?}");
    assert_eq!(names(queries[0].as_ref().unwrap()),vec![json!("i"),json!("b")]);
    assert_eq!(names(queries[1].as_ref().unwrap()),vec![json!("o"),json!("b")]);
    assert_eq!(queries[2].as_ref().unwrap().len(),4);
    assert_eq!(before,serde_json::to_value(graph).unwrap());
    assert!(graph.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
}

#[test]
fn definition_provider_completion_direction_inherits_and_inverts_without_qualifying_effective_conjugation() {
    let inspection=linked(&format!("{BASE} package P {{ classifier G {{ in i; out o; inout b; feature u; }} classifier H specializes G; classifier C conjugates G; classifier D conjugates C; }}"));
    let graph=inspection.elements();let before=serde_json::to_value(graph).unwrap();
    let controls=reference();
    assert_eq!(controls["observations"].as_array().unwrap().len(),18);
    for control in controls["observations"].as_array().unwrap().iter()
        .filter(|row|row["supplied_materialized_operation_input"]==false) {
        assert_eq!(control["completion_before"],false);
        assert_eq!(control["completion_after"],false);
        assert_eq!(query_constructed_feature_direction(graph,named(graph,control["owner"].as_str().unwrap()),
            named(graph,control["feature"].as_str().unwrap())).unwrap(),control["value"]);
    }
    let queries=query_constructed_references(graph,&[(named(graph,"C"),"input")]).unwrap();
    assert!(matches!(&queries[0],Err(DefinitionReferenceQueryError::Rejected{..})),
        "context direction alone does not qualify conjugated effective feature construction");
    assert_eq!(before,serde_json::to_value(graph).unwrap());
}

#[test]
fn definition_provider_completion_union_composes_imported_then_owned_membership_order() {
    let inspection=linked("package Q { classifier A; classifier B; } package P { public import Q::*; classifier C; }");
    let graph=inspection.elements();let before=serde_json::to_value(graph).unwrap();
    let result=query_constructed_references(graph,&[(named(graph,"P"),"membership")]).unwrap();
    let members=result[0].as_ref().unwrap();
    let targets=members.iter().map(|member| {
        let id=member.properties["owned_related_element"][0].as_str().unwrap();
        graph.iter().find(|e|e.id==id).unwrap()
    }).collect::<Vec<_>>();
    assert_eq!(names(&targets),vec![json!("A"),json!("B"),json!("C")]);
    assert_eq!(before,serde_json::to_value(graph).unwrap());
}

#[test]
fn definition_provider_completion_pending_conjugation_and_import_reads_remain_typed() {
    let inspection=inspect("package P { classifier G { in i; } classifier C conjugates Missing; public import Absent::*; }");
    let graph=inspection.elements();
    let direction=query_constructed_feature_direction(graph,named(graph,"C"),named(graph,"i"));
    assert!(matches!(&direction,Err(DefinitionReferenceQueryError::Required{
        prerequisite:DefinitionQueryPrerequisite::ReadField{field,..}}) if field=="original_type"),"{direction:?}");
    let union=query_constructed_references(graph,&[(named(graph,"P"),"membership")]).unwrap();
    assert!(matches!(&union[0],Err(DefinitionReferenceQueryError::Required{
        prerequisite:DefinitionQueryPrerequisite::ReadField{field,..}}) if field=="imported_namespace"),"{union:?}");
}

#[test]
fn definition_provider_completion_direction_rejects_malformed_enum_and_identity() {
    let inspection=inspect("package P { classifier G { in i; } }");
    let graph=inspection.elements();let g=named(graph,"G").to_owned();let i=named(graph,"i").to_owned();
    let mut bad=graph.to_vec();
    bad.iter_mut().find(|e|e.id==i).unwrap().properties.insert("direction".into(),json!("unreviewed"));
    assert!(query_constructed_feature_direction(&bad,&g,&i).is_err());
    bad=graph.to_vec();bad.push(bad[0].clone());
    assert!(query_constructed_feature_direction(&bad,&g,&i).is_err());
    assert!(query_constructed_feature_direction(graph,"missing",&i).is_err());
    assert!(query_constructed_feature_direction(graph,&g,&g).is_err());
}

#[test]
fn definition_provider_completion_direction_terminates_cycles_on_supplied_materialized_operation_inputs() {
    let inspection=linked(&format!("{BASE} package P {{ classifier G {{ in i; }} classifier A specializes B; classifier B specializes A; }}"));
    let mut graph=inspection.elements().to_vec();
    // This explicit Ecore operation fixture supplies completed inputs. Production
    // providers are never prepared here, and this does not qualify a source cycle.
    for owner in ["A","B"] {
        let id=named(&graph,owner).to_owned();
        graph.iter_mut().find(|e|e.id==id).unwrap().properties.insert("is_implied_included".into(),json!(true));
    }
    let before=serde_json::to_value(&graph).unwrap();
    let controls=reference();
    for control in controls["observations"].as_array().unwrap().iter()
        .filter(|row|row["supplied_materialized_operation_input"]==true) {
        assert_eq!(control["completion_before"],true);
        assert_eq!(control["completion_after"],true);
        assert_eq!(query_constructed_feature_direction(&graph,named(&graph,control["owner"].as_str().unwrap()),
            named(&graph,control["feature"].as_str().unwrap())).unwrap(),control["value"]);
    }
    assert_eq!(before,serde_json::to_value(&graph).unwrap());
}
