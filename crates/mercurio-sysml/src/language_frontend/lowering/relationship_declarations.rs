//! Curated AST-to-KIR bridge for explicit KerML relationships.
//! Operands use the shared resolver; the private AST payload never enters KIR.
use super::ir::ResolvedUsage;
use mercurio_foundation::{
    kir::KirElement,
    language_contracts::{ast::SourceSpan, diagnostics::Diagnostic},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::OnceLock};

const KEY: &str = "__relationship_endpoints";
#[derive(Serialize, Deserialize)]
pub(crate) struct Endpoints<T> {
    pub sources: Vec<T>,
    pub targets: Vec<T>,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct Operand<T> {
    pub steps: Vec<T>,
    pub type_ref: Option<String>,
}
#[derive(Deserialize)]
struct Rule {
    source_fields: Vec<String>,
    target_fields: Vec<String>,
    many: bool,
}
#[derive(Deserialize)]
struct Rules {
    rules: BTreeMap<String, Rule>,
}
fn rule(construct: &str) -> Result<Option<&'static Rule>, Diagnostic> {
    static RULES: OnceLock<Result<Rules, String>> = OnceLock::new();
    let rules = RULES
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../resources/kernel/relationship-declarations.overlay.json"
            ))
            .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| Diagnostic::new(e.clone(), None))?;
    Ok(rules.rules.get(construct))
}
pub(crate) fn has_direct_relationship_body(kind: &str) -> Result<bool, Diagnostic> {
    Ok(rule(kind.rsplit("::").next().unwrap_or(kind))?.is_some())
}
/// Namespace bodies own annotating members; all other Element owners use
/// Annotation. In particular Connector is both Relationship and Namespace.
#[path = "ecore_hierarchy_generated.rs"]
mod ecore_hierarchy_generated;

pub(crate) fn metaclass_conforms(kind: &str, base: &str) -> bool {
    // KIR still accepts legacy package-qualified names. Class ancestry itself
    // is generated from resolved EMF, with unknown classes failing closed.
    let name = kind.rsplit("::").next().unwrap_or(kind);
    ecore_hierarchy_generated::conforms(name, base)
}

pub(crate) fn metaclass_names() -> &'static [&'static str] {
    ecore_hierarchy_generated::CLASSES
}

pub(crate) fn metaclass_is(kind: &str, base: &str) -> Result<bool, Diagnostic> {
    Ok(metaclass_conforms(kind, base))
}

#[cfg(test)]
mod hierarchy_tests {
    use super::metaclass_is;
    #[test]
    fn generated_ecore_hierarchy_handles_multiple_inheritance_and_unknown_classes() {
        for base in ["Connector", "Relationship", "Namespace", "Element"] {
            assert!(metaclass_is("SysML::Connector", base).unwrap());
        }
        assert!(!metaclass_is("SysML::Class", "Relationship").unwrap());
        assert!(!metaclass_is("UnknownClass", "UnknownClass").unwrap());
        assert!(metaclass_is("SysML::Systems::RequirementUsage", "Feature").unwrap());
    }
}

pub(crate) fn uses_owned_annotation(kind: &str) -> Result<bool, Diagnostic> {
    Ok(!metaclass_is(kind, "Namespace")?)
}
pub(crate) fn load<T: DeserializeOwned>(
    map: &BTreeMap<String, String>,
    span: &SourceSpan,
) -> Result<Option<Endpoints<T>>, Diagnostic> {
    map.get(KEY)
        .map(|s| {
            serde_json::from_str(s).map_err(|e| {
                Diagnostic::new(
                    format!("invalid relationship operands: {e}"),
                    Some(span.clone()),
                )
            })
        })
        .transpose()
}
pub(crate) fn store<T: Serialize>(
    map: &mut BTreeMap<String, String>,
    endpoints: &Endpoints<T>,
    span: &SourceSpan,
) -> Result<(), Diagnostic> {
    map.insert(
        KEY.into(),
        serde_json::to_string(endpoints).map_err(|e| {
            Diagnostic::new(
                format!("invalid relationship operands: {e}"),
                Some(span.clone()),
            )
        })?,
    );
    Ok(())
}

