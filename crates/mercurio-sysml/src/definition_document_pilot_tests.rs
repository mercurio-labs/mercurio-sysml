//! Independent whole-document observations from pinned Pilot. The projection
//! reads effective non-derived attributes, including explicitly named getter
//! dependencies, rather than equating EObject.eGet with raw field storage.
use super::{parse_and_link, DefinitionDocumentError};
use crate::{KirDocument, SourceLanguage};
use crate::abstract_syntax_json::{export_sysml_abstract_syntax_value, import_sysml_abstract_syntax_value};
use crate::language_frontend::lowering::{ecore_defaults, ecore_model, emit};
use crate::language_frontend::lowering::relationship_declarations::metaclass_conforms;
use mercurio_foundation::kir::KirElement;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

fn reference_ids(value: &Value) -> Vec<&str> {
    match value {
        Value::Null => Vec::new(),
        Value::String(id) => vec![id.as_str()],
        Value::Array(values) => values.iter().map(|v| v.as_str().expect("canonical reference ID")).collect(),
        _ => panic!("Unexpected stored reference shape: {value:?}"),
    }
}

fn containment_paths<'a>(document: &'a KirDocument) -> (BTreeMap<&'a str, String>, BTreeMap<String, &'a KirElement>) {
    let index = document.elements.iter().map(|e| (e.id.as_str(), e)).collect::<BTreeMap<_, _>>();
    assert_eq!(index.len(), document.elements.len(), "Duplicate published identity");
    let children = document.elements.iter().flat_map(|element| element.properties.iter().filter_map(move |(field, value)| {
        let contract = ecore_model::feature(&element.kind, field)?;
        (contract.containment && !contract.derived && !contract.volatile && !contract.transient)
            .then(|| reference_ids(value))
    }).flatten()).collect::<BTreeSet<_>>();
    let roots = document.elements.iter().filter(|e| !children.contains(e.id.as_str())).collect::<Vec<_>>();
    assert_eq!(roots.len(), 1, "Exactly one source RootNamespace must be published");
    assert_eq!(roots[0].kind.rsplit("::").next(), Some("Namespace"));
    let mut paths = BTreeMap::new();
    let mut by_path = BTreeMap::new();
    let mut pending = vec![(roots[0], "$".to_string())];
    while let Some((element, path)) = pending.pop() {
        assert!(paths.insert(element.id.as_str(), path.clone()).is_none(), "Duplicate/cyclic containment");
        assert!(by_path.insert(path.clone(), element).is_none());
        for (field, value) in &element.properties {
            let Some(contract) = ecore_model::feature(&element.kind, field) else { continue; };
            if !contract.containment || contract.derived || contract.volatile || contract.transient { continue; }
            for (ordinal, id) in reference_ids(value).into_iter().enumerate() {
                pending.push((*index.get(id).expect("Closed containment"), format!("{path}/{field}/{ordinal}")));
            }
        }
    }
    assert_eq!(paths.len(), document.elements.len(), "No disconnected or extra native objects");
    (paths, by_path)
}

fn attribute_value(graph: &[KirElement], element: &KirElement, field: &str) -> Value {
    let contract = ecore_model::feature(&element.kind, field).expect("Imported attribute identity");
    assert_eq!(contract.kind, ecore_model::FeatureKind::Attribute);
    assert!(!contract.derived && !contract.volatile && !contract.transient);
    // Pilot's non-derived Membership member-name getters are overridden for
    // owning memberships. Compose the existing canonical endpoint/name services
    // explicitly; do not invent a default or copy a fixture's expected string.
    if matches!(field, "member_name" | "member_short_name")
        && metaclass_conforms(&element.kind, "OwningMembership")
    {
        let emit::StoredMembershipEndpoint::Resolved(member) =
            emit::stored_membership_endpoint(graph, element, "member_element").unwrap()
        else { panic!("Owning membership has no canonical endpoint"); };
        return emit::generated_effective_names(graph, member).unwrap()
            [usize::from(field == "member_short_name")].clone().map(Value::String).unwrap_or(Value::Null);
    }
    if contract.upper != 1 && !element.properties.contains_key(field) {
        return Value::Array(Vec::new());
    }
    ecore_defaults::read_attribute(element, field)
        .unwrap_or_else(|error| panic!("Unimplemented effective attribute {}.{field}: {error}", element.kind))
}

