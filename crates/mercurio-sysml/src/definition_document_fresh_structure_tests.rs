//! Fresh physical construction evidence, separate from complete V01 lifecycle.
use super::*;
use crate::language_frontend::lowering::ecore_model;
use mercurio_foundation::kir::KirElement;
use std::collections::BTreeMap;

fn blank(id: &str, kind: &str) -> KirElement {
    KirElement {id: id.into(), kind: format!("SysML::{kind}"), layer: 2, properties: BTreeMap::new()}
}
fn ids(results: &[&KirElement]) -> Vec<String> {
    results.iter().map(|e| e.id.clone()).collect()
}

#[test]
fn definition_fresh_structure_matches_all_independent_concrete_pilot_factories() {
    let controls: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-result-fresh-structure-pilot-controls.json")).unwrap();
    assert_eq!(controls["qualification_certificate"], false);
    assert_eq!(controls["observations"].as_array().unwrap().len(), 167);
    for row in controls["observations"].as_array().unwrap() {
        let mut element = blank("fresh", row["kind"].as_str().unwrap());
        ecore_model::initialize_fresh_structure(&mut element);
        assert_eq!(row["completion_flag"], false);
        for (name, expected) in row["slots"].as_object().unwrap() {
            let field = match name.as_str() {
                "ownedRelationship" => "owned_relationship", "ownedRelatedElement" => "owned_related_element",
                "owningRelationship" => "owning_relationship", "owningRelatedElement" => "owning_related_element",
                other => panic!("Unreviewed stored construction contract: {other}"),
            };
            assert_eq!(&element.properties[field], expected, "{}.{name}", element.kind);
            let contract = ecore_model::feature(&element.kind, field).unwrap();
            assert!(!contract.derived && !contract.volatile && !contract.transient);
        }
        assert_eq!(element.properties.len(), row["slots"].as_object().unwrap().len());
        let before = serde_json::to_value(&element).unwrap();
        ecore_model::initialize_fresh_structure(&mut element);
        assert_eq!(before, serde_json::to_value(&element).unwrap());
        assert!(!element.properties.contains_key("is_implied_included"));
    }
}

#[test]
fn definition_fresh_structure_preserves_populated_and_invalid_explicit_slots() {
    let mut element = blank("relation", "OwningMembership");
    element.properties.insert("owned_related_element".into(), json!(["child"]));
    element.properties.insert("owning_related_element".into(), json!("parent"));
    element.properties.insert("owning_relationship".into(), json!(17));
    ecore_model::initialize_fresh_structure(&mut element);
    assert_eq!(element.properties["owned_related_element"], json!(["child"]));
    assert_eq!(element.properties["owning_related_element"], json!("parent"));
    assert_eq!(element.properties["owning_relationship"], json!(17));
    assert!(!ecore_model::validate_explicit_model(&[element]).is_empty());
}

#[test]
fn definition_fresh_structure_never_repairs_external_partial_graph_queries() {
    let graph = vec![blank("partial", "Package")];
    let results = query_constructed_references(&graph,
        &[("partial", "owning_relationship"), ("partial", "owned_relationship")]).unwrap();
    for (field, result) in ["owning_relationship", "owned_relationship"].iter().zip(results) {
        assert!(matches!(result, Err(DefinitionReferenceQueryError::Required {
            prerequisite: DefinitionQueryPrerequisite::ReadField { ref owner_id, field: ref actual }
        }) if owner_id=="partial" && actual==field));
    }
}

#[test]
fn definition_fresh_structure_all_original_syntax_nodes_have_canonical_slots_and_pending_reads() {
    let sources = [
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-00.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-01.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-02.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-03.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-04.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-05.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-06.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-07.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-08.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-09.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-10.kerml"),
        include_str!("../../../docs/conformance/2026-08-support/definition-pipeline-evidence/value-service-inputs/valuation-source-11.kerml"),
    ];
    let mut total_nodes = 0;
    let mut generated_parameters = 0;
    let mut pending_types = 0;
    for (i, text) in sources.into_iter().enumerate() {
        let uri = format!("fresh-{i}.kerml");
        let inspection = inspect_source_structure(&[
            DefinitionSource {uri: &uri, text, language: SourceLanguage::Kerml}]).unwrap();
        let graph = inspection.elements();
        total_nodes += graph.len();
        for element in graph {
            assert!(element.properties.contains_key("owned_relationship"), "{}", element.id);
            assert!(element.properties.contains_key("owning_relationship"), "{}", element.id);
            if ecore_model::feature(&element.kind, "owned_related_element").is_some() {
                assert!(element.properties.contains_key("owned_related_element"), "{}", element.id);
                assert!(element.properties.contains_key("owning_related_element"), "{}", element.id);
            }
            if element.id.contains(".operand.") && element.kind == "SysML::ParameterMembership" {
                generated_parameters += 1;
                assert_eq!(element.properties["owning_relationship"], serde_json::Value::Null);
                assert!(element.properties["owning_related_element"].is_string());
            }
        }
        let root = graph.iter().find(|e| e.id.split('.').count()==3).unwrap();
        assert_eq!(root.properties["owning_relationship"], serde_json::Value::Null);
        assert!(inspection.reference_targets_with_dependencies(&root.id, "owning_relationship").unwrap().is_empty());
        for port in inspection.pending_references().iter().filter(|p| p.field=="type") {
            pending_types += 1;
            assert!(matches!(inspection.reference_targets_with_dependencies(&port.owner_id, &port.field),
                Err(DefinitionReferenceQueryError::Required {
                    prerequisite: DefinitionQueryPrerequisite::ReadField {ref owner_id, ref field}
                }) if owner_id==&port.owner_id && field=="type"));
        }
        assert!(ecore_model::validate_explicit_reference_graph(graph).is_empty());
        assert!(graph.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
    }
    assert!(total_nodes > 150);
    assert!(generated_parameters > 5);
    assert!(pending_types >= 12);
}

#[test]
fn definition_fresh_structure_canonical_attachment_and_fresh_serialization_query() {
    let document = parse_and_link("package P { package Q; }", SourceLanguage::Kerml).unwrap();
    assert_eq!(document.metadata["semantic_validation"], json!("not_assessed"));
    assert!(ecore_model::validate_publication(&document.elements, ecore_model::ReferenceCompleteness::Closed).is_empty());
    let bytes = serde_json::to_vec(&document.elements).unwrap();
    let restored: Vec<KirElement> = serde_json::from_slice(&bytes).unwrap();
    let p = restored.iter().find(|e|e.properties.get("declared_name")==Some(&json!("P"))).unwrap();
    let q = restored.iter().find(|e|e.properties.get("declared_name")==Some(&json!("Q"))).unwrap();
    let requests = [(p.id.as_str(),"owned_member"),(q.id.as_str(),"owning_namespace")];
    let before = serde_json::to_value(&restored).unwrap();
    let results = query_constructed_references(&restored,&requests).unwrap();
    assert_eq!(ids(results[0].as_ref().unwrap()),vec![q.id.clone()]);
    assert_eq!(ids(results[1].as_ref().unwrap()),vec![p.id.clone()]);
    assert_eq!(before,serde_json::to_value(&restored).unwrap());
    assert!(restored.iter().all(|e|e.properties.get("is_implied_included")!=Some(&json!(true))));
}
