//! Effective EMF default literals for native source-model construction.
//! The generated table is pinned; explicit KIR values always win.
use mercurio_foundation::kir::KirElement;
use serde_json::Value;

use super::ecore_model;
use super::relationship_declarations::metaclass_conforms;

#[path = "ecore_defaults_generated.rs"]
mod generated;

#[path = "ecore_constructor_constants_generated.rs"]
mod constructors;
#[path="ecore_nullable_defaults_generated.rs"]
mod nullable_constructors;
#[path="ecore_owner_nullable_defaults_generated.rs"]
mod owner_nullable_constructors;

/// Bounded javac-resolved constant assignments, separate from Ecore literals.
pub(crate) fn constructor_attribute(kind: &str, field: &str) -> Option<&'static str> {
    let kind = kind.rsplit("::").next().unwrap_or(kind);
    constructors::VALUES
        .iter()
        .find(|(owner, name, _)| *owner == kind && *name == field)
        .map(|(_, _, value)| *value)
}

/// Read a stored attribute without materializing defaults into the model.
/// Imported literals/primitive defaults supply absent values only where no
/// known constructor or delegate dependency changes that interpretation.
pub(crate) fn read_attribute(element: &KirElement, field: &str) -> Result<Value, String> {
    let contract = ecore_model::feature(&element.kind, field)
        .ok_or_else(|| format!("Unknown Ecore attribute {}.{field}", element.kind))?;
    if contract.kind != ecore_model::FeatureKind::Attribute || contract.derived || contract.volatile
    {
        return Err(format!(
            "Ecore attribute {}.{field} requires a semantic service",
            element.kind
        ));
    }
    if let Some(value) = element.properties.get(field) {
        ecore_model::validate_value(&element.kind, field, value)?;
        return Ok(value.clone());
    }
    if let Some(value) = constructor_attribute(&element.kind, field) {
        let value = Value::String(value.into());
        ecore_model::validate_value(&element.kind, field, &value)?;
        return Ok(value);
    }
    let kind = element.kind.rsplit("::").next().unwrap_or(&element.kind);
    if nullable_constructors::NULL_DEFAULTS.contains(&(kind,field)) || owner_nullable_constructors::NULL_DEFAULTS.contains(&(kind,field)) {
        ecore_model::validate_value(&element.kind,field,&Value::Null)?;
        return Ok(Value::Null);
    }
    if generated::DEFAULT_READ_DEPENDENCIES.contains(&(kind, field)) {
        return Err(format!(
            "Ecore attribute {kind}.{field} requires a constructor/delegate service"
        ));
    }
    let value = if let Some((_, _, literal)) = generated::DEFAULTS
        .iter()
        .find(|(owner, candidate, _)| *candidate == field && metaclass_conforms(kind, owner))
    {
        match *literal {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            value => Value::String(value.into()),
        }
    } else if let Some((_, _, literal)) = generated::PRIMITIVE_DEFAULTS
        .iter()
        .find(|(owner, candidate, _)| *candidate == field && metaclass_conforms(kind, owner))
    {
        serde_json::from_str(literal).map_err(|e| format!("Invalid generated default: {e}"))?
    } else {
        return Err(format!(
            "No qualified absent-value policy for {kind}.{field}"
        ));
    };
    ecore_model::validate_value(&element.kind, field, &value)?;
    Ok(value)
}

pub(crate) fn apply(elements: &mut [KirElement]) {
    for element in elements {
        for &(owner, field, literal) in generated::DEFAULTS {
            if !element.properties.contains_key(field) && metaclass_conforms(&element.kind, owner) {
                let value = match literal {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    other => Value::String(other.into()),
                };
                element.properties.insert(field.into(), value);
            }
        }
    }
}