fn compare_document(document: &KirDocument, case: &Value) {
    let (paths, by_path) = containment_paths(document);
    let expected = case["nodes"].as_array().unwrap();
    assert_eq!(by_path.len(), expected.len(), "No missing/extra canonical nodes for {}", case["source"]);
    for row in expected {
        let path = row["path"].as_str().unwrap();
        let element = by_path.get(path).unwrap_or_else(|| panic!("Missing canonical node {path}"));
        assert_eq!(element.kind.rsplit("::").next().unwrap(), row["kind"].as_str().unwrap(), "{path}");
        let mut fields = row["attributes"].as_object().unwrap().keys().map(String::as_str).collect::<BTreeSet<_>>();
        // Include explicit native attributes too, so an extra non-null stored
        // scalar cannot disappear merely because the oracle omitted it.
        fields.extend(element.properties.iter().filter_map(|(field, value)| {
            let contract = ecore_model::feature(&element.kind, field)?;
            (contract.kind == ecore_model::FeatureKind::Attribute && !contract.derived
                && !contract.volatile && !contract.transient && field != "element_id" && !value.is_null())
                .then_some(field.as_str())
        }));
        let attributes = fields.into_iter().filter_map(|field| {
            let value = attribute_value(&document.elements, element, field);
            (!value.is_null()).then_some((field.to_string(), value))
        }).collect::<serde_json::Map<_, _>>();
        assert_eq!(json!(attributes), row["attributes"], "Effective attribute mismatch at {path}: {}", case["source"]);
        let mut children = serde_json::Map::new();
        for (field, value) in &element.properties {
            let Some(contract) = ecore_model::feature(&element.kind, field) else { continue; };
            if !contract.containment || contract.derived || contract.transient || contract.volatile { continue; }
            children.insert(field.clone(), json!(reference_ids(value).iter().map(|id| &paths[id]).collect::<Vec<_>>()));
        }
        assert_eq!(json!(children), row["children"], "Containment order mismatch at {path}");
    }
    // A nonempty extra canonical stored reference is a discrepancy too.
    for (path, element) in &by_path {
        for (field, value) in &element.properties {
            let Some(contract) = ecore_model::feature(&element.kind, field) else { continue; };
            if contract.kind != ecore_model::FeatureKind::Reference || contract.derived
                || contract.volatile || contract.transient || contract.container || contract.containment
                || reference_ids(value).is_empty() { continue; }
            assert!(case["links"].as_array().unwrap().iter().any(|link|
                link["owner_path"].as_str() == Some(path.as_str()) && link["field"].as_str() == Some(field.as_str())),
                "Extra native stored reference {path}.{field}");
        }
    }
    for link in case["links"].as_array().unwrap() {
        let owner = by_path[link["owner_path"].as_str().unwrap()];
        let field = link["field"].as_str().unwrap();
        let targets = super::reference_targets(document, &owner.id, field)
            .unwrap_or_else(|error| panic!("Unsupported effective reference {}.{field}: {error}", owner.kind));
        let actual = targets.iter().map(|target| &paths[target.id.as_str()]).collect::<Vec<_>>();
        assert_eq!(json!(actual), link["target_paths"], "Resolved target mismatch for {link}");
    }
}

#[test]
fn definition_documents_match_independent_pilot_models_links_and_roundtrips() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json"
    )).unwrap();
    let mut accepted = 0;
    for case in artifact["controls"].as_array().unwrap() {
        let language = match case["language"].as_str().unwrap() {
            "kerml" => SourceLanguage::Kerml,
            "sysml" => SourceLanguage::Sysml,
            other => panic!("Unknown oracle language {other}"),
        };
        let source = case["source"].as_str().unwrap();
        let result = parse_and_link(source, language);
        if !case["accepted"].as_bool().unwrap() {
            assert!(matches!(result, Err(DefinitionDocumentError::Syntax(_))),
                "An unsupported execution is not an independent syntax rejection: {source}: {result:?}");
            continue;
        }
        let document = result.unwrap_or_else(|error| panic!("{source}: {error}"));
        compare_document(&document, case);
        let exported = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
        assert!(!exported.has_errors(), "{:?}", exported.diagnostics);
        let imported = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors(), "{:?}", imported.diagnostics);
        compare_document(&imported.persistable_document().unwrap(), case);
        accepted += 1;
    }
    assert_eq!(accepted, 90);
}

