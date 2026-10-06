//! Focused controls for immutable native reference-query batches.
use super::*;
use mercurio_foundation::kir::KirElement;

fn fixture() -> DefinitionStructureInspection {
    inspect_source_structure(&[DefinitionSource {
        uri: "query-batch.kerml",
        text: "package P { function f { in x : Missing::Value; return result; } }",
        language: SourceLanguage::Kerml,
    }]).unwrap()
}

fn identity<'a>(graph: &'a [KirElement], name: &str) -> &'a str {
    &graph.iter().find(|e| e.properties.get("declared_name") == Some(&json!(name))).unwrap().id
}

#[test]
fn definition_query_batches_preserve_order_results_requirements_and_rejections() {
    let inspection = fixture();
    let graph = inspection.elements();
    let function = identity(graph, "f");
    let pending = &inspection.pending_references()[0];
    let requests = [(function, "result"), (pending.owner_id.as_str(), pending.field.as_str()),
        (function, "declared_name"), ("absent", "result"), (function, "result")];
    let before = serde_json::to_value(graph).unwrap();
    let results = query_constructed_references(graph, &requests).unwrap();
    assert_eq!(results.len(), requests.len());
    assert_eq!(results[0].as_ref().unwrap().iter().map(|e| &e.id).collect::<Vec<_>>(),
        results[4].as_ref().unwrap().iter().map(|e| &e.id).collect::<Vec<_>>());
    assert!(matches!(&results[1], Err(DefinitionReferenceQueryError::Required {
        prerequisite: DefinitionQueryPrerequisite::ReadField {owner_id, field}
    }) if owner_id == &pending.owner_id && field == &pending.field));
    assert!(matches!(&results[2], Err(DefinitionReferenceQueryError::Rejected {..})));
    assert!(matches!(&results[3], Err(DefinitionReferenceQueryError::Rejected {..})));
    for ((owner, field), result) in requests.iter().zip(&results) {
        let single = inspection.reference_targets_with_dependencies(owner, field);
        match (single, result) {
            (Ok(expected), Ok(actual)) => assert_eq!(
                expected.iter().map(|e| &e.id).collect::<Vec<_>>(),
                actual.iter().map(|e| &e.id).collect::<Vec<_>>()),
            (Err(DefinitionReferenceQueryError::Required {prerequisite: a}),
             Err(DefinitionReferenceQueryError::Required {prerequisite: b})) => assert_eq!(&a, b),
            (Err(DefinitionReferenceQueryError::Rejected {..}), Err(DefinitionReferenceQueryError::Rejected {..})) => {}
            other => panic!("Individual and batch query differ: {other:?}"),
        }
    }
    assert_eq!(before, serde_json::to_value(graph).unwrap());
    assert!(graph.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
}

#[test]
fn definition_query_batches_reject_invalid_graph_identity_before_requests() {
    let inspection = fixture();
    for empty in [false, true] {
        let mut graph = inspection.elements().to_vec();
        if empty {
            graph[0].id.clear();
        } else {
            graph.push(graph[0].clone());
        }
        assert!(matches!(query_constructed_references(&graph, &[("absent", "result")]),
            Err(DefinitionReferenceQueryError::Rejected {..})));
    }
}

#[test]
fn definition_query_batches_preserve_ecore_stored_and_derived_alias_contracts() {
    let inspection = fixture();
    let graph = inspection.elements();
    let function = identity(graph, "f");
    let result = identity(graph, "result");
    let requests = [(function, "owned_feature"), (function, "result"),
        (result, "owning_type"), (result, "owning_relationship"), (function, "owned_feature")];
    let results = query_constructed_references(graph, &requests).unwrap();
    assert!(results.iter().all(Result::is_ok), "{results:?}");
    assert_eq!(results[0].as_ref().unwrap().len(), 2);
    assert_eq!(results[1].as_ref().unwrap()[0].id, result);
    assert_eq!(results[2].as_ref().unwrap()[0].id, function);
    assert!(results[3].as_ref().unwrap()[0].kind.ends_with("ReturnParameterMembership"));
    assert_eq!(results[4].as_ref().unwrap().iter().map(|e| &e.id).collect::<Vec<_>>(),
        results[0].as_ref().unwrap().iter().map(|e| &e.id).collect::<Vec<_>>());
    assert!(!graph.iter().find(|e| e.id == function).unwrap().properties.contains_key("owned_feature"));
    assert!(!graph.iter().find(|e| e.id == result).unwrap().properties.contains_key("owning_type"));
}

#[test]
fn definition_query_batches_build_fresh_views_after_new_native_input() {
    for member in ["first", "second", "first"] {
        let text = format!("package P {{ function f {{ return '{member}'; }} }}");
        let inspection = inspect_source_structure(&[DefinitionSource {
            uri: "fresh-query.kerml", text: &text, language: SourceLanguage::Kerml,
        }]).unwrap();
        let graph = inspection.elements();
        let result = query_constructed_references(graph, &[(identity(graph, "f"), "result")]).unwrap();
        assert_eq!(result[0].as_ref().unwrap()[0].properties["declared_name"], json!(member));
    }
}