/// Effective primitive defaults for generated object construction. Enum defaults
/// are deliberately excluded: upstream constructors may override EMF's first
/// literal (Feature.direction, for example, is null on a fresh Pilot Feature).
pub(crate) fn apply_implicit_primitives(elements: &mut [KirElement]) -> Result<(), String> {
    for element in elements {
        for &(owner, field, literal) in generated::PRIMITIVE_DEFAULTS {
            if !element.properties.contains_key(field) && metaclass_conforms(&element.kind, owner) {
                let value: Value = serde_json::from_str(literal).map_err(|e| e.to_string())?;
                ecore_model::validate_value(&element.kind, field, &value)?;
                element.properties.insert(field.into(), value);
            }
        }
    }
    Ok(())
}

/// Promote attached Pilot getter observations for every explicit Ecore default.
/// An observed value can differ from the literal when generated or delegated
/// Pilot behavior overrides it. Missing observations fail the marked import.
pub fn promote_observed_library_defaults(elements: &mut [KirElement]) -> Result<usize, String> {
    let mut promoted = 0;
    for element in elements {
        // Foundation adds KIR descriptors for exported scalar metadata after
        // Pilot normalization. They are not Pilot EObjects or Ecore classes.
        if element.kind == "MetamodelFeature" && element.id.starts_with("metafeature.") {
            continue;
        }
        if !metaclass_conforms(&element.kind, "Element") {
            return Err(format!(
                "{} has unknown pinned Ecore class {}",
                element.id, element.kind
            ));
        }
        for &(owner, field, literal) in generated::DEFAULTS {
            if !metaclass_conforms(&element.kind, owner) {
                continue;
            }
            let contract = ecore_model::feature(&element.kind, field).ok_or_else(|| {
                format!(
                    "{} ({}) lacks pinned Ecore feature {field}",
                    element.id, element.kind
                )
            })?;
            if contract.default_literal != Some(literal) {
                return Err(format!(
                    "{} ({}) has mismatched Ecore default {field}",
                    element.id, element.kind
                ));
            }
            let observed = element
                .properties
                .get("metadata")
                .and_then(Value::as_object)
                .and_then(|metadata| metadata.get(field))
                .ok_or_else(|| {
                    format!(
                        "{} ({}) lacks observed Ecore attribute {field}",
                        element.id, element.kind
                    )
                })?
                .clone();
            ecore_model::validate_value(&element.kind, field, &observed)
                .map_err(|reason| format!("{} ({}): {reason}", element.id, element.kind))?;
            if let Some(existing) = element.properties.get(field) {
                if existing != &observed {
                    return Err(format!(
                        "{} ({}) has conflicting Ecore attribute {field}: {existing} versus {observed}",
                        element.id, element.kind
                    ));
                }
            } else {
                element.properties.insert(field.into(), observed);
                promoted += 1;
            }
        }
    }
    Ok(promoted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn definition_nullable_portion_reads_match_resolved_initializers_and_pilot() {
        let evidence:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/occurrence-contribution-pilot-controls.json"))).unwrap();
        for (kind,observed) in evidence["absent_portion_values"].as_object().unwrap() {
            let mut element=KirElement {id:"usage".into(),kind:format!("SysML::{kind}"),layer:2,properties:BTreeMap::new()};
            assert_eq!(read_attribute(&element,"portion_kind").unwrap(),*observed);assert!(element.properties.is_empty());
            for value in [serde_json::json!("snapshot"),serde_json::json!("timeslice"),Value::Null] {
                element.properties.insert("portion_kind".into(),value.clone());assert_eq!(read_attribute(&element,"portion_kind").unwrap(),value);
            }
            element.properties.insert("portion_kind".into(),serde_json::json!("unknown"));assert!(read_attribute(&element,"portion_kind").is_err());
        }
        let unsupported=KirElement{id:"action".into(),kind:"SysML::ConnectionUsage".into(),layer:2,properties:BTreeMap::new()};assert!(read_attribute(&unsupported,"portion_kind").is_err());
    }

    #[test]
    fn definition_action_state_transition_nullable_portions_follow_resolved_constructors() {
        let evidence:Value=serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/conformance/2026-08-support/owner-typing-pilot-controls.json"))).unwrap();
        for (kind,observed) in evidence["absent_portion_values"].as_object().unwrap() {
            assert_eq!(*observed,Value::Bool(true));
            let mut element=KirElement{id:"receiver".into(),kind:format!("SysML::{kind}"),layer:2,properties:BTreeMap::new()};
            assert_eq!(read_attribute(&element,"portion_kind").unwrap(),Value::Null);assert!(element.properties.is_empty());
            for value in [Value::Null,serde_json::json!("snapshot"),serde_json::json!("timeslice")] {element.properties.insert("portion_kind".into(),value.clone());assert_eq!(read_attribute(&element,"portion_kind").unwrap(),value);}
            element.properties.insert("portion_kind".into(),serde_json::json!("invalid"));assert!(read_attribute(&element,"portion_kind").is_err());
        }
    }
    #[test]
    fn resolved_constructor_reads_match_runtime_and_preserve_explicit_state() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../resources/metamodels/sysml-2.0-pilot-2026-08/constructor-constants.extract.json"
        )).unwrap();
        for control in evidence["controls"].as_array().unwrap() {
            let mut element = KirElement {
                id: "expression".into(),
                kind: control["class"].as_str().unwrap().into(),
                layer: 2,
                properties: BTreeMap::new(),
            };
            assert_eq!(
                read_attribute(&element, "operator").unwrap(),
                control["observed_value"]
            );
            assert!(element.properties.is_empty());
            element
                .properties
                .insert("operator".into(), serde_json::json!("explicit"));
            assert_eq!(
                read_attribute(&element, "operator").unwrap(),
                serde_json::json!("explicit")
            );
            element.properties.insert("operator".into(), Value::Null);
            assert!(read_attribute(&element, "operator").is_err());
        }
        assert!(constructor_attribute("OperatorExpression", "operator").is_none());
    }

    #[test]
    fn attribute_reads_use_imported_defaults_preserve_storage_and_refuse_dependencies() {
        let make = |kind: &str| KirElement {
            id: kind.into(),
            kind: format!("SysML::{kind}"),
            layer: 2,
            properties: BTreeMap::new(),
        };
        for (kind, field, expected) in [
            (
                "NamespaceImport",
                "visibility",
                serde_json::json!("private"),
            ),
            (
                "OwningMembership",
                "visibility",
                serde_json::json!("public"),
            ),
            ("MembershipImport", "is_recursive", serde_json::json!(false)),
            ("NamespaceImport", "is_import_all", serde_json::json!(false)),
            ("Feature", "is_unique", serde_json::json!(true)),
            ("LiteralInteger", "value", serde_json::json!(0)),
        ] {
            let mut element = make(kind);
            assert_eq!(read_attribute(&element, field).unwrap(), expected);
            assert!(element.properties.is_empty());
            element
                .properties
                .insert(field.into(), Value::Array(vec![]));
            assert!(read_attribute(&element, field).is_err());
        }
        for (kind, field) in [
            ("NamespaceExpose", "is_import_all"),
            ("PartUsage", "is_variable"),
            ("Feature", "direction"),
            ("Package", "owned_relationship"),
        ] {
            assert!(
                read_attribute(&make(kind), field).is_err(),
                "{kind}.{field}"
            );
        }
        let mut element = make("NamespaceImport");
        element
            .properties
            .insert("visibility".into(), serde_json::json!("public"));
        let saved = element.clone();
        assert_eq!(read_attribute(&element, "visibility").unwrap(), "public");
        assert_eq!(element, saved);
    }

    #[test]
    fn implicit_primitives_preserve_assignments_and_exclude_enum_defaults() {
        let mut elements: Vec<_> = [
            "PartDefinition",
            "PartUsage",
            "LiteralBoolean",
            "LiteralInteger",
            "LiteralRational",
            "Feature",
        ]
        .into_iter()
        .map(|kind| KirElement {
            id: kind.into(),
            kind: format!("SysML::{kind}"),
            layer: 2,
            properties: BTreeMap::new(),
        })
        .collect();
        elements[0]
            .properties
            .insert("is_variation".into(), Value::Bool(true));
        apply_implicit_primitives(&mut elements).unwrap();
        assert_eq!(elements[0].properties["is_variation"], true);
        assert_eq!(elements[1].properties["is_variation"], false);
        assert_eq!(elements[2].properties["value"], false);
        assert_eq!(elements[3].properties["value"], 0);
        assert_eq!(elements[4].properties["value"].as_f64(), Some(0.0));
        assert!(!elements[5].properties.contains_key("direction"));
        assert!(!elements[1].properties.contains_key("portion_kind"));
        let saved = elements.clone();
        apply_implicit_primitives(&mut elements).unwrap();
        assert_eq!(elements, saved);
    }

    #[test]
    fn inherited_defaults_preserve_explicit_values_and_other_classes() {
        let mut elements = vec![
            KirElement {
                id: "feature".into(),
                kind: "SysML::PayloadFeature".into(),
                layer: 2,
                properties: BTreeMap::from([("is_unique".into(), Value::Bool(false))]),
            },
            KirElement {
                id: "import".into(),
                kind: "SysML::NamespaceImport".into(),
                layer: 2,
                properties: BTreeMap::from([("visibility".into(), Value::String("public".into()))]),
            },
            KirElement {
                id: "relationship".into(),
                kind: "SysML::Dependency".into(),
                layer: 2,
                properties: BTreeMap::new(),
            },
        ];
        apply(&mut elements);
        assert_eq!(elements[0].properties["is_unique"], false);
        assert_eq!(elements[0].properties["is_ordered"], false);
        assert_eq!(elements[0].properties["is_implied_included"], false);
        assert_eq!(elements[1].properties["visibility"], "public");
        assert_eq!(elements[1].properties["is_import_all"], false);
        assert_eq!(elements[2].properties["is_implied"], false);
        assert!(!elements[2].properties.contains_key("is_unique"));
    }

    #[test]
    fn library_observations_override_literals_and_require_complete_evidence() {
        let mut element = KirElement {
            id: "library-feature".into(),
            kind: "Feature".into(),
            layer: 2,
            properties: BTreeMap::new(),
        };
        let mut metadata = serde_json::Map::new();
        for &(owner, field, literal) in generated::DEFAULTS {
            if metaclass_conforms(&element.kind, owner) {
                metadata.insert(
                    field.into(),
                    match literal {
                        "true" => Value::Bool(true),
                        "false" => Value::Bool(false),
                        value => Value::String(value.into()),
                    },
                );
            }
        }
        metadata.insert("is_variable".into(), Value::Bool(true));
        element
            .properties
            .insert("metadata".into(), Value::Object(metadata));
        let mut elements = vec![element];
        assert!(promote_observed_library_defaults(&mut elements).unwrap() > 0);
        assert_eq!(elements[0].properties["is_variable"], true);
        assert_eq!(elements[0].properties["is_unique"], true);
        assert_eq!(promote_observed_library_defaults(&mut elements).unwrap(), 0);
        elements[0]
            .properties
            .get_mut("metadata")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("is_variable".into(), Value::String("true".into()));
        assert!(
            promote_observed_library_defaults(&mut elements)
                .unwrap_err()
                .contains("invalid")
        );
        elements[0]
            .properties
            .get_mut("metadata")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("is_variable".into(), Value::Bool(false));
        assert!(
            promote_observed_library_defaults(&mut elements)
                .unwrap_err()
                .contains("conflicting")
        );
        elements[0]
            .properties
            .get_mut("metadata")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("is_variable");
        assert!(
            promote_observed_library_defaults(&mut elements)
                .unwrap_err()
                .contains("is_variable")
        );
        elements[0].kind = "NewUnreviewedClass".into();
        assert!(
            promote_observed_library_defaults(&mut elements)
                .unwrap_err()
                .contains("unknown pinned Ecore class")
        );
        elements[0].kind = "MetamodelFeature".into();
        elements[0].id = "metafeature.library-feature.is_variable".into();
        assert_eq!(promote_observed_library_defaults(&mut elements).unwrap(), 0);
    }
}