#[test]
fn definition_feature_types_match_pilot_and_roundtrip() {
    let artifact: Value = serde_json::from_str(include_str!("../resources/metamodels/sysml-2.0-pilot-2026-08/feature-defaults.extract.json")).unwrap();
    let controls = artifact["feature_type_controls"].as_array().unwrap();
    assert_eq!(controls.len(), 8);
    let associations = artifact["association_type_controls"].as_array().unwrap();
    assert_eq!(associations.len(), 8);
    let disagreement: Value = serde_json::from_str(include_str!(
        "../../../docs/conformance/2026-08-support/feature-type-disagreement.json")).unwrap();
    assert_eq!(controls.iter().filter(|c| c["source"] == disagreement["source"]).count(), 1);
    for case in controls.iter().chain(associations) {
        let doc = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
        compare_document(&doc, case);
        let exported = export_sysml_abstract_syntax_value(&doc, Default::default()).unwrap();
        assert!(!exported.has_errors());
        let imported = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors());
        assert!(imported.publication_diagnostics(true).is_empty());
        let roundtrip = imported.persistable_document().unwrap();
        compare_document(&roundtrip, case);
        for document in [&doc, &roundtrip] {
            let feature = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("x"))).unwrap();
            let before = serde_json::to_value(document).unwrap();
            if case.get("types").is_some() {
                let types = super::reference_targets(document, &feature.id, "type").unwrap();
                let expected = if case["source"] == disagreement["source"] {
                    assert_eq!(case["types"], disagreement["pilot_types"]);
                    assert_ne!(disagreement["pilot_types"], disagreement["normative_types"]);
                    &disagreement["normative_types"]
                } else { &case["types"] };
                assert_eq!(&json!(types.iter().map(|e| &e.properties["declared_name"]).collect::<Vec<_>>()), expected, "{}", case["source"]);
            } else {
                for field in ["related_type", "source_type", "target_type"] {
                    let types = super::reference_targets(document, &feature.id, field).unwrap();
                    assert_eq!(json!(types.iter().map(|e| &e.properties["declared_name"]).collect::<Vec<_>>()), case["association_types"][field], "{}", case["source"]);
                }
            }
            assert_eq!(serde_json::to_value(document).unwrap(), before);
            assert!(document.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
        }
    }
}

#[test]
fn definition_feature_owned_sources_match_pilot_and_roundtrip() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/feature-defaults.extract.json")).unwrap();
    let controls = artifact["source_controls"].as_array().unwrap();
    assert_eq!(controls.len(), 2);
    let ends = artifact["end_general_controls"].as_array().unwrap();
    assert_eq!(ends.len(), 2);
    let connectors = artifact["connector_participant_controls"].as_array().unwrap();
    assert_eq!(connectors.len(), 5);
    let absent = artifact["absent_reference_controls"].as_array().unwrap();
    assert_eq!(absent.len(), 5);
    let associations = artifact["association_source_controls"].as_array().unwrap();
    assert_eq!(associations.len(), 4);
    let bindings=artifact["binding_connector_controls"].as_array().unwrap();assert_eq!(bindings.len(),4);
    for case in controls.iter().chain(ends).chain(connectors).chain(absent).chain(associations).chain(bindings) {
        assert_eq!(case["accepted"], true);
        let document = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
        compare_document(&document, case);
        let exported = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
        assert!(!exported.has_errors(), "{:?}", exported.diagnostics);
        let imported = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors(), "{:?}", imported.diagnostics);
        assert!(imported.publication_diagnostics(true).is_empty());
        compare_document(&imported.persistable_document().unwrap(), case);
        if let Some(projections) = case["connector_projections"].as_array() {
            for checked in [&document, &imported.persistable_document().unwrap()] {
                for projection in projections {
                    let connector = checked.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("x"))).unwrap();
                    for field in ["source_feature", "target_feature", "related_feature"] {
                        let actual = super::reference_targets(checked, &connector.id, field).unwrap();
                        assert_eq!(json!(actual.iter().map(|e| &e.properties["declared_name"]).collect::<Vec<_>>()), projection[field]);
                    }
                    assert!(checked.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
                }
            }
        }
    }
}

