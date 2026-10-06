//! Native export boundary for the bounded definition-driven core preview.
//! Semantic transformation/validation and release qualification remain separate.
use mercurio_sysml::abstract_syntax_json::{
    export_sysml_abstract_syntax_value, import_sysml_abstract_syntax_value,
};
use mercurio_sysml::definition_document::{parse_and_link, DefinitionDocumentError};
use mercurio_sysml::{KirDocument, SourceLanguage};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::io::Write;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Spec {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    case_id: String,
    language: Language,
    source: String,
    #[serde(default)]
    reference_queries: Vec<ReferenceQuery>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceQuery {
    owner_path: String,
    field: String,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Language {
    Kerml,
    Sysml,
}
impl Language {
    fn native(self) -> SourceLanguage {
        match self {
            Self::Kerml => SourceLanguage::Kerml,
            Self::Sysml => SourceLanguage::Sysml,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Kerml => "kerml",
            Self::Sysml => "sysml",
        }
    }
}

pub(super) fn export(spec_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let spec: Spec = serde_json::from_str(&std::fs::read_to_string(spec_path)?)?;
    let mut identities = BTreeSet::new();
    if spec.cases.is_empty()
        || spec
            .cases
            .iter()
            .any(|case| case.case_id.trim().is_empty() || !identities.insert(case.case_id.as_str()))
    {
        return Err("core preview requires nonempty, distinct case identities".into());
    }
    let mut output = std::io::BufWriter::new(std::fs::File::create(output_path)?);
    for case in &spec.cases {
        serde_json::to_writer(&mut output, &export_case(case))?;
        writeln!(output)?;
    }
    output.flush()?;
    Ok(())
}

fn export_case(case: &Case) -> Value {
    let mut row = json!({
        "schema": "dev.mercurio.definition-core-preview-model.v1",
        "case_id": case.case_id, "language": case.language.name(),
        "definition_profile": "sysml-2.0-pilot-2026-08",
        "qualification_certificate": false, "candidate_promoted": false,
        "semantic_validation": "not_assessed", "transformation_completion": "not_assessed"
    });
    match parse_and_link(&case.source, case.language.native()) {
        Err(error) => {
            let stage = match &error {
                DefinitionDocumentError::Syntax(_) => "syntax",
                DefinitionDocumentError::Lexical(_) => "lexical",
                DefinitionDocumentError::Unsupported(_) => "unsupported",
                DefinitionDocumentError::ResourceLimit(_) => "resource_limit",
                DefinitionDocumentError::Artifact(_) => "artifact",
                DefinitionDocumentError::ConstructionOrLinking(_) => "construction_or_linking",
            };
            row["status"] = json!("blocked");
            row["stage"] = json!(stage);
            row["error"] = json!(error.to_string());
        }
        Ok(document) => match export_and_restore(&document) {
            Err(error) => {
                row["status"] = json!("blocked");
                row["stage"] = json!("persistence");
                row["error"] = json!(error);
            }
            Ok((abstract_syntax, restored)) => {
                let views =
                    reference_views(&document, &case.reference_queries).and_then(|original| {
                        Ok((
                            original,
                            reference_views(&restored, &case.reference_queries)?,
                        ))
                    });
                let (original_views, restored_views) = match views {
                    Ok(views) => views,
                    Err(error) => {
                        row["status"] = json!("blocked");
                        row["stage"] = json!("reference_projection");
                        row["error"] = json!(error);
                        return row;
                    }
                };
                row["reference_views"] = original_views;
                row["roundtrip_reference_views"] = restored_views;
                row["status"] = json!("core_constructed");
                row["stage"] = json!("stored_structure_and_supported_linking");
                row["element_count"] = json!(document.elements.len());
                row["kir_document"] = json!(document);
                row["abstract_syntax"] = abstract_syntax;
                row["roundtrip_kir_document"] = json!(restored);
                row["kir_roundtrip"] = json!("exact");
            }
        },
    }
    row
}

/// Read caller-selected canonical containment paths through the existing
/// native reference service. Requests contain no target/value expectations.
fn reference_views(document: &KirDocument, requests: &[ReferenceQuery]) -> Result<Value, String> {
    let roots = document
        .elements
        .iter()
        .filter(|element| {
            element.kind.rsplit("::").next() == Some("Namespace")
                && element
                    .properties
                    .get("owning_relationship")
                    .map_or(true, Value::is_null)
        })
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return Err("expected exactly one source RootNamespace".into());
    }
    let mut result = Vec::new();
    for request in requests {
        let mut owner = roots[0];
        let suffix = request
            .owner_path
            .strip_prefix('$')
            .ok_or("invalid canonical containment path")?;
        if !suffix.is_empty() {
            let parts = suffix
                .strip_prefix('/')
                .ok_or("invalid canonical containment path")?
                .split('/')
                .collect::<Vec<_>>();
            if parts.len() % 2 != 0 {
                return Err("invalid canonical containment path".into());
            }
            for pair in parts.chunks_exact(2) {
                let ordinal = pair[1].parse::<usize>().map_err(|e| e.to_string())?;
                let value = owner
                    .properties
                    .get(pair[0])
                    .ok_or("missing canonical containment field")?;
                let identity = match value {
                    Value::Array(values) => values.get(ordinal).and_then(Value::as_str),
                    Value::String(id) if ordinal == 0 => Some(id.as_str()),
                    _ => None,
                }
                .ok_or("invalid canonical containment ordinal")?;
                owner = document
                    .elements
                    .iter()
                    .find(|e| e.id == identity)
                    .ok_or("unclosed canonical containment")?;
            }
        }
        let targets = mercurio_sysml::definition_document::reference_targets(
            document,
            &owner.id,
            &request.field,
        )
        .map_err(|e| e.to_string())?;
        result.push(
            json!({"owner_path": request.owner_path, "field": request.field,
            "target_ids": targets.iter().map(|e| e.id.as_str()).collect::<Vec<_>>()}),
        );
    }
    Ok(json!(result))
}

fn export_and_restore(document: &KirDocument) -> Result<(Value, KirDocument), String> {
    let serialized = serde_json::to_vec(document).map_err(|e| e.to_string())?;
    let fresh: KirDocument = serde_json::from_slice(&serialized).map_err(|e| e.to_string())?;
    if serde_json::to_value(&fresh).map_err(|e| e.to_string())?
        != serde_json::to_value(document).map_err(|e| e.to_string())?
    {
        return Err("fresh KIR persistence changed the document".into());
    }
    let exported = export_sysml_abstract_syntax_value(&fresh, Default::default())
        .map_err(|e| e.to_string())?;
    if exported.has_errors() {
        return Err(format!(
            "abstract-syntax export: {:?}",
            exported.diagnostics
        ));
    }
    let imported = import_sysml_abstract_syntax_value(exported.value.clone(), Default::default())
        .map_err(|e| e.to_string())?;
    if imported.has_errors() {
        return Err(format!(
            "abstract-syntax import: {:?}",
            imported.diagnostics
        ));
    }
    let restored = imported.persistable_document().map_err(|e| e.to_string())?;
    Ok((exported.value, restored))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn core_preview_exports_native_models_without_semantic_completion() {
        let row = export_case(&Case {
            case_id: "typing".into(),
            language: Language::Kerml,
            source: "package P { class A; feature x : A; }".into(),
            reference_queries: vec![ReferenceQuery {
                owner_path: "$".into(),
                field: "owned_relationship".into(),
            }],
        });
        assert_eq!(row["status"], "core_constructed");
        assert_eq!(row["kir_roundtrip"], "exact");
        assert_eq!(
            row["reference_views"][0]["target_ids"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            row["roundtrip_reference_views"].as_array().unwrap().len(),
            1
        );
        assert!(row["abstract_syntax"].is_array() || row["abstract_syntax"].is_object());
        assert_eq!(
            row["kir_document"]["metadata"]["semantic_validation"],
            "not_assessed"
        );
        assert_eq!(row["semantic_validation"], "not_assessed");
        assert_eq!(row["transformation_completion"], "not_assessed");
        assert_eq!(row["qualification_certificate"], false);
        assert_eq!(row["candidate_promoted"], false);
        assert_eq!(
            row["kir_document"]["elements"].as_array().unwrap().len(),
            row["roundtrip_kir_document"]["elements"]
                .as_array()
                .unwrap()
                .len()
        );
    }
    #[test]
    fn core_preview_failures_keep_stage_and_never_export_partial_models() {
        for (source, stage) in [
            ("package P {", "syntax"),
            (
                "package P { feature x : Missing; }",
                "construction_or_linking",
            ),
        ] {
            let row = export_case(&Case {
                case_id: "negative".into(),
                language: Language::Kerml,
                source: source.into(),
                reference_queries: Vec::new(),
            });
            assert_eq!(row["status"], "blocked");
            assert_eq!(row["stage"], stage);
            for field in ["kir_document", "abstract_syntax", "roundtrip_kir_document"] {
                assert!(
                    row.get(field).is_none(),
                    "a failed stage must not export {field}"
                );
            }
            assert_eq!(row["qualification_certificate"], false);
        }
    }
}