pub(crate) fn emit_owned_feature_chain(
    owner: &mut KirElement,
    elements: &mut Vec<KirElement>,
    operand: &Operand<String>,
    side: &str,
    index: usize,
) -> Result<String, Diagnostic> {
    if operand.steps.len() < 2 {
        return Err(Diagnostic::new("feature chain requires two steps", None));
    }
    let chain_id = format!("{}.{}.{}.chain", owner.id, side, index);
    let mut properties = BTreeMap::from([
        ("chaining_feature".into(), json!(operand.steps)),
        ("owning_relationship".into(), json!(owner.id)),
    ]);
    if let Some(ty) = &operand.type_ref {
        properties.insert("type".into(), json!(ty));
    }
    let mut links = Vec::new();
    for (index, target) in operand.steps.iter().enumerate() {
        let id = format!("{chain_id}.chaining.{index}");
        let mut link = BTreeMap::from([
            ("source".into(), json!([chain_id])),
            ("target".into(), json!([target])),
            ("related_element".into(), json!([chain_id, target])),
            ("owning_related_element".into(), json!(chain_id)),
            ("chaining_feature".into(), json!(target)),
            ("is_implied".into(), json!(false)),
        ]);
        if let Some(metadata) = owner.properties.get("metadata") {
            link.insert("metadata".into(), chain_metadata(metadata, "OwnedFeatureChaining", "SysML::FeatureChaining"));
        }
        elements.push(KirElement { id: id.clone(), kind: "SysML::FeatureChaining".into(), layer: 2, properties: link });
        links.push(id);
    }
    properties.insert("owned_relationship".into(), json!(links));
    if let Some(metadata) = owner.properties.get("metadata") {
        properties.insert("metadata".into(), chain_metadata(metadata, "OwnedFeatureChain", "SysML::Feature"));
    }
    elements.push(KirElement { id: chain_id.clone(), kind: "SysML::Feature".into(), layer: 2, properties });
    super::emit::append_unique_property_ref_list(&mut owner.properties, "owned_related_element", &chain_id);
    Ok(chain_id)
}
pub(crate) fn emit(
    usage: &ResolvedUsage,
    element: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    let Some(rule) = rule(&usage.construct)? else {
        return Ok(());
    };
    let operands =
        load::<Operand<String>>(&usage.metadata_properties, &usage.span)?.ok_or_else(|| {
            Diagnostic::new("missing relationship operands", Some(usage.span.clone()))
        })?;
    let mut endpoint =
        |operand: &Operand<String>, side: &str, index: usize| -> Result<String, Diagnostic> {
            if operand.steps.is_empty() {
                return Err(Diagnostic::new(
                    "empty relationship operand",
                    Some(usage.span.clone()),
                ));
            }
            if operand.steps.len() == 1 {
                return Ok(operand.steps[0].clone());
            }
            emit_owned_feature_chain(element, elements, operand, side, index)
        };
    let mut endpoints = Endpoints {
        sources: operands
            .sources
            .iter()
            .enumerate()
            .map(|(i, p)| endpoint(p, "source", i))
            .collect::<Result<Vec<_>, _>>()?,
        targets: operands
            .targets
            .iter()
            .enumerate()
            .map(|(i, p)| endpoint(p, "target", i))
            .collect::<Result<Vec<_>, _>>()?,
    };
    if endpoints.sources.is_empty()
        || endpoints.targets.is_empty()
        || (!rule.many && (endpoints.sources.len() != 1 || endpoints.targets.len() != 1))
    {
        return Err(Diagnostic::new(
            "invalid relationship endpoint cardinality",
            Some(usage.span.clone()),
        ));
    }
    // Ecore Relationship.source/target are ordered and unique; preserve first occurrence.
    let unique = |values: &mut Vec<String>| {
        let mut seen = std::collections::BTreeSet::new();
        values.retain(|value| seen.insert(value.clone()));
    };
    unique(&mut endpoints.sources);
    unique(&mut endpoints.targets);
    for (fields, values) in [
        (&rule.source_fields, &endpoints.sources),
        (&rule.target_fields, &endpoints.targets),
    ] {
        let value = if rule.many {
            json!(values)
        } else {
            json!(values[0])
        };
        for field in fields {
            element.properties.insert(field.clone(), value.clone());
        }
    }
    element
        .properties
        .insert("source".into(), json!(endpoints.sources));
    element
        .properties
        .insert("target".into(), json!(endpoints.targets));
    let mut related: Vec<_> = endpoints
        .sources
        .iter()
        .chain(&endpoints.targets)
        .cloned()
        .collect();
    unique(&mut related);
    element
        .properties
        .insert("related_element".into(), json!(related));
    element
        .properties
        .insert("is_implied".into(), Value::Bool(false));
    Ok(())
}

/// Materialize the OwnedSubsetting called by KerML MultiplicitySubset.
/// The resolved subset target is a Feature reference, not a declared child.
pub(crate) fn emit_multiplicity_subsettings(
    usage: &ResolvedUsage,
    element: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    if usage.construct != "MultiplicitySubset" { return Ok(()); }
    let chain = load::<Operand<String>>(&usage.metadata_properties, &usage.span)?;
    if chain.is_none() && usage.subsetted_features.len() != 1 {
        return Err(Diagnostic::new("MultiplicitySubset requires one resolved subset target", Some(usage.span.clone())));
    }
    let id = format!("{}.subsetting.0", element.id);
    super::emit::append_unique_property_ref_list(&mut element.properties, "owned_relationship", &id);
    let mut properties = BTreeMap::from([
        ("owning_related_element".into(), json!(element.id)),
        ("subsetting_feature".into(), json!(element.id)),
        ("specific".into(), json!(element.id)),
        ("source".into(), json!([element.id])),
        ("is_implied".into(), Value::Bool(false)),
    ]);
    if let Some(metadata) = element.properties.get("metadata") {
        let mut metadata = metadata.clone();
        metadata["lowering"] = json!({"construct":"OwnedSubsetting","metaclass":"SysML::Subsetting"});
        properties.insert("metadata".into(), metadata);
    }
    let mut subsetting = KirElement { id, kind: "SysML::Subsetting".into(), layer: 2, properties };
    let target = if let Some(chain) = chain {
        if !chain.sources.is_empty() || chain.targets.len() != 1 || chain.targets[0].steps.len() < 2 {
            return Err(Diagnostic::new("invalid MultiplicitySubset feature chain", Some(usage.span.clone())));
        }
        emit_owned_feature_chain(&mut subsetting, elements, &chain.targets[0], "target", 0)?
    } else {
        usage.subsetted_features[0].clone()
    };
    subsetting.properties.insert("subsetted_feature".into(), json!(target));
    subsetting.properties.insert("general".into(), json!(target));
    subsetting.properties.insert("target".into(), json!([target]));
    subsetting.properties.insert("related_element".into(), json!([element.id, target]));
    super::emit::append_unique_property_ref_list(&mut element.properties, "subsetted_features", &target);
    elements.push(subsetting);
    Ok(())
}

fn chain_metadata(metadata: &Value, construct: &str, metaclass: &str) -> Value {
    let mut metadata = metadata.clone();
    if let Some(fields) = metadata.as_object_mut() {
        fields.insert(
            "lowering".into(),
            json!({"construct":construct,"metaclass":metaclass}),
        );
    }
    metadata
}