#[test]
fn definition_usage_provider_sources_match_pilot_and_roundtrip() {
    let artifact: Value = serde_json::from_str(include_str!("../resources/metamodels/sysml-2.0-pilot-2026-08/feature-defaults.extract.json")).unwrap();
    let controls = artifact["usage_source_controls"].as_array().unwrap();
    assert_eq!(controls.len(), 4);
    for case in controls {
        let document = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Sysml).unwrap();
        compare_document(&document, case);
        let before = serde_json::to_value(&document).unwrap();
        let exported = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
        assert!(!exported.has_errors());
        let imported = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors());
        assert!(imported.publication_diagnostics(true).is_empty());
        compare_document(&imported.persistable_document().unwrap(), case);
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
        assert!(document.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
    }
}

#[test]
fn definition_ordinary_strategy_source_batch_matches_pilot() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json")).unwrap();
    let controls = artifact["ordinary_strategy_source_controls"].as_array().unwrap();
    assert_eq!(controls.len(), 25);
    for case in controls {
        let language = if case["language"] == "kerml" { SourceLanguage::Kerml } else { SourceLanguage::Sysml };
        let source = case["source"].as_str().unwrap();
        let result = parse_and_link(source, language);
        if case["accepted"] == false {
            assert!(matches!(result, Err(DefinitionDocumentError::Syntax(_))), "{source}: {result:?}");
            continue;
        }
        let document = result.unwrap_or_else(|error| panic!("{source}: {error}"));
        if source.contains("bind a = b") || source.contains("first a then b") || source.contains("binding x of a = b") {
            // Scope/endpoint construction is verified separately. These sources
            // omit required libraries or still need unbound subtype providers.
            let (paths, by_path) = containment_paths(&document);
            let mut checked = 0;
            for link in case["links"].as_array().unwrap() {
                let owner = by_path[link["owner_path"].as_str().unwrap()];
                if !metaclass_conforms(&owner.kind, "ReferenceSubsetting") { continue; }
                let targets = super::reference_targets(&document, &owner.id, link["field"].as_str().unwrap()).unwrap();
                assert_eq!(json!(targets.iter().map(|e| &paths[e.id.as_str()]).collect::<Vec<_>>()), link["target_paths"]);
                checked += 1;
            }
            assert!(checked > 0);
            for node in case["nodes"].as_array().unwrap() {
                if node["attributes"]["is_end"] == true {
                    assert_eq!(by_path[node["path"].as_str().unwrap()].properties["is_end"], true);
                }
            }
            let connector = document.elements.iter().find(|e| metaclass_conforms(&e.kind, "Connector")).unwrap();
            let error = super::reference_targets(&document, &connector.id, "source").unwrap_err().to_string();
            if matches!(connector.kind.rsplit("::").next(),Some("BindingConnector" | "BindingConnectorAsUsage")) {
                // The bounded provider is implemented; this source intentionally
                // has no library, so its remaining dependency is explicit.
                assert!(error.contains("missing standard library root Links"), "{error}");
            } else {
                assert!(error.contains("generalization provider") || error.contains("added-member strategy"), "{error}");
            }
            assert!(ecore_model::validate_publication(&document.elements, ecore_model::ReferenceCompleteness::Closed).is_empty());
            continue; // Partial scope evidence, excluded from matching models.
        }
        compare_document(&document, case);
        let exported = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
        assert!(!exported.has_errors(), "{source}: {:?}", exported.diagnostics);
        let imported = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors(), "{source}: {:?}", imported.diagnostics);
        assert!(imported.publication_diagnostics(true).is_empty(), "{source}");
        compare_document(&imported.persistable_document().unwrap(), case);
    }
}

#[test]
fn definition_connector_projections_match_complete_pilot_inputs() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json")).unwrap();
    let controls = artifact["connector_projection_controls"].as_array().unwrap();
    assert_eq!(controls.len(), 7);
    for case in controls {
        assert_eq!(case["accepted"], true);
        let mut document = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
        // Match explicit complete-generalization inputs supplied in Pilot.
        // Production projection never invents these flags.
        for element in &mut document.elements {
            if metaclass_conforms(&element.kind, "Type") { element.properties.insert("is_implied_included".into(), json!(true)); }
        }
        let before = serde_json::to_value(&document).unwrap();
        compare_document(&document, case);
        assert_eq!(serde_json::to_value(&document).unwrap(), before);
        if case["source"] == "package P { connector x { end feature e; } }" {
            let end = document.elements.iter_mut().find(|e| e.properties.get("declared_name") == Some(&json!("e"))).unwrap();
            end.properties.insert("is_implied_included".into(), json!(false));
            let connector = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("x"))).unwrap();
            let error = super::reference_targets(&document, &connector.id, "related_feature").unwrap_err().to_string();
            assert!(error.contains("implicit reference-subsetting provider"), "{error}");
            document.elements.iter_mut().find(|e| e.properties.get("declared_name") == Some(&json!("e"))).unwrap()
                .properties.insert("is_implied_included".into(), json!(true));
        }
        let exported = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
        assert!(!exported.has_errors());
        let imported = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors());
        compare_document(&imported.persistable_document().unwrap(), case);
    }
}

#[test]
fn definition_library_provider_disagreements_remain_explicit() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json"
    )).unwrap();
    let disagreements = artifact["library_lookup_disagreements"].as_array().unwrap();
    assert_eq!(disagreements.len(), 2);
    for case in disagreements {
        assert_eq!(case["syntax_accepted"], true);
        assert_eq!(case["error_count"], 1);
        assert_eq!(case["unresolved_references"], json!([
            {"owner_kind":"FeatureTyping", "field":"target"},
            {"owner_kind":"FeatureTyping", "field":"general"},
            {"owner_kind":"FeatureTyping", "field":"type"},
        ]));
        // Native closed-graph library binding deliberately uses canonical
        // membership/name semantics. Pilot's exported-object index does not
        // resolve these aliases/re-exports. This is separate evidence, never
        // counted as a matching Pilot document or normative qualification.
        let document = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
        let target = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("FromDefault"))).unwrap();
        let typing = document.elements.iter().find(|e| metaclass_conforms(&e.kind, "FeatureTyping")).unwrap();
        assert_eq!(super::reference_targets(&document, &typing.id, "type").unwrap()[0].id, target.id);
    }
}

#[test]
fn definition_individual_source_exposes_variability_dependency() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json"
    )).unwrap();
    let cases = artifact["unsupported_native_dependencies"].as_array().unwrap();
    assert_eq!(cases.len(), 1);
    let case = &cases[0];
    assert_eq!(case["accepted"], true);
    let document = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Sysml).unwrap();
    let multiplicity = document.elements.iter().find(|e| e.kind == "SysML::Multiplicity").unwrap();
    assert_eq!(emit::generated_effective_names(&document.elements, multiplicity).unwrap(), [None,None]);
    let paths = containment_paths(&document).0;
    let usage = document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!("x"))).unwrap();
    let path = &paths[usage.id.as_str()];
    let observed = case["nodes"].as_array().unwrap().iter().find(|e| e["path"] == *path).unwrap();
    // Native naming/linking is now supported. The independent getter comparison
    // reveals the separate Usage.mayTimeVary algorithm; do not count this model
    // as matching or turn the observed value into a source-specific default.
    assert_eq!(attribute_value(&document.elements, usage, "is_variable"), json!(false));
    assert_eq!(observed["attributes"]["is_variable"], json!(true));
    assert_eq!(document.metadata["semantic_validation"], "not_assessed");
    assert!(document.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
}

#[test]
fn definition_specialization_matches_independent_pilot() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json"
    )).unwrap();
    let controls = artifact["conformance_controls"].as_array().unwrap();
    assert_eq!(controls.len(), 4);
    for control in controls {
        let document = parse_and_link(control["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
        let before = serde_json::to_value(&document).unwrap();
        for pair in control["pairs"].as_array().unwrap() {
            let endpoint = |field: &str| document.elements.iter().find(|e|
                e.properties.get("declared_name") == Some(&pair[field])).unwrap();
            assert_eq!(super::specializes(&document, &endpoint("subtype").id, &endpoint("supertype").id).unwrap(),
                pair["specializes"].as_bool().unwrap(), "{pair}");
        }
        assert_eq!(before, serde_json::to_value(&document).unwrap());
    }
}

#[test]
fn definition_specialization_preserves_unknown_provider_boundary() {
    let mut document = parse_and_link("package P { part def A; part x : A; }", SourceLanguage::Sysml).unwrap();
    let named = |name: &str| document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!(name))).unwrap().id.clone();
    let usage = named("x"); let definition = named("A"); let package = named("P");
    assert_eq!(super::specializes(&document, &usage, &usage).unwrap(), true);
    // Explicit typing proves this positive path without completing PartUsage's
    // implicit provider; unrelated negative ancestry still remains unknown.
    assert!(super::specializes(&document, &usage, &definition).unwrap());
    assert!(super::specializes(&document, &usage, &package).is_err());
    assert!(super::specializes(&document, &definition, &package).is_err());
    assert!(super::specializes(&document, "missing", &definition).is_err());
    document.elements.push(document.elements[0].clone());
    assert!(super::specializes(&document, &definition, &definition).is_err());
}

#[test]
fn definition_variability_matches_complete_pilot_inputs() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json"
    )).unwrap();
    let controls = artifact["variability_controls"].as_array().unwrap();
    assert_eq!(controls.len(), 32);
    let mut positive = 0;
    for control in controls {
        let mut document = parse_and_link(control["source"].as_str().unwrap(), SourceLanguage::Sysml).unwrap();
        // Match the explicit complete-model input supplied independently to Pilot.
        // Production queries never set these flags themselves.
        for element in &mut document.elements {
            if metaclass_conforms(&element.kind, "Type") { element.properties.insert("is_implied_included".into(), json!(true)); }
        }
        let usage = document.elements.iter_mut().find(|e| e.properties.get("declared_name") == Some(&json!("x"))).unwrap();
        usage.properties.insert("is_composite".into(), control["composite"].clone());
        usage.properties.insert("is_portion".into(), control["portion"].clone());
        let id = usage.id.clone();
        let before = serde_json::to_value(&document).unwrap();
        let answer = super::may_time_vary(&document, &id).unwrap();
        assert_eq!(answer, control["may_time_vary"].as_bool().unwrap(), "{control}");
        positive += usize::from(answer);
        assert_eq!(before, serde_json::to_value(&document).unwrap());
        let restored: KirDocument = serde_json::from_value(before).unwrap();
        assert_eq!(super::may_time_vary(&restored, &id).unwrap(), answer);
    }
    assert!(positive > 0 && positive < controls.len());
}

#[test]
fn definition_variability_preserves_missing_provider_boundary() {
    let mut document = parse_and_link("package P { item def Owner { item x; } item detached; }", SourceLanguage::Sysml).unwrap();
    let named = |name: &str| document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!(name))).unwrap().id.clone();
    let usage = named("x"); let detached = named("detached"); let package = named("P");
    let before = serde_json::to_value(&document).unwrap();
    assert!(super::may_time_vary(&document, &usage).is_err());
    assert!(!super::may_time_vary(&document, &detached).unwrap());
    assert!(super::may_time_vary(&document, &package).is_err());
    assert!(super::may_time_vary(&document, "missing").is_err());
    assert_eq!(before, serde_json::to_value(&document).unwrap());
    document.elements.push(document.elements[0].clone());
    assert!(super::may_time_vary(&document, &usage).is_err());
}

#[test]
fn definition_variation_typing_matches_independent_pilot() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json"
    )).unwrap();
    let controls = artifact["variation_controls"].as_array().unwrap();
    assert_eq!(controls.len(), 8);
    for control in controls {
        let document = parse_and_link(control["source"].as_str().unwrap(), SourceLanguage::Sysml).unwrap();
        let before = serde_json::to_value(&document).unwrap();
        let contributions = control["contributions"].as_array().unwrap();
        assert_eq!(contributions.len(), 1);
        for contribution in contributions {
            let named = |field: &str| document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&contribution[field])).unwrap();
            let specific = named("specific"); let general = named("general");
            assert!(super::specializes(&document, &specific.id, &general.id).unwrap());
            assert_eq!(contribution["relationship"], if metaclass_conforms(&general.kind, "Definition") { "FeatureTyping" } else { "Subsetting" });
            let restored: KirDocument = serde_json::from_value(before.clone()).unwrap();
            assert!(super::specializes(&restored, &specific.id, &general.id).unwrap());
        }
        assert_eq!(before, serde_json::to_value(&document).unwrap());
        assert!(document.elements.iter().all(|e| e.properties.get("is_implied_included") != Some(&json!(true))));
    }
}

#[test]
fn definition_variation_typing_does_not_complete_usage_provider() {
    for source in [
        "package P { part choice { part option; } part def Unrelated; }",
        "package P { variation part choice { variant part option; } part def Unrelated; }",
    ] {
        let document = parse_and_link(source, SourceLanguage::Sysml).unwrap();
        let named = |name: &str| document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!(name))).unwrap();
        assert!(super::specializes(&document, &named("option").id, &named("Unrelated").id).is_err());
        if !source.contains("variation") {
            assert!(super::specializes(&document, &named("option").id, &named("choice").id).is_err());
        }
    }
}

#[test]
fn definition_participant_contribution_matches_independent_pilot() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/definition-document-pilot-controls.json"
    )).unwrap();
    let controls = artifact["participant_controls"].as_array().unwrap();
    assert_eq!(controls.len(), 6);
    for control in controls {
        let document = parse_and_link(control["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
        let before = serde_json::to_value(&document).unwrap();
        let named = |name: &str| document.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!(name))).unwrap();
        let candidate = named("candidate"); let participant = named("participant");
        let answer = super::specializes(&document, &candidate.id, &participant.id);
        if control["contributions"].as_array().unwrap().is_empty() {
            assert!(answer.is_err(), "An absent contribution does not complete the provider");
        } else {
            assert_eq!(control["contributions"], json!(["participant"]));
            assert!(answer.unwrap());
            let restored: KirDocument = serde_json::from_value(before.clone()).unwrap();
            assert!(super::specializes(&restored, &candidate.id, &participant.id).unwrap());
        }
        assert_eq!(before, serde_json::to_value(&document).unwrap());
    }
}

#[test]
fn definition_participant_contribution_rejects_missing_dependencies() {
    let source = "standard library package Links { assoc Link { feature participant; } } package P { assoc Owner { end feature candidate; } }";
    let base = parse_and_link(source, SourceLanguage::Kerml).unwrap();
    let named = |name: &str| base.elements.iter().find(|e| e.properties.get("declared_name") == Some(&json!(name))).unwrap().id.clone();
    let candidate = named("candidate"); let participant = named("participant"); let owner = named("Owner"); let library = named("Links");
    for mutation in ["library", "relationships", "connector"] {
        let mut document = base.clone();
        if mutation == "library" {
            document.elements.iter_mut().find(|e| e.id == library).unwrap().properties.insert("is_standard".into(), json!(false));
        } else if mutation == "relationships" {
            document.elements.iter_mut().find(|e| e.id == candidate).unwrap().properties.insert("owned_relationship".into(), json!(["missing"]));
        } else {
            document.elements.iter_mut().find(|e| e.id == owner).unwrap().kind = "SysML::Connector".into();
        }
        let before = serde_json::to_value(&document).unwrap();
        assert!(super::specializes(&document, &candidate, &participant).is_err(), "{mutation}");
        assert_eq!(before, serde_json::to_value(&document).unwrap());
    }
}


#[test]
fn definition_binding_stored_subtrees_identify_lifecycle_gaps() {
    let artifact: Value = serde_json::from_str(include_str!(
        "../resources/metamodels/sysml-2.0-pilot-2026-08/compatibility.extract.json")).unwrap();
    for complete in [false,true] {
    for case in artifact["binding_stages"].as_array().unwrap() {
        let mut document = parse_and_link(case["source"].as_str().unwrap(), SourceLanguage::Kerml).unwrap();
        let named = |name: &str| document.elements.iter()
            .find(|e| e.properties.get("declared_name") == Some(&json!(name))).unwrap().id.clone();
        let owner = named(case["owner"].as_str().unwrap());
        let source = named("x"); let target = named(case["target"].as_str().unwrap());
        if complete {
            let other=named("y");
            super::complete_ordinary_features(&mut document,&[(&source,"x.complete"),(&other,"y.complete")]).unwrap();
            super::materialize_complete_binding(&mut document,&owner,"binding",&source,&target).unwrap();
        }
        else { super::materialize_binding_defaults(&mut document, &owner, "binding", &source, &target).unwrap(); }
        let exported = export_sysml_abstract_syntax_value(&document, Default::default()).unwrap();
        assert!(!exported.has_errors());
        let imported = import_sysml_abstract_syntax_value(exported.value, Default::default()).unwrap();
        assert!(!imported.has_errors());
        for doc in [&document, &imported.persistable_document().unwrap()] {
            let issues = super::assess_implied_inclusion(doc).unwrap();
            assert_eq!(issues.iter().map(|i| i.element_id.as_str()).collect::<BTreeSet<_>>(),
                if complete {BTreeSet::new()} else {BTreeSet::from(["binding", "binding.end.0", "binding.end.1"])});
            // The source Type owns an explicit membership, which contains the
            // implied binding. The invariant concerns direct ownedRelationships;
            // it must not incorrectly propagate to every containing ancestor.
            let (paths, by_path) = containment_paths(doc);
            let root = if complete {"$"}else{paths["binding.membership"].as_str()};
            let rows = case[if complete {"stored_document"}else{"stored_subtree"}].as_array().unwrap();
            let inside = |path: &str| path == root || path.starts_with(&format!("{root}/"));
            assert_eq!(by_path.keys().filter(|p| inside(p)).count(), rows.len());
            let mut completion_differences = 0;
            let mut partial_endpoint_projections = 0;
            for row in rows {
                let path = format!("{}{}", root, &row["path"].as_str().unwrap()[1..]);
                let element = by_path[&path];
                assert_eq!(element.kind.rsplit("::").next().unwrap(), row["kind"]);
                let mut attributes = row["attributes"].as_object().unwrap().keys().cloned().collect::<BTreeSet<_>>();
                let mut references = row["references"].as_object().unwrap().keys().cloned().collect::<BTreeSet<_>>();
                for field in element.properties.keys() {
                    let Some(contract) = ecore_model::feature(&element.kind, field) else { continue; };
                    if contract.derived || contract.transient || contract.volatile || field == "element_id" { continue; }
                    match contract.kind {
                        ecore_model::FeatureKind::Attribute => { attributes.insert(field.clone()); }
                        ecore_model::FeatureKind::Reference => { references.insert(field.clone()); }
                    }
                }
                let mut actual = serde_json::Map::new();
                for field in attributes {
                    let value = attribute_value(&doc.elements, element, &field);
                    if !value.is_null() { actual.insert(field, value); }
                }
                let mut expected = row["attributes"].as_object().unwrap().clone();
                if !complete && metaclass_conforms(&element.kind, "Type") {
                    assert_eq!(actual.remove("is_implied_included"), Some(json!(false)));
                    assert_eq!(expected.remove("is_implied_included"), Some(json!(true)));
                    completion_differences += 1;
                }
                assert_eq!(actual, expected, "Stored attributes at {path}");
                let mut actual = serde_json::Map::new();
                for field in references {
                    if !complete && element.kind.rsplit("::").next() == Some("BindingConnector") && matches!(field.as_str(), "source" | "target") {
                        // Imported binary-default assessment now enables these
                        // partial-state getters. Compare their exact endpoints to
                        // the independent subtree below. All three lifecycle flags
                        // remain false and retain their separate gap assertions.
                        assert_eq!(row["references"][&field].as_array().unwrap().len(), 1);
                        partial_endpoint_projections += 1;
                    }
                    let contract = ecore_model::redefined_feature(&element.kind, &field).unwrap();
                    let targets = if !contract.derived && !contract.volatile && !contract.transient
                        && !element.properties.contains_key(contract.field) {
                        // Optional absent stored slots compare as empty. An expected
                        // nonempty endpoint still fails the exact comparison below.
                        assert_eq!(contract.lower, 0);
                        Vec::new()
                    } else { super::reference_targets(doc, &element.id, &field).unwrap_or_else(|e| panic!("{path}.{field}: {e}")) };
                    let names = targets.iter().map(|target| {
                        let path = &paths[target.id.as_str()];
                        if inside(path) { format!("${}", &path[root.len()..]) }
                        else {
                            // All external controls have explicit ordinary qualified names.
                            let mut ancestry = by_path.iter().filter(|(p, _)| path == *p || path.starts_with(&format!("{p}/")))
                                .collect::<Vec<_>>();
                            ancestry.sort_by_key(|(p, _)| p.len());
                            let names = ancestry.iter().filter_map(|(_, e)| e.properties.get("declared_name").and_then(Value::as_str)).collect::<Vec<_>>();
                            assert!(!names.is_empty());
                            format!("external:{}", names.join("::"))
                        }
                    }).collect::<Vec<_>>();
                    actual.insert(field, json!(names));
                }
                let expected = row["references"].as_object().unwrap().clone();
                assert_eq!(actual, expected, "Stored references at {path}");
            }
            assert_eq!(completion_differences, if complete {0}else{3});
            assert_eq!(partial_endpoint_projections, if complete {0}else{2});
        }
    }
}

}
