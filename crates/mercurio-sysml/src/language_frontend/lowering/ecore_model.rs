//! Native structural-feature service backed by the pinned effective Ecore.
//!
//! Ecore supplies declarations, bounds, and target types. This module's value
//! representation and assignment checks are handwritten Rust; it does not
//! interpret EMF delegates or infer missing derived feature values.
use mercurio_foundation::kir::{KirElement, KirFieldKind, KirFieldRegistry};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use super::relationship_declarations::{metaclass_conforms, metaclass_names};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FeatureKind {
    Attribute,
    Reference,
}

/// Imported dispatch evidence only; a binding is not an implemented algorithm.
pub(crate) struct SettingDelegateContract {
    pub uri: &'static str,
    pub status: &'static str,
    pub candidates: &'static [&'static str],
    pub fallback: Option<&'static str>,
}

#[allow(dead_code)]
pub(crate) struct FeatureContract {
    pub id: &'static str,
    pub owner: &'static str,
    pub field: &'static str,
    pub name: &'static str,
    pub enum_literals: Option<&'static [&'static str]>,
    pub kind: FeatureKind,
    pub target: &'static str,
    pub lower: i32,
    pub upper: i32,
    pub ordered: bool,
    pub unique: bool,
    pub containment: bool,
    pub container: bool,
    pub derived: bool,
    pub transient: bool,
    pub volatile: bool,
    pub changeable: bool,
    pub unsettable: bool,
    pub resolve_proxies: Option<bool>,
    pub opposite: Option<&'static str>,
    pub default_literal: Option<&'static str>,
    /// Resolved EMF default, distinct from generated constructor initialization.
    pub emf_default_json: &'static str,
    pub setting_delegate: Option<SettingDelegateContract>,
    /// Direct subset references in annotation order; transitive set membership
    /// cannot substitute for the first source selected by the default delegate.
    pub subset_sources: &'static [&'static str],
}

#[path = "ecore_model_generated.rs"]
mod generated;

#[path = "ecore_required_dependencies_generated.rs"]
mod required_dependencies;

/// Returns the declared or inherited feature. Generation rejects ambiguous
/// inherited names, so there is at most one match for a known EClass.
pub(crate) fn feature(kind: &str, field: &str) -> Option<&'static FeatureContract> {
    // Build once from native tables; model import must not rescan all feature
    // declarations for every property on every element.
    type EffectiveFields = BTreeMap<&'static str, BTreeMap<&'static str, &'static FeatureContract>>;
    static EFFECTIVE: OnceLock<EffectiveFields> = OnceLock::new();
    EFFECTIVE
        .get_or_init(|| {
            metaclass_names()
                .iter()
                .map(|kind| {
                    let fields = generated::FEATURES
                        .iter()
                        .filter(|contract| metaclass_conforms(kind, contract.owner))
                        .map(|contract| (contract.field, contract))
                        .collect();
                    (*kind, fields)
                })
                .collect()
        })
        .get(kind.rsplit("::").next().unwrap_or(kind))?
        .get(field)
        .copied()
}

/// Imported first direct subset reference used by the pinned default list
/// delegate. This identifies a dependency; it does not compute its value.
pub(crate) fn default_projection_source(contract: &FeatureContract) -> Option<&'static FeatureContract> {
    let binding = contract.setting_delegate.as_ref()?;
    if contract.kind != FeatureKind::Reference || contract.upper != -1 || !contract.unique
        || !contract.derived || binding.uri != "http://www.omg.org/spec/SysML"
        || binding.status != "default_setting_delegate_fallback_source"
        || !binding.candidates.is_empty()
        || binding.fallback != Some("org.omg.sysml.delegate.setting.DefaultDerivedPropertySettingDelegate") {
        return None;
    }
    let first = contract.subset_sources.first()?;
    generated::FEATURES.iter().find(|source| source.id == *first)
}

/// Resolve a requested inherited property to its most-specific redefinition.
/// Multiple surviving branches are explicitly unsupported. Selecting a derived
/// contract does not execute its delegate or supply a stored-value fallback.
pub(crate) fn redefined_feature(
    kind: &str,
    field: &str,
) -> Result<&'static FeatureContract, String> {
    let kind = kind.rsplit("::").next().unwrap_or(kind);
    if generated::AMBIGUOUS_REDEFINED_FIELDS.contains(&(kind, field)) {
        return Err(format!(
            "Ambiguous Ecore property redefinition {kind}.{field}"
        ));
    }
    let target = generated::REDEFINED_FIELDS
        .iter()
        .find(|(owner, requested, _)| *owner == kind && *requested == field)
        .map(|(_, _, target)| *target)
        .unwrap_or(field);
    feature(kind, target).ok_or_else(|| format!("Unknown Ecore property {kind}.{field}"))
}

/// Resolved signature and invocation-selector branches; execution is separate.
pub(crate) struct LibraryNamespaceContract {
    pub owner: &'static str,
    pub target: &'static str,
    pub lower: i32,
    pub upper: i32,
    pub delegate_uri: &'static str,
    pub binding_status: &'static str,
    pub branches: &'static [(&'static str, &'static str)],
}

pub(crate) fn library_namespace_contract() -> &'static LibraryNamespaceContract {
    &generated::LIBRARY_NAMESPACE
}

/// Imported invocation signature/branches; recording these does not implement
/// any branch algorithm. Each native consumer declares its bounded strategies.
pub(crate) struct InvocationParameterContract {
    pub name: &'static str, pub target: &'static str,
    pub lower: i32, pub upper: i32, pub ordered: bool, pub unique: bool,
}
pub(crate) struct InvocationContract {
    pub name: &'static str, pub owner: &'static str, pub target: &'static str,
    pub lower: i32, pub upper: i32, pub ordered: bool, pub unique: bool,
    pub parameters: &'static [InvocationParameterContract],
    pub delegate_uri: &'static str, pub binding_status: &'static str,
    pub branches: &'static [(&'static str, &'static str)],
}
pub(crate) fn model_level_evaluable_contract() -> &'static InvocationContract {
    &generated::MODEL_LEVEL_EVALUABLE
}

/// Importing the operation selects a native dependency; it does not execute it.
pub(crate) fn direction_of_contract() -> &'static InvocationContract {
    &generated::DIRECTION_OF
}

/// The signature selects explicit native delegate algorithms, not stored defaults.
pub(crate) fn parameter_direction_contract() -> &'static InvocationContract {
    &generated::PARAMETER_DIRECTION
}

pub(crate) fn source_target_feature_contract() -> &'static InvocationContract {
    &generated::SOURCE_TARGET_FEATURE
}

/// Imported membership-union contributors under the existing reviewed ordering
/// policy. Ecore does not establish inter-subset ordering by itself.
pub(crate) fn membership_union_sources(kind: &str) -> Result<&'static [(&'static str, &'static str)], String> {
    const ORDER: &[(&str, &str)] = &[("Namespace", "imported_membership"),
        ("Namespace", "owned_membership"), ("Type", "inherited_membership")];
    let (owner, _, inputs) = generated::UNION_FIELDS.iter()
        .find(|(owner, field, _)| *field == "membership" && metaclass_conforms(kind, owner))
        .ok_or("Missing imported membership union")?;
    let contract = feature(kind, "membership").ok_or("Missing membership union feature")?;
    if *owner != "Namespace" || !contract.derived || !contract.ordered || !contract.unique
        || inputs.len() != ORDER.len() || !inputs.iter().all(|input| ORDER.contains(input)) {
        return Err("Membership union requires an unreviewed contributor/order policy".into());
    }
    Ok(ORDER)
}

fn opposite_feature(contract: &FeatureContract) -> Option<&'static FeatureContract> {
    type DeclaredFields = BTreeMap<(&'static str, &'static str), &'static FeatureContract>;
    static DECLARED: OnceLock<DeclaredFields> = OnceLock::new();
    let identity = contract.opposite?.split_once('/')?;
    DECLARED
        .get_or_init(|| {
            generated::FEATURES
                .iter()
                .map(|contract| ((contract.owner, contract.name), contract))
                .collect()
        })
        .get(&identity)
        .copied()
}

/// Resolve a stored containment's reciprocal container feature. This checks
/// imported identities/types without computing derived ownership delegates.
pub(crate) fn containment_inverse(
    kind: &str,
    field: &str,
    child: &str,
) -> Result<&'static FeatureContract, String> {
    let contract = feature(kind, field).ok_or("Unknown containment feature")?;
    if !contract.containment || contract.derived || contract.container || !contract.changeable || contract.unsettable {
        return Err(format!("Unsupported containment {kind}.{field}"));
    }
    let inverse = opposite_feature(contract).ok_or("Missing containment opposite")?;
    if !inverse.container
        || inverse.derived
        || !inverse.changeable
        || inverse.unsettable
        || inverse.opposite != Some(format!("{}/{}", contract.owner, contract.name).as_str())
    {
        return Err(format!("Nonreciprocal containment {kind}.{field}"));
    }
    validate_reference_endpoint(kind, field, child)?;
    validate_reference_endpoint(child, inverse.field, kind)?;
    Ok(inverse)
}

/// Move one stored containment edge transactionally. Ecore selects endpoint
/// types, opposite fields and bounds; graph mutation and cycle checks are native.
/// Derived containment and incomplete/external ownership require other services.
pub(crate) fn move_contained_element(
    elements: &mut Vec<KirElement>,
    child_id: &str,
    owner_id: &str,
    field: &str,
) -> Result<(), String> {
    let mut indices = BTreeMap::new();
    for (index, element) in elements.iter().enumerate() {
        if element.id.is_empty() || indices.insert(element.id.as_str(), index).is_some() {
            return Err("Containment move requires unique nonempty identities".into());
        }
    }
    for element in elements.iter() {
        for field in element.properties.keys() {
            if feature(&element.kind, field).is_some_and(|c| c.derived || c.volatile) {
                return Err("Containment move requires a derived-value recomputation service for this graph".into());
            }
        }
    }
    let child_index = *indices.get(child_id).ok_or("Missing containment child")?;
    let owner_index = *indices.get(owner_id).ok_or("Missing destination owner")?;
    if child_index == owner_index {
        return Err("Cannot contain an element in itself".into());
    }
    if let Some(issue) = validate_publication(elements, ReferenceCompleteness::Partial).first() {
        return Err(format!(
            "Invalid input graph {}.{}: {}",
            issue.element_id, issue.field, issue.message
        ));
    }
    let inverse = containment_inverse(
        &elements[owner_index].kind,
        field,
        &elements[child_index].kind,
    )?;
    let target =
        feature(&elements[owner_index].kind, field).ok_or("Missing containment contract")?;
    if target.upper == 1 || target.volatile || inverse.volatile {
        return Err("Unsupported stored containment shape".into());
    }
    // Inspect both directions: partial graphs must not silently lose an old
    // owner's claim or manufacture an inverse while performing a move.
    let mut old = None;
    for (index, element) in elements.iter().enumerate() {
        for (name, value) in &element.properties {
            let Some(contract) =
                feature(&element.kind, name).filter(|c| c.containment && !c.derived)
            else {
                continue;
            };
            if value
                .as_array()
                .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(child_id)))
            {
                if old.is_some() {
                    return Err("Multiple existing containment claims".into());
                }
                let back = containment_inverse(&element.kind, name, &elements[child_index].kind)?;
                if elements[child_index]
                    .properties
                    .get(back.field)
                    .and_then(Value::as_str)
                    != Some(element.id.as_str())
                {
                    return Err("Existing containment has no reciprocal owner".into());
                }
                old = Some((index, contract, back));
            }
        }
    }
    for contract in generated::FEATURES.iter().filter(|c| {
        c.container && !c.derived && metaclass_conforms(&elements[child_index].kind, c.owner)
    }) {
        if let Some(value) = elements[child_index]
            .properties
            .get(contract.field)
            .filter(|v| !v.is_null())
        {
            if !old.is_some_and(|(index, _, back)| {
                back.field == contract.field && value.as_str() == Some(elements[index].id.as_str())
            }) {
                return Err("Existing owner is unresolved or lacks reciprocal containment".into());
            }
        }
    }
    if old.is_some_and(|(index, contract, _)| index == owner_index && contract.field == field) {
        return Ok(()); // Preserve list position on repeated moves to the same owner.
    }
    let mut staged = elements.clone();
    if let Some((index, contract, back)) = old {
        if back.field != inverse.field && back.lower > 0 {
            return Err("Cannot clear a required old container feature".into());
        }
        let ids = staged[index]
            .properties
            .get_mut(contract.field)
            .and_then(Value::as_array_mut)
            .ok_or("Malformed old containment")?;
        ids.retain(|id| id.as_str() != Some(child_id));
        staged[child_index].properties.remove(back.field);
    }
    let ids = staged[owner_index]
        .properties
        .entry(field.into())
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or("Malformed destination containment")?;
    ids.push(Value::String(child_id.into()));
    staged[child_index]
        .properties
        .insert(inverse.field.into(), Value::String(owner_id.into()));
    if let Some(issue) = validate_publication(&staged, ReferenceCompleteness::Partial).first() {
        return Err(format!(
            "Invalid containment move {}.{}: {}",
            issue.element_id, issue.field, issue.message
        ));
    }
    *elements = staged;
    Ok(())
}

/// Supply Foundation with exact-kind shapes from resolved Ecore ancestry.
/// Bounds, enum domains and semantic algorithms remain this service's concern.
pub(crate) fn register_field_contracts(registry: &mut KirFieldRegistry, kind: &str) {
    for contract in generated::FEATURES
        .iter()
        .filter(|c| metaclass_conforms(kind, c.owner))
    {
        let shape = match (contract.kind, contract.upper == 1) {
            (FeatureKind::Attribute, true) => KirFieldKind::Scalar,
            (FeatureKind::Attribute, false) => KirFieldKind::ScalarList,
            (FeatureKind::Reference, true) => KirFieldKind::Reference,
            (FeatureKind::Reference, false) => KirFieldKind::ReferenceList,
        };
        registry.register_scoped_field(kind, contract.field, shape);
    }
}

/// Check an explicit KIR property against Ecore's shape, bounds, and value
/// type. Absent features are left to construction/delegate validation.
pub(crate) fn validate_value(kind: &str, field: &str, value: &Value) -> Result<(), String> {
    let contract =
        feature(kind, field).ok_or_else(|| format!("unknown Ecore feature {kind}.{field}"))?;
    let values: Vec<&Value> = if contract.upper == 1 {
        if value.is_null() && contract.lower == 0 {
            return Ok(());
        }
        if value.is_array() {
            return Err(format!(
                "Ecore singular feature {kind}.{field} cannot be a list"
            ));
        }
        vec![value]
    } else {
        value
            .as_array()
            .ok_or_else(|| format!("Ecore multi-valued feature {kind}.{field} requires a list"))?
            .iter()
            .collect()
    };
    if values.len() < contract.lower as usize
        || (contract.upper >= 0 && values.len() > contract.upper as usize)
    {
        return Err(format!("Ecore bounds violated for {kind}.{field}"));
    }
    let mut unique = BTreeSet::new();
    for item in values {
        let valid = match contract.kind {
            FeatureKind::Reference => item.as_str().is_some_and(|id| !id.is_empty()),
            FeatureKind::Attribute => match contract.target {
                "Ecore::EBoolean" => item.is_boolean(),
                "Ecore::EString" => item.is_string(),
                "Ecore::EInt" => item.as_i64().is_some_and(|v| i32::try_from(v).is_ok()),
                "Ecore::EDouble" => item.is_number(),
                _ => item.as_str().is_some_and(|literal| {
                    contract
                        .enum_literals
                        .is_some_and(|values| values.contains(&literal))
                }),
            },
        };
        if !valid {
            return Err(format!("invalid Ecore value for {kind}.{field}: {item}"));
        }
        if contract.unique && !unique.insert(item.to_string()) {
            return Err(format!("duplicate Ecore value for {kind}.{field}: {item}"));
        }
    }
    Ok(())
}

pub(crate) fn validate_reference_endpoint(
    kind: &str,
    field: &str,
    endpoint_kind: &str,
) -> Result<(), String> {
    let contract =
        feature(kind, field).ok_or_else(|| format!("unknown Ecore feature {kind}.{field}"))?;
    if contract.kind != FeatureKind::Reference
        || !metaclass_conforms(endpoint_kind, contract.target)
    {
        return Err(format!(
            "Ecore reference target mismatch for {kind}.{field}: {endpoint_kind}"
        ));
    }
    Ok(())
}

pub(crate) struct ReferenceGraphIssue {
    pub element_id: String,
    pub field: String,
    pub message: String,
}

/// Required-value assessment is separate from partial-model persistence.
/// `unverified` means a semantic service or external target is still needed;
/// it must never be interpreted as successful validation.
pub(crate) struct RequiredFeatureIssue {
    pub element_id: String,
    pub field: String,
    pub feature_id: String,
    pub message: String,
    pub unverified: bool,
}

pub(crate) fn assess_required_features(elements: &[KirElement]) -> Vec<RequiredFeatureIssue> {
    assess_required_features_impl(elements, false, |_, contract| {
        Err(format!("Required Ecore feature {} needs a semantic service", contract.id))
    })
}

/// The caller supplies named native consumers, never serialized derived caches.
/// Consumer failures remain unverified; only returned values are validated here.
pub(crate) fn assess_required_features_with<'g>(
    elements: &'g [KirElement],
    semantic_value: impl FnMut(&'g KirElement, &FeatureContract) -> Result<Value, String>,
) -> Vec<RequiredFeatureIssue> {
    assess_required_features_impl(elements, true, semantic_value)
}

fn assess_required_features_impl<'g>(
    elements: &'g [KirElement],
    resolve_redefinitions: bool,
    mut semantic_value: impl FnMut(&'g KirElement, &FeatureContract) -> Result<Value, String>,
) -> Vec<RequiredFeatureIssue> {
    let by_id: BTreeMap<_, _> = elements.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut issues = Vec::new();
    for element in elements {
        let mut report = |field: &str, message: String, unverified| {
            issues.push(RequiredFeatureIssue {
                element_id: element.id.clone(),
                field: field.into(),
                feature_id: feature(&element.kind, field).map(|c| c.id).unwrap_or("").into(),
                message,
                unverified,
            });
        };
        if !metaclass_conforms(&element.kind, "Element") {
            report(
                "",
                format!("Unknown pinned Ecore class {}", element.kind),
                true,
            );
            continue;
        }
        for contract in generated::FEATURES
            .iter()
            .filter(|c| c.lower > 0 && metaclass_conforms(&element.kind, c.owner))
        {
            let field = contract.field;
            // The candidate uses the imported most-specific property, while
            // the partial abstract-syntax assessment retains its stored-only
            // boundary. Keep the requested obligation/bounds as well.
            let effective = if resolve_redefinitions {
                match redefined_feature(&element.kind, field) {
                    Ok(effective) => effective,
                    Err(message) => { report(field, message, true); continue; }
                }
            } else { contract };
            // Recording or explicitly serializing a derived value is not
            // evidence that its defining algorithm has been implemented.
            let value = if effective.derived || effective.volatile {
                match semantic_value(element, effective) {
                    Ok(value) => value,
                    Err(message) => {
                        report(field, message, true);
                        continue;
                    }
                }
            } else if let Some(value) = element.properties.get(effective.field) {
                value.clone()
            } else if field == "element_id" && contract.owner == "Element" {
                // JSON @id is represented by the canonical KIR identity.
                if element.id.is_empty() {
                    report(field, "Missing canonical element identity".into(), false);
                    continue;
                }
                Value::String(element.id.clone())
            } else if let Some(value) =
                super::ecore_defaults::constructor_attribute(&element.kind, field)
            {
                Value::String(value.into())
            } else if required_dependencies::CONSTRUCTOR_DEPENDENCIES.contains(&(
                element.kind.rsplit("::").next().unwrap_or(&element.kind),
                field,
            )) {
                report(
                    field,
                    format!(
                        "Required Ecore feature {} needs its constructor service",
                        contract.id
                    ),
                    true,
                );
                continue;
            } else if contract.kind == FeatureKind::Reference
                || (contract.target == "Ecore::EString" && contract.default_literal.is_none())
            {
                report(
                    field,
                    format!(
                        "Missing required Ecore feature {} (lower bound {})",
                        contract.id, contract.lower
                    ),
                    false,
                );
                continue;
            } else {
                match super::ecore_defaults::read_attribute(element, field) {
                    Ok(value) => value,
                    Err(message) => {
                        report(field, message, true);
                        continue;
                    }
                }
            };
            // A redefinition can narrow many-valued storage to a singular
            // value (Documentation.documentedElement). Check the effective
            // shape first, then project that value into the original bound.
            let requested_value = if effective.upper == 1 && contract.upper != 1 {
                if value.is_null() { Value::Array(Vec::new()) }
                else { Value::Array(vec![value.clone()]) }
            } else { value.clone() };
            if let Err(message) = validate_value(&element.kind, field, &requested_value) {
                report(field, message, false);
                continue;
            }
            if effective.field != field {
                if let Err(message) = validate_value(&element.kind, effective.field, &value) {
                    report(field, message, false);
                    continue;
                }
            }
            if contract.kind == FeatureKind::Reference {
                let ids: Vec<&str> = match &value {
                    Value::String(id) => vec![id],
                    Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
                    _ => Vec::new(),
                };
                for id in ids {
                    match by_id.get(id) {
                        Some(target) => {
                            if let Err(message) = validate_reference_endpoint(&element.kind, field, &target.kind)
                                .and_then(|()| validate_reference_endpoint(&element.kind, effective.field, &target.kind))
                            {
                                report(field, message, false);
                            }
                        }
                        None => report(
                            field,
                            format!(
                                "Required reference {} needs external target {id}",
                                contract.id
                            ),
                            true,
                        ),
                    }
                }
            }
        }
    }
    issues
}

/// Validate the present, canonical Ecore properties of a partial native model.
/// Unknown extension properties and missing/externally resolved values remain
/// outside this check. Callers still apply their serialization field contract.
pub(crate) fn validate_explicit_model(elements: &[KirElement]) -> Vec<ReferenceGraphIssue> {
    let by_id: BTreeMap<_, _> = elements
        .iter()
        .map(|e| (e.id.as_str(), e.kind.as_str()))
        .collect();
    let mut issues = validate_explicit_reference_graph(elements);
    issues.extend(validate_explicit_subsets(elements));
    issues.extend(validate_explicit_unions(elements));
    for element in elements {
        for (field, value) in &element.properties {
            let Some(contract) = feature(&element.kind, field) else {
                continue;
            };
            if let Err(message) = validate_value(&element.kind, field, value) {
                issues.push(ReferenceGraphIssue {
                    element_id: element.id.clone(),
                    field: field.clone(),
                    message,
                });
                continue;
            }
            if contract.kind == FeatureKind::Reference {
                let ids: Vec<&str> = match value {
                    Value::String(id) => vec![id],
                    Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
                    _ => Vec::new(),
                };
                for id in ids {
                    if let Some(kind) = by_id.get(id) {
                        if let Err(message) =
                            validate_reference_endpoint(&element.kind, field, kind)
                        {
                            issues.push(ReferenceGraphIssue {
                                element_id: element.id.clone(),
                                field: field.clone(),
                                message,
                            });
                        }
                    }
                }
            }
        }
    }
    issues
}

/// Initialize the physical reference slots of a fresh native object from Ecore.
/// Only stored reciprocal containment and nullable container slots are known at
/// allocation. Cross-references and derived views retain their own dependencies.
/// This is never an import repair pass: explicit values are preserved, including
/// malformed values that the publication boundary must reject.
pub(crate) fn initialize_fresh_structure(element: &mut KirElement) {
    type Slots = BTreeMap<&'static str, Vec<(&'static str, Value)>>;
    static SLOTS: OnceLock<Slots> = OnceLock::new();
    let slots = SLOTS.get_or_init(|| {
        let contracts = generated::FEATURES.iter().filter(|c|
            c.kind == FeatureKind::Reference && c.changeable && !c.unsettable
                && !c.derived && !c.transient && !c.volatile && c.opposite.is_some()
                && c.lower == 0 && ((c.containment && c.upper == -1)
                    || (c.container && c.upper == 1 && c.emf_default_json == "null")))
            .collect::<Vec<_>>();
        metaclass_names().iter().map(|kind| {
            let fields = contracts.iter().filter(|c| metaclass_conforms(kind, c.owner))
                .map(|c| (c.field, if c.container { Value::Null } else { Value::Array(Vec::new()) }))
                .collect();
            (*kind, fields)
        }).collect()
    });
    let kind = element.kind.rsplit("::").next().unwrap_or(&element.kind);
    if let Some(fields) = slots.get(kind) {
        for (field, value) in fields {
            element.properties.entry((*field).into()).or_insert_with(|| value.clone());
        }
    }
}

/// Publication distinguishes an inspectable partial graph from a fully linked
/// graph. Partial retains opaque interchange extensions; Closed requires pinned
/// classes, resolved references and both directions of each present stored
/// opposite. Absent required values and executable delegate semantics remain
/// separate assessments in either mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReferenceCompleteness {
    Partial,
    Closed,
}

/// Shared publication boundary for canonical models built from source, imported,
/// mutated or persisted. Preserve explicit derived snapshots and validate their
/// known contracts; never erase a snapshot to turn disagreement into success.
/// The graph transaction policy is handwritten Rust over imported Ecore contracts.
pub(crate) fn validate_publication(
    elements: &[KirElement],
    references: ReferenceCompleteness,
) -> Vec<ReferenceGraphIssue> {
    let mut issues = Vec::new();
    let mut ids = BTreeSet::new();
    for element in elements {
        if element.id.is_empty() || !ids.insert(element.id.as_str()) {
            issues.push(ReferenceGraphIssue { element_id: element.id.clone(), field: "element_id".into(),
                message: "Publication requires unique nonempty model identities".into() });
        }
        // Partial interchange preserves opaque extension kinds (for example
        // Issue). Closed candidate publication accepts pinned EClasses only.
        // No Ecore validation credit is assigned to an opaque extension.
        if references == ReferenceCompleteness::Closed
            && !metaclass_names().contains(&element.kind.rsplit("::").next().unwrap_or(&element.kind)) {
            issues.push(ReferenceGraphIssue { element_id: element.id.clone(), field: String::new(),
                message: format!("Unknown pinned Ecore class {}", element.kind) });
        }
    }
    if !issues.is_empty() { return issues; }
    issues.extend(validate_explicit_model(elements));
    if references == ReferenceCompleteness::Closed {
        let by_id: BTreeMap<_, _> = elements.iter().map(|e| (e.id.as_str(), e)).collect();
        for element in elements {
            for (field, value) in &element.properties {
                let Some(contract) = feature(&element.kind, field).filter(|c| c.kind == FeatureKind::Reference) else {
                    continue;
                };
                let targets: Vec<&str> = match value {
                    Value::String(id) => vec![id],
                    Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
                    _ => Vec::new(),
                };
                for target in targets {
                    let Some(endpoint) = by_id.get(target) else {
                        issues.push(ReferenceGraphIssue { element_id: element.id.clone(), field: field.clone(),
                            message: format!("Closed publication requires resolved reference {target}") });
                        continue;
                    };
                    // Existing explicit mismatches are checked above. Closed
                    // canonical storage additionally cannot omit the inverse of
                    // a present stored edge. Derived opposites remain getters
                    // with separate semantic dependencies, not required slots.
                    if !contract.derived && !contract.volatile && !contract.transient {
                        if let Some(inverse) = opposite_feature(contract)
                            .filter(|c| !c.derived && !c.volatile && !c.transient)
                        {
                            if !endpoint.properties.contains_key(inverse.field) {
                                issues.push(ReferenceGraphIssue { element_id: element.id.clone(), field: field.clone(),
                                    message: format!("Closed publication requires stored opposite {target}.{} for {}.{field}", inverse.field, element.id) });
                            }
                        }
                    }
                }
            }
        }
    }
    issues
}

pub(crate) struct StoredFeatureEdit<'a> {
    pub element_id: &'a str,
    pub field: &'a str,
    /// None removes an optional stored property. This does not implement EMF's
    /// separate isSet state for an unsettable feature.
    pub value: Option<Value>,
}

/// Apply a whole batch atomically. Direct stored writes use imported domains,
/// cardinality, uniqueness and mutability; linked references must resolve in the
/// existing graph. Ownership changes use move_contained_element instead.
/// Derived snapshots require a recomputation service, so effective writes to
/// graphs containing them fail explicitly without altering those snapshots.
pub(crate) fn apply_stored_feature_edits(
    elements: &mut Vec<KirElement>,
    edits: &[StoredFeatureEdit<'_>],
) -> Result<(), String> {
    if let Some(issue) = validate_publication(elements, ReferenceCompleteness::Partial).first() {
        return Err(format!("Invalid input graph {}.{}: {}", issue.element_id, issue.field, issue.message));
    }
    let indices: BTreeMap<_, _> = elements.iter().enumerate().map(|(i, e)| (e.id.as_str(), i)).collect();
    let mut changed = false;
    let mut touched = BTreeSet::new();
    let mut staged = elements.clone();
    for edit in edits {
        if !touched.insert((edit.element_id, edit.field)) {
            return Err(format!("Duplicate stored feature edit {}.{}", edit.element_id, edit.field));
        }
        let index = *indices.get(edit.element_id).ok_or_else(|| format!("Missing edit target {}", edit.element_id))?;
        let element = &mut staged[index];
        let contract = feature(&element.kind, edit.field).ok_or_else(|| format!("Unknown stored Ecore feature {}.{}", element.kind, edit.field))?;
        let effective = redefined_feature(&element.kind, edit.field)?;
        if effective.field != contract.field {
            return Err(format!("Ecore feature {} is redefined by {}; use its storage/delegate service", contract.id, effective.id));
        }
        if !contract.changeable || contract.derived || contract.volatile || contract.transient || contract.setting_delegate.is_some() {
            return Err(format!("Ecore feature {} is not directly writable stored state", contract.id));
        }
        if contract.unsettable {
            return Err(format!("Ecore feature {} requires an explicit isSet service", contract.id));
        }
        if contract.containment || contract.container || contract.opposite.is_some() {
            return Err(format!("Ecore feature {} requires the ownership/opposite mutation service", contract.id));
        }
        if edit.field == "element_id" {
            return Err("Canonical identity changes require a graph identity service".into());
        }
        if let Some(value) = &edit.value {
            validate_value(&element.kind, edit.field, value)?;
            if contract.kind == FeatureKind::Reference {
                let targets: Vec<&str> = match value {
                    Value::String(id) => vec![id],
                    Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
                    _ => Vec::new(),
                };
                for target in targets {
                    let target_index = *indices.get(target).ok_or_else(|| format!("Stored reference edit requires resolved target {target}; resolveProxies={:?} is not a resolver", contract.resolve_proxies))?;
                    validate_reference_endpoint(&element.kind, edit.field, &elements[target_index].kind)?;
                }
            }
            if element.properties.get(edit.field) != Some(value) { changed = true; }
            element.properties.insert(edit.field.into(), value.clone());
        } else {
            if contract.lower > 0 {
                return Err(format!("Cannot remove required stored feature {}", contract.id));
            }
            changed |= element.properties.remove(edit.field).is_some();
        }
    }
    if changed && elements.iter().any(|element| element.properties.keys().any(|field|
        feature(&element.kind, field).is_some_and(|c| c.derived || c.volatile))) {
        return Err("Stored edits require a derived-value recomputation service for this graph".into());
    }
    if let Some(issue) = validate_publication(&staged, ReferenceCompleteness::Partial).first() {
        return Err(format!("Invalid staged graph {}.{}: {}", issue.element_id, issue.field, issue.message));
    }
    *elements = staged;
    Ok(())
}

/// Imported direct/transitive subset relationship applicable to this metaclass.
pub(crate) fn has_subset_contract(kind: &str, subset: &str, superset: &str) -> bool {
    generated::SUBSET_FIELDS.iter().any(|(owner, child, parent)|
        *child == subset && *parent == superset && metaclass_conforms(kind, owner))
}

/// Check present subset/superset snapshots using imported transitive contracts.
/// Missing properties remain unknown, including derived values. This neither
/// executes delegates nor maintains subset values during mutation.
fn validate_explicit_subsets(elements: &[KirElement]) -> Vec<ReferenceGraphIssue> {
    fn values(value: &Value) -> Vec<&Value> {
        match value {
            Value::Null => Vec::new(),
            Value::Array(values) => values.iter().collect(),
            value => vec![value],
        }
    }
    let mut issues = Vec::new();
    for element in elements {
        for (owner, subset, superset) in generated::SUBSET_FIELDS {
            if !metaclass_conforms(&element.kind, owner) { continue; }
            let (Some(child), Some(parent)) = (element.properties.get(*subset), element.properties.get(*superset)) else { continue; };
            // Shape diagnostics belong to validate_explicit_model. Never coerce
            // malformed scalar/collection representations into valid sets.
            if validate_value(&element.kind, subset, child).is_err()
                || validate_value(&element.kind, superset, parent).is_err() { continue; }
            let parent_values = values(parent);
            if values(child).iter().any(|value| !parent_values.contains(value)) {
                issues.push(ReferenceGraphIssue {
                    element_id: element.id.clone(), field: (*subset).into(),
                    message: format!("Explicit subset {subset} contains a value absent from {superset}"),
                });
            }
        }
    }
    issues
}

pub(crate) fn evaluate_plain_type_generalizations(elements: &[KirElement], id: &str,
    library: &BTreeMap<String, String>) -> Result<Vec<String>, String> {
    let by_id: BTreeMap<_, _> = elements.iter().map(|e| (e.id.as_str(), e)).collect();
    if by_id.len() != elements.len() { return Err("Duplicate model identity".into()); }
    let owner = by_id.get(id).ok_or_else(|| format!("Unknown element {id}"))?;
    super::emit::stored_plain_type_generalizations(elements, owner, library).map_err(|d| format!("{d:?}"))
}

/// Evaluate on a private graph copy after completing only the bounded plain-Type
/// implicit defaults. Original storage and completeness assertions are unchanged.
pub(crate) fn evaluate_plain_type_membership(elements: &[KirElement], id: &str,
    library: &BTreeMap<String, String>) -> Result<Value, String> {
    evaluate_plain_type_generalizations(elements, id, library)?;
    let mut staged = elements.to_vec();
    let mut next = 0usize;
    for owner in elements.iter().filter(|e| metaclass_conforms(&e.kind, "Type")) {
        if super::ecore_defaults::read_attribute(owner, "is_implied_included")? == true { continue; }
        let computed = evaluate_plain_type_generalizations(elements, &owner.id, library)?;
        let explicit = super::emit::stored_explicit_general_types(elements, owner)
            .map_err(|d| format!("{d:?}"))?;
        if !computed.starts_with(&explicit) { return Err("Generalization contribution order changed".into()); }
        let relation_kind = if metaclass_conforms(&owner.kind, "Classifier") { "Subclassification" } else { "Specialization" };
        let specific = redefined_feature(relation_kind, "specific")?;
        let general = redefined_feature(relation_kind, "general")?;
        let mut added = Vec::new();
        for target in &computed[explicit.len()..] {
            let relation_id = loop {
                let candidate = format!("__native_default_general_{next}"); next += 1;
                if !staged.iter().any(|e| e.id == candidate) { break candidate; }
            };
            staged.push(KirElement { id: relation_id.clone(), kind: relation_kind.into(), layer: owner.layer,
                properties: BTreeMap::from([
                    ("owning_related_element".into(), Value::String(owner.id.clone())),
                    (specific.field.into(), Value::String(owner.id.clone())),
                    (general.field.into(), Value::String(target.clone())),
                    ("is_implied".into(), Value::Bool(true)),
                ]) });
            added.push(Value::String(relation_id));
        }
        let stored = staged.iter_mut().find(|e| e.id == owner.id).ok_or_else(|| "Missing staged Type".to_owned())?;
        let relationships = stored.properties.get_mut("owned_relationship").and_then(Value::as_array_mut)
            .ok_or_else(|| "Missing canonical relationship collection".to_owned())?;
        relationships.extend(added);
        stored.properties.insert("is_implied_included".into(), Value::Bool(true));
    }
    let owner = staged.iter().find(|e| e.id == id).ok_or_else(|| format!("Unknown element {id}"))?;
    evaluate_explicit_union(&staged, owner, "membership")
}

pub(crate) fn evaluate_plain_type_membership_from_libraries(elements: &[KirElement], id: &str) -> Result<Value, String> {
    let bindings = super::emit::stored_standard_default_bindings(elements).map_err(|d| format!("{d:?}"))?;
    evaluate_plain_type_membership(elements, id, &bindings)
}

/// Evaluate the pinned ordered union from explicit or canonically derived inputs.
/// Ecore supplies membership/type/shape contracts. The inter-subset ordering is
/// a handwritten Pilot-compatible policy, independently tested against its
/// runtime getter; the annotation alone does not establish this ordering.
pub(crate) fn evaluate_explicit_union(
    elements: &[KirElement],
    element: &KirElement,
    field: &str,
) -> Result<Value, String> {
    if field != "membership" { return Err("No imported membership union contract".into()); }
    let inputs = membership_union_sources(&element.kind)?;
    let by_id: BTreeMap<_, _> = elements.iter().map(|e| (e.id.as_str(), e)).collect();
    if by_id.len() != elements.len() { return Err("Duplicate model identity".into()); }
    let canonical = super::emit::stored_namespace_membership_inputs(elements, element)
        .map_err(|diagnostic| format!("{diagnostic:?}"))?;
    let imported = if canonical.as_ref().is_some_and(|(_, imports)| *imports) && metaclass_conforms(&element.kind, "Namespace") {
        Some(super::emit::stored_namespace_imported_memberships(elements, element)
            .map_err(|diagnostic| format!("{diagnostic:?}"))?)
    } else { None };
    let inherited = super::emit::stored_type_inherited_memberships(elements, element)
        .map_err(|diagnostic| format!("{diagnostic:?}"))?;
    let empty = Value::Array(Vec::new());
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    for (input_owner, input_field) in inputs {
        if !metaclass_conforms(&element.kind, input_owner) { continue; }
        let derived = match (*input_field, &canonical) {
            ("inherited_membership", _) => inherited.as_ref(),
            ("owned_membership", Some((owned, _))) => Some(owned),
            ("imported_membership", Some((_, false))) => Some(&empty),
            ("imported_membership", Some((_, true))) => imported.as_ref(),
            _ => None,
        };
        let snapshot = element.properties.get(*input_field);
        if let (Some(derived), Some(snapshot)) = (derived, snapshot) {
            if derived != snapshot {
                return Err(format!("Union input {input_field} snapshot disagrees with canonical ownership"));
            }
        }
        let value = derived.or(snapshot)
            .ok_or_else(|| format!("Union {field} requires resolved input {input_field}"))?;
        validate_value(&element.kind, input_field, value)?;
        let values = value.as_array().ok_or_else(|| format!("Union input {input_field} requires a collection"))?;
        for value in values {
            let id = value.as_str().ok_or_else(|| format!("Invalid union input identity {input_field}"))?;
            let target = by_id.get(id).ok_or_else(|| format!("Union input {input_field} requires resolved target {id}"))?;
            validate_reference_endpoint(&element.kind, input_field, &target.kind)?;
            if seen.insert(id) { result.push(value.clone()); }
        }
    }
    Ok(Value::Array(result))
}

/// Validate union membership only when every applicable direct subset snapshot
/// is present. Ordering, deriving those snapshots and maintaining them remain
/// separate semantic dependencies. An omitted subset is unknown, not empty.
fn validate_explicit_unions(elements: &[KirElement]) -> Vec<ReferenceGraphIssue> {
    fn values(value: &Value) -> Vec<&Value> {
        match value {
            Value::Null => Vec::new(),
            Value::Array(values) => values.iter().collect(),
            value => vec![value],
        }
    }
    let mut issues = Vec::new();
    for element in elements {
        for (owner, field, inputs) in generated::UNION_FIELDS {
            if !metaclass_conforms(&element.kind, owner) { continue; }
            let Some(union) = element.properties.get(*field) else { continue; };
            if validate_value(&element.kind, field, union).is_err() { continue; }
            let mut expected = Vec::new();
            let mut complete = true;
            for (input_owner, input_field) in *inputs {
                if !metaclass_conforms(&element.kind, input_owner) { continue; }
                let Some(value) = element.properties.get(*input_field) else { complete = false; break; };
                if validate_value(&element.kind, input_field, value).is_err() { complete = false; break; }
                expected.extend(values(value));
            }
            if !complete { continue; }
            let actual = values(union);
            if actual.iter().any(|v| !expected.contains(v)) || expected.iter().any(|v| !actual.contains(v)) {
                issues.push(ReferenceGraphIssue {
                    element_id: element.id.clone(), field: (*field).into(),
                    message: format!("Explicit union {field} disagrees with its complete subset snapshots"),
                });
            }
        }
    }
    issues
}

/// Check only facts explicitly present in a partial native graph. Ecore supplies
/// opposite identities and containment flags; reciprocal consistency, single
/// containment and cycle detection are handwritten Rust. Missing inverse values
/// are not synthesized, and derived containment requires its delegate algorithm.
pub(crate) fn validate_explicit_reference_graph(
    elements: &[KirElement],
) -> Vec<ReferenceGraphIssue> {
    let by_id: BTreeMap<_, _> = elements.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut issues = Vec::new();
    // child -> (parent, containing EFeature identity, assertion origin)
    let mut parents: BTreeMap<&str, (&str, &str, &str, &str)> = BTreeMap::new();
    for element in elements {
        for (field, value) in &element.properties {
            let Some(contract) =
                feature(&element.kind, field).filter(|c| c.kind == FeatureKind::Reference)
            else {
                continue;
            };
            let opposite = opposite_feature(contract);
            let ids: Vec<&str> = match value {
                Value::String(id) => vec![id],
                Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
                _ => continue,
            };
            for id in ids {
                if let (Some(target), Some(inverse)) = (by_id.get(id), opposite) {
                    if let Some(back) = target.properties.get(inverse.field) {
                        let contains_source = back.as_str() == Some(element.id.as_str())
                            || back.as_array().is_some_and(|items| {
                                items
                                    .iter()
                                    .any(|item| item.as_str() == Some(element.id.as_str()))
                            });
                        if !contains_source {
                            issues.push(ReferenceGraphIssue {
                                element_id: element.id.clone(), field: field.clone(),
                                message: format!("Ecore opposite mismatch: {}.{} references {id}, but {}.{} does not reference {}",
                                    element.id, field, id, inverse.field, element.id),
                            });
                        }
                    }
                }
                let containment = if contract.containment && !contract.derived {
                    Some((id, element.id.as_str(), contract.id))
                } else if contract.container && !contract.derived {
                    opposite
                        .filter(|c| c.containment && !c.derived)
                        .map(|inverse| (element.id.as_str(), id, inverse.id))
                } else {
                    None
                };
                if let Some((child, parent, identity)) = containment {
                    if let Some((prior_parent, prior_identity, _, _)) = parents.get(child) {
                        if (*prior_parent, *prior_identity) != (parent, identity) {
                            issues.push(ReferenceGraphIssue {
                                element_id: element.id.clone(), field: field.clone(),
                                message: format!("Ecore element {child} has conflicting containment claims from {prior_parent} and {parent}"),
                            });
                        }
                    } else {
                        parents.insert(child, (parent, identity, &element.id, field));
                    }
                }
            }
        }
    }
    let mut completed = BTreeSet::new();
    for start in parents.keys().copied() {
        let mut path = BTreeSet::new();
        let mut cursor = start;
        while !completed.contains(cursor) {
            let Some((parent, _, subject, field)) = parents.get(cursor) else {
                break;
            };
            if !path.insert(cursor) {
                issues.push(ReferenceGraphIssue {
                    element_id: (*subject).into(),
                    field: (*field).into(),
                    message: format!("Ecore containment cycle involving {cursor}"),
                });
                break;
            }
            cursor = parent;
        }
        completed.extend(path);
    }
    issues
}

/// Reconcile synthesized library feature descriptors with pinned Ecore before
/// Foundation builds its document-wide KIR field registry for persistence.
/// The source label is retained so a correction remains auditable. Ecore does
/// not supply the algorithm that synthesizes these library descriptors.
pub fn reconcile_library_metafeatures_with_ecore(
    elements: &mut [KirElement],
) -> Result<usize, String> {
    let mut corrected = 0;
    for element in elements
        .iter_mut()
        .filter(|element| element.kind == "MetamodelFeature")
    {
        let properties = &mut element.properties;
        let Some(owner) = properties.get("owner").and_then(Value::as_str) else {
            continue;
        };
        let Some(field) = properties
            .get("kir_property")
            .or_else(|| properties.get("declared_name"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let class = owner.rsplit("::").next().unwrap_or(owner).to_owned();
        let Some(contract) = feature(&class, field) else {
            continue;
        };
        if contract.kind != FeatureKind::Attribute {
            continue;
        }
        let Some(source_kind) = properties
            .get("feature_kind")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            return Err(format!(
                "{} has invalid source feature kind for pinned Ecore attribute {}",
                element.id, contract.id
            ));
        };
        let ecore_id = Value::String(contract.id.into());
        if properties
            .get("x_ecore_feature_id")
            .is_some_and(|value| value != &ecore_id)
        {
            return Err(format!(
                "{} has conflicting pinned Ecore feature ID",
                element.id
            ));
        }
        properties.insert("x_ecore_feature_id".into(), ecore_id);

        // Foundation's generic field registry does not infer inheritance. The
        // language layer supplies the resolved, exact Ecore descendant set.
        let owner_kinds = Value::Array(
            metaclass_names()
                .iter()
                .filter(|kind| metaclass_conforms(kind, &class))
                .flat_map(|kind| {
                    [
                        Value::String((*kind).into()),
                        Value::String(format!("SysML::{kind}")),
                    ]
                })
                .collect(),
        );
        if owner_kinds.as_array().is_none_or(Vec::is_empty)
            || properties
                .get("kir_owner_kinds")
                .is_some_and(|value| value != &owner_kinds)
        {
            return Err(format!(
                "{} has conflicting Ecore owner-kind closure",
                element.id
            ));
        }
        properties.insert("kir_owner_kinds".into(), owner_kinds);

        if contract.upper != 1 {
            let upper = Value::from(contract.upper);
            if properties.get("upper").is_some_and(|value| value != &upper) {
                return Err(format!(
                    "{} has conflicting Ecore attribute upper bound",
                    element.id
                ));
            }
            properties.insert("upper".into(), upper);
        }
        match source_kind.as_str() {
            "attribute" => {
                if properties.get("x_source_feature_kind").is_some()
                    && properties.get("x_source_feature_kind")
                        != Some(&Value::String("reference".into()))
                {
                    return Err(format!(
                        "{} has conflicting Ecore reconciliation provenance",
                        element.id
                    ));
                }
            }
            "reference" => {
                if properties.contains_key("x_source_feature_kind") {
                    return Err(format!(
                        "{} has preexisting Ecore reconciliation provenance",
                        element.id
                    ));
                }
                properties.insert(
                    "x_source_feature_kind".into(),
                    Value::String(source_kind.into()),
                );
                properties.insert(
                    "x_ecore_feature_id".into(),
                    Value::String(contract.id.into()),
                );
                properties.insert("feature_kind".into(), Value::String("attribute".into()));
                corrected += 1;
            }
            other => {
                return Err(format!(
                    "{} has unexpected source feature kind {other} for pinned Ecore attribute {}",
                    element.id, contract.id
                ));
            }
        }
    }
    Ok(corrected)
}

#[cfg(test)]
mod tests {
    #[test]
    fn closed_publication_rejects_opaque_interchange_extensions() {
        let extension = KirElement { id: "issue".into(), kind: "Issue".into(), layer: 2,
            properties: BTreeMap::from([("text".into(), serde_json::json!("Interchange diagnostic"))]) };
        assert!(validate_publication(std::slice::from_ref(&extension), ReferenceCompleteness::Partial).is_empty());
        assert!(validate_publication(&[extension], ReferenceCompleteness::Closed).iter().any(|i| i.message.contains("Unknown pinned Ecore class")));
    }

    #[test]
    fn closed_publication_requires_both_directions_of_present_stored_opposites() {
        use serde_json::json;
        let graph = vec![
            KirElement { id: "p".into(), kind: "SysML::Package".into(), layer: 2,
                properties: BTreeMap::from([("owned_relationship".into(), json!(["r"]))]) },
            KirElement { id: "r".into(), kind: "SysML::Relationship".into(), layer: 2,
                properties: BTreeMap::from([("owning_related_element".into(), json!("p")),
                    ("owned_related_element".into(), json!(["c"]))]) },
            KirElement { id: "c".into(), kind: "SysML::Class".into(), layer: 2,
                properties: BTreeMap::from([("owning_relationship".into(), json!("r"))]) },
        ];
        assert!(validate_publication(&graph, ReferenceCompleteness::Closed).is_empty());
        // Exercise both imported stored opposite pairs, omitting either end.
        for (index, field) in [(0, "owned_relationship"), (1, "owning_related_element"),
                              (1, "owned_related_element"), (2, "owning_relationship")] {
            let mut partial = graph.clone();
            partial[index].properties.remove(field);
            let before = serde_json::to_value(&partial).unwrap();
            assert!(validate_publication(&partial, ReferenceCompleteness::Partial).is_empty(), "{field}");
            let issues = validate_publication(&partial, ReferenceCompleteness::Closed);
            assert_eq!(issues.len(), 1, "{field}");
            assert!(issues[0].message.contains("requires stored opposite"), "{}", issues[0].message);
            assert!(issues[0].message.contains(field), "{}", issues[0].message);
            assert_eq!(serde_json::to_value(&partial).unwrap(), before);
        }
        let derived = vec![
            KirElement { id: "p".into(), kind: "SysML::Package".into(), layer: 2,
                properties: BTreeMap::from([("owned_member".into(), json!(["c"]))]) },
            KirElement { id: "c".into(), kind: "SysML::Class".into(), layer: 2, properties: BTreeMap::new() },
            KirElement { id: "typing".into(), kind: "SysML::FeatureTyping".into(), layer: 2, properties: BTreeMap::new() },
        ];
        // Missing derived inverses and required semantic inputs are deliberately
        // separate obligations; this structural check does not invent them.
        assert!(validate_publication(&derived, ReferenceCompleteness::Closed).is_empty());
    }

    #[test]
    fn every_stored_contract_enforces_its_imported_value_shape() {
        use serde_json::json;
        let mut count = 0;
        for contract in generated::FEATURES.iter().filter(|c| !c.derived) {
            count += 1;
            let item = match contract.kind {
                FeatureKind::Reference => json!("target"),
                FeatureKind::Attribute => match contract.target {
                    "Ecore::EBoolean" => json!(false),
                    "Ecore::EString" => json!("value"),
                    "Ecore::EInt" => json!(3),
                    "Ecore::EDouble" => json!(3.5),
                    _ => json!(contract.enum_literals.unwrap()[0]),
                },
            };
            let value = if contract.upper == 1 { item.clone() } else { json!([item.clone()]) };
            assert!(validate_value(contract.owner, contract.field, &value).is_ok(), "{}", contract.id);
            let wrong_shape = if contract.upper == 1 { json!([item.clone()]) } else { item.clone() };
            assert!(validate_value(contract.owner, contract.field, &wrong_shape).is_err(), "{}", contract.id);
            if contract.unique && contract.upper != 1 {
                assert!(validate_value(contract.owner, contract.field, &json!([item.clone(), item])).is_err(), "{}", contract.id);
            }
            assert!(contract.changeable);
            assert!(!contract.unsettable);
            assert_eq!(contract.resolve_proxies, if contract.kind == FeatureKind::Reference { Some(true) } else { None });
        }
        assert_eq!(count, 87); // Structural value-shape evidence, not semantic qualification.
    }

    #[test]
    fn fresh_structure_initialization_uses_ancestry_without_overwriting_values() {
        use serde_json::json;
        let mut element = KirElement { id: "r".into(), kind: "SysML::OwningMembership".into(), layer: 0,
            properties: BTreeMap::from([("owned_related_element".into(), json!(["child"]))]) };
        initialize_fresh_structure(&mut element);
        assert_eq!(element.properties["owned_relationship"], json!([]));
        assert_eq!(element.properties["owned_related_element"], json!(["child"]));
        assert_eq!(element.properties["owning_relationship"], Value::Null);
        assert_eq!(element.properties["owning_related_element"], Value::Null);
        assert_eq!(element.properties.len(), 4);
        assert!(!element.properties.contains_key("is_implied_included"));
        let before = element.properties.clone();
        initialize_fresh_structure(&mut element);
        assert_eq!(element.properties, before);
    }

    use super::*;
    #[test]
    fn explicit_union_checks_match_independent_pilot_getter_observations() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../resources/metamodels/sysml-2.0-pilot-2026-08/union-pilot-controls.json"
        )).unwrap();
        let controls = evidence["controls"].as_array().unwrap();
        assert_eq!(controls.len(), 8);
        for control in controls {
            let properties = control.as_object().unwrap().iter().filter(|(k, _)| k.as_str() != "kind")
                .map(|(k, v)| (k.clone(), v.clone())).collect();
            let element = KirElement { id: "namespace".into(), kind: control["kind"].as_str().unwrap().into(), layer: 0, properties };
            let mut graph = vec![element.clone()];
            for id in ["a", "b", "c", "d"] {
                graph.push(KirElement { id: id.into(), kind: "SysML::Membership".into(), layer: 0, properties: BTreeMap::new() });
            }
            assert_eq!(evaluate_explicit_union(&graph, &element, "membership").unwrap(), control["membership"]);
            assert!(validate_explicit_unions(&[element.clone()]).is_empty(), "{control}");
            assert!(validate_explicit_subsets(&[element.clone()]).is_empty(), "{control}");
            let mut extra = element.clone();
            extra.properties.get_mut("membership").unwrap().as_array_mut().unwrap().push(serde_json::json!("not-an-input"));
            assert_eq!(validate_explicit_unions(&[extra]).len(), 1);
            if !element.properties["membership"].as_array().unwrap().is_empty() {
                let mut missing = element.clone();
                missing.properties.get_mut("membership").unwrap().as_array_mut().unwrap().remove(0);
                assert_eq!(validate_explicit_unions(&[missing]).len(), 1);
            }
            // Missing inputs cannot be interpreted as authoritative empty lists.
            let mut partial = element;
            partial.properties.remove("imported_membership");
            assert!(evaluate_explicit_union(&graph, &partial, "membership").unwrap_err().contains("resolved input"));
            assert!(validate_explicit_unions(&[partial]).is_empty());
        }
    }

    #[test]
    fn explicit_union_snapshots_require_every_applicable_subset() {
        let mut element = KirElement { id: "n".into(), kind: "SysML::Package".into(), layer: 0,
            properties: BTreeMap::from([
                ("owned_membership".into(), serde_json::json!(["a"])),
                ("imported_membership".into(), serde_json::json!(["a", "b"])),
                ("membership".into(), serde_json::json!(["b", "a"])),
            ]) };
        // Set membership is checked; cross-subset ordering is not yet qualified.
        assert!(validate_explicit_unions(&[element.clone()]).is_empty());
        element.properties.insert("membership".into(), serde_json::json!(["a", "b", "c"]));
        assert_eq!(validate_explicit_unions(&[element.clone()]).len(), 1);
        element.kind = "SysML::Class".into();
        assert!(validate_explicit_unions(&[element.clone()]).is_empty());
        element.properties.insert("inherited_membership".into(), serde_json::json!([]));
        assert_eq!(validate_explicit_unions(&[element.clone()]).len(), 1);
        element.properties.insert("inherited_membership".into(), serde_json::json!(["c"]));
        assert!(validate_explicit_unions(&[element.clone()]).is_empty());
        element.properties.insert("membership".into(), serde_json::json!(["a"]));
        assert_eq!(validate_explicit_unions(&[element.clone()]).len(), 1);
        element.properties.remove("imported_membership");
        assert!(validate_explicit_unions(&[element]).is_empty());
    }

    #[test]
    fn explicit_subset_snapshots_follow_generated_transitive_contracts() {
        let mut element = KirElement { id: "r".into(), kind: "SysML::Relationship".into(), layer: 0,
            properties: BTreeMap::from([
                ("source".into(), serde_json::json!(["a"])),
                ("related_element".into(), serde_json::json!(["a", "b"])),
            ]) };
        assert!(validate_explicit_subsets(&[element.clone()]).is_empty());
        element.properties.insert("related_element".into(), serde_json::json!(["b"]));
        assert_eq!(validate_explicit_subsets(&[element.clone()]).len(), 1);
        assert!(validate_explicit_model(&[element.clone()]).iter().any(|issue| issue.message.contains("Explicit subset")));
        element.properties.remove("related_element");
        assert!(validate_explicit_subsets(&[element.clone()]).is_empty());
        element.properties.insert("related_element".into(), serde_json::json!([]));
        element.properties.insert("source".into(), serde_json::json!([]));
        assert!(validate_explicit_subsets(&[element]).is_empty());
        let member = KirElement { id: "m".into(), kind: "SysML::OwningMembership".into(), layer: 0,
            properties: BTreeMap::from([
                ("owned_member_element".into(), serde_json::json!("a")),
                ("related_element".into(), serde_json::json!(["b"])),
            ]) };
        // ownedMemberElement -> ownedRelatedElement -> relatedElement;
        // the absent intermediate snapshot does not hide the contradiction.
        assert!(validate_explicit_subsets(&[member]).iter().any(|issue| issue.field == "owned_member_element"));
    }

    #[test]
    fn containment_moves_match_independent_emf_forward_and_inverse_writes() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../resources/metamodels/sysml-2.0-pilot-2026-08/containment-moves-pilot-controls.json"
        )).unwrap();
        let controls = &evidence["controls"];
        let mut graph: Vec<KirElement> = controls["initial"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| {
                let kind = node["class"].as_str().unwrap();
                let properties = node["fields"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(name, value)| {
                        let contract = generated::FEATURES
                            .iter()
                            .find(|c| c.name == name && metaclass_conforms(kind, c.owner))
                            .unwrap();
                        (contract.field.into(), value.clone())
                    })
                    .collect();
                KirElement {
                    id: node["id"].as_str().unwrap().into(),
                    kind: kind.into(),
                    layer: 2,
                    properties,
                }
            })
            .collect();
        let steps = controls["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 8);
        assert_eq!(
            steps
                .iter()
                .filter(|s| s["recursive_destination"] == false)
                .count(),
            6
        );
        assert_eq!(
            steps
                .iter()
                .filter(|s| s["recursive_destination"] == true && s["accepted"] == false)
                .count(),
            1
        );
        assert_eq!(
            steps
                .iter()
                .filter(|s| s["recursive_destination"] == true && s["accepted"] == true)
                .count(),
            1
        );
        for step in steps {
            let owner = step["owner"].as_str().unwrap();
            let owner_kind = &graph.iter().find(|e| e.id == owner).unwrap().kind;
            let contract = generated::FEATURES
                .iter()
                .find(|c| {
                    c.name == step["field"].as_str().unwrap()
                        && metaclass_conforms(owner_kind, c.owner)
                })
                .unwrap();
            let before = graph.clone();
            let outcome = move_contained_element(
                &mut graph,
                step["child"].as_str().unwrap(),
                owner,
                contract.field,
            );
            let valid = !step["recursive_destination"].as_bool().unwrap();
            assert_eq!(
                outcome.is_ok(),
                step["accepted"].as_bool().unwrap() && valid,
                "{step}"
            );
            if !valid {
                // EMF's generated inverse setter rejects its ancestor guard;
                // the direct list write bypasses it. We reject both atomically.
                assert_eq!(graph, before);
                if step["write_side"] == "forward" {
                    assert_eq!(step["accepted"], json!(true));
                    continue; // Explicit mutation-level disagreement, not a matching state.
                }
                assert_eq!(step["accepted"], json!(false));
            }
            if outcome.is_err() {
                assert_eq!(graph, before);
            }
            assert_eq!(step["after"].as_array().unwrap().len(), graph.len());
            for expected in step["after"].as_array().unwrap() {
                let actual = graph
                    .iter()
                    .find(|e| e.id == expected["id"].as_str().unwrap())
                    .unwrap();
                let fields = expected["fields"].as_object().unwrap();
                let contracts = generated::FEATURES.iter().filter(|c| {
                    c.kind == FeatureKind::Reference
                        && !c.derived
                        && !c.volatile
                        && (c.containment || c.container)
                        && metaclass_conforms(&actual.kind, c.owner)
                });
                assert_eq!(
                    fields.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                    contracts.map(|c| c.name).collect::<BTreeSet<_>>()
                );
                for (name, value) in fields {
                    let feature = generated::FEATURES
                        .iter()
                        .find(|c| c.name == name && metaclass_conforms(&actual.kind, c.owner))
                        .unwrap();
                    let absent = if feature.upper == 1 {
                        Value::Null
                    } else {
                        json!([])
                    };
                    assert_eq!(
                        actual.properties.get(feature.field).unwrap_or(&absent),
                        value,
                        "{}.{name}, move {} to {}",
                        actual.id,
                        step["child"],
                        step["owner"]
                    );
                }
            }
        }
    }

    #[test]
    fn required_stored_features_match_independent_emf_multiplicity_checks() {
        let evidence: Value = serde_json::from_str(include_str!(
            "../../../resources/metamodels/sysml-2.0-pilot-2026-08/required-features-pilot-controls.json"
        )).unwrap();
        let controls = evidence["controls"].as_array().unwrap();
        assert_eq!(controls.len(), 1536);
        let mut matched = 0;
        let mut unverified = 0;
        for control in controls {
            let kind = control["class"].as_str().unwrap();
            let name = control["feature"].as_str().unwrap();
            let contract = generated::FEATURES
                .iter()
                .find(|c| c.name == name && c.owner == control["owner"].as_str().unwrap())
                .unwrap();
            assert!(contract.lower > 0 && !contract.derived && !contract.volatile);
            let element = KirElement {
                id: "probe".into(),
                kind: kind.into(),
                layer: 2,
                properties: BTreeMap::new(),
            };
            let issues = assess_required_features(&[element]);
            let issue = issues.iter().find(|issue| issue.field == contract.field);
            if issue.is_some_and(|issue| issue.unverified) {
                unverified += 1;
            } else {
                assert_eq!(
                    Some(issue.is_none()),
                    control["valid"].as_bool(),
                    "{control}"
                );
                matched += 1;
            }
        }
        eprintln!("EMF required-feature observations: {matched} matching, {unverified} unverified");
        assert_eq!(matched, 1481);
        assert_eq!(unverified, 55);
        for &(kind, field) in required_dependencies::CONSTRUCTOR_DEPENDENCIES {
            let mut element = KirElement {
                id: "probe".into(),
                kind: kind.into(),
                layer: 2,
                properties: BTreeMap::new(),
            };
            assert!(
                !assess_required_features(&[element.clone()])
                    .iter()
                    .any(|issue| issue.field == field)
            );
            element.properties.insert(field.into(), json!("explicit"));
            assert!(
                !assess_required_features(&[element.clone()])
                    .iter()
                    .any(|issue| issue.field == field)
            );
            element.properties.insert(field.into(), Value::Null);
            assert!(
                assess_required_features(&[element])
                    .iter()
                    .any(|issue| issue.field == field && !issue.unverified)
            );
        }
    }

    #[test]
    fn definition_required_redefinitions_check_narrowed_and_original_bounds() {
        let graph = [
            KirElement { id: "doc".into(), kind: "SysML::Documentation".into(), layer: 2, properties: BTreeMap::new() },
            KirElement { id: "target".into(), kind: "SysML::Package".into(), layer: 2, properties: BTreeMap::new() },
        ];
        for (value, expected_invalid) in [(json!("target"), false), (Value::Null, true),
            (json!(["target"]), true), (json!(["target","target"]), true)] {
            let issues = assess_required_features_with(&graph, |_, contract| {
                if contract.field == "documented_element" { Ok(value.clone()) }
                else { Err("Other dependencies remain unassessed".into()) }
            });
            for field in ["annotated_element", "documented_element"] {
                let issue = issues.iter().find(|issue|issue.element_id == "doc" && issue.field == field);
                assert_eq!(issue.is_some(), expected_invalid, "{field}");
                if let Some(issue) = issue { assert!(!issue.unverified); }
            }
        }
    }

    #[test]
    fn required_semantic_values_obey_imported_bounds_types_and_dependencies() {
        let owner = KirElement { id: "feature".into(), kind: "SysML::Feature".into(), layer: 2, properties: BTreeMap::new() };
        let target = KirElement { id: "package".into(), kind: "SysML::Package".into(), layer: 2, properties: BTreeMap::new() };
        let graph = [owner, target];
        for (value, unverified) in [(Ok(json!("feature")), None), (Ok(Value::Null), Some(false)),
            (Ok(json!("package")), Some(false)), (Ok(json!(["feature", "feature"])), Some(false)),
            (Ok(json!("missing")), Some(true)),
            (Err("Unimplemented delegate dependency".into()), Some(true))] {
            let issues = assess_required_features_with(&graph, |_, contract| {
                if contract.field == "feature_target" { value.clone() }
                else { Err("Other delegate unassessed".into()) }
            });
            let issue = issues.iter().find(|issue| issue.element_id == "feature" && issue.field == "feature_target");
            assert_eq!(issue.map(|issue| issue.unverified), unverified);
            if let Some(issue) = issue { assert!(issue.feature_id.ends_with("Feature/featureTarget")); }
        }
    }

    #[test]
    fn property_redefinitions_select_effective_contracts_without_delegate_fallback() {
        for (kind, base, expected) in [
            ("Subclassification", "general", "superclassifier"),
            ("FeatureTyping", "general", "type"),
            ("Subsetting", "general", "subsetted_feature"),
            ("Redefinition", "general", "redefined_feature"),
            ("Redefinition", "specific", "redefining_feature"),
            ("Specialization", "general", "general"),
        ] {
            assert_eq!(redefined_feature(kind, base).unwrap().field, expected);
        }
        assert!(redefined_feature("AllocationUsage", "type").is_err());
        assert!(redefined_feature("Missing", "general").is_err());
        assert!(
            redefined_feature("ActorMembership", "owned_member_element")
                .unwrap()
                .derived
        );
    }

    use mercurio_foundation::kir::{KirDocument, KirFieldKind, KirFieldRegistry};
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn inherited_fields_and_reference_targets_follow_ecore() {
        let ownership = feature("SysML::Package", "owned_relationship").unwrap();
        assert_eq!(ownership.owner, "Element");
        assert_eq!(ownership.target, "Relationship");
        assert!(ownership.containment && ownership.ordered && ownership.unique);
        validate_reference_endpoint(
            "SysML::Package",
            "owned_relationship",
            "SysML::OwningMembership",
        )
        .unwrap();
        assert!(
            validate_reference_endpoint("SysML::Package", "owned_relationship", "SysML::Class")
                .is_err()
        );
        assert!(feature("UnknownClass", "owned_relationship").is_none());
    }

    #[test]
    fn attribute_type_and_reference_bounds_are_shared_checks() {
        validate_value("SysML::PartUsage", "is_unique", &json!(true)).unwrap();
        assert!(validate_value("SysML::PartUsage", "is_unique", &json!("true")).is_err());
        validate_value("SysML::MultiplicityRange", "bound", &json!(["a", "b"])).unwrap();
        assert!(validate_value("SysML::MultiplicityRange", "bound", &json!([])).is_err());
        assert!(
            validate_value("SysML::MultiplicityRange", "bound", &json!(["a", "b", "c"])).is_err()
        );
        assert!(validate_value("SysML::MultiplicityRange", "bound", &json!(["a", "a"])).is_err());
        // Ecore explicitly allows repeated related elements on a Relationship.
        validate_value("SysML::Relationship", "related_element", &json!(["a", "a"])).unwrap();
    }

    #[test]
    fn enum_and_integer_domains_follow_ecore() {
        for (kind, field, literal) in [
            ("PartUsage", "direction", "inout"),
            ("PartUsage", "portion_kind", "snapshot"),
            ("RequirementConstraintMembership", "kind", "assumption"),
            ("StateSubactionMembership", "kind", "entry"),
            ("TransitionFeatureMembership", "kind", "guard"),
            ("TriggerInvocationExpression", "kind", "when"),
            ("OwningMembership", "visibility", "protected"),
        ] {
            validate_value(kind, field, &json!(literal)).unwrap();
            assert!(validate_value(kind, field, &json!("unsupported")).is_err());
            assert!(validate_value(kind, field, &json!(0)).is_err());
        }
        // Grammar spellings and Ecore literals are not interchangeable.
        assert!(validate_value("OwningMembership", "visibility", &json!("expose")).is_err());
        validate_value("PartUsage", "direction", &Value::Null).unwrap();
        assert!(validate_value("OwningMembership", "visibility", &Value::Null).is_err());
        for value in [i32::MIN as i64, i32::MAX as i64] {
            validate_value("LiteralInteger", "value", &json!(value)).unwrap();
        }
        for value in [i32::MIN as i64 - 1, i32::MAX as i64 + 1] {
            assert!(validate_value("LiteralInteger", "value", &json!(value)).is_err());
        }
    }

    #[test]
    fn explicit_graph_checks_opposites_containers_and_cycles() {
        let element = |id: &str, kind: &str, fields: Value| KirElement {
            id: id.into(),
            kind: kind.into(),
            layer: 2,
            properties: fields
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        };
        let graph = vec![
            element("p", "Package", json!({"owned_relationship":["r"]})),
            element(
                "r",
                "OwningMembership",
                json!({"owning_related_element":"p", "owned_related_element":["child"]}),
            ),
            element("child", "PartUsage", json!({"owning_relationship":"r"})),
        ];
        assert!(validate_explicit_reference_graph(&graph).is_empty());
        let mut wrong_inverse = graph.clone();
        wrong_inverse[1]
            .properties
            .insert("owning_related_element".into(), json!(null));
        assert!(
            validate_explicit_reference_graph(&wrong_inverse)
                .iter()
                .any(|i| i.message.contains("opposite mismatch"))
        );
        let mut two_containers = graph.clone();
        two_containers.push(element("q", "Package", json!({"owned_relationship":["r"]})));
        assert!(
            validate_explicit_reference_graph(&two_containers)
                .iter()
                .any(|i| i.message.contains("conflicting containment"))
        );
        let cycle = vec![
            element("p", "Package", json!({"owned_relationship":["r"]})),
            element(
                "r",
                "OwningMembership",
                json!({"owned_related_element":["p"]}),
            ),
        ];
        assert_eq!(
            validate_explicit_reference_graph(&cycle)
                .iter()
                .filter(|i| i.message.contains("containment cycle"))
                .count(),
            1
        );
    }

    #[test]
    fn partial_graphs_do_not_invent_inverses_or_derived_containers() {
        let element = |id: &str, kind: &str, field: &str, value: Value| KirElement {
            id: id.into(),
            kind: kind.into(),
            layer: 2,
            properties: BTreeMap::from([(field.into(), value)]),
        };
        let graph = vec![
            element("p", "Package", "owned_relationship", json!(["external"])),
            element(
                "r",
                "OwningMembership",
                "owned_related_element",
                json!(["literal"]),
            ),
            element(
                "call",
                "InvocationExpression",
                "operand",
                json!(["literal"]),
            ),
            element("literal", "LiteralInteger", "value", json!(1)),
        ];
        assert!(validate_explicit_reference_graph(&graph).is_empty());
        assert!(!graph[3].properties.contains_key("owning_relationship"));
    }

    #[test]
    fn ecore_reconciled_visibility_survives_foundation_persistence() {
        let descriptor = |id: &str, owner: &str| KirElement {
            id: id.into(),
            kind: "MetamodelFeature".into(),
            layer: 2,
            properties: BTreeMap::from([
                ("owner".into(), json!(owner)),
                ("kir_property".into(), json!("visibility")),
                ("declared_name".into(), json!("visibility")),
                ("feature_kind".into(), json!("reference")),
            ]),
        };
        let mut document = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                descriptor("meta.import.visibility", "KerML::Root::Import"),
                descriptor("meta.membership.visibility", "KerML::Root::Membership"),
                KirElement {
                    id: "import.1".into(),
                    kind: "SysML::Import".into(),
                    layer: 2,
                    properties: BTreeMap::from([("visibility".into(), json!("private"))]),
                },
            ],
        };
        assert_eq!(
            reconcile_library_metafeatures_with_ecore(&mut document.elements).unwrap(),
            2
        );
        assert_eq!(
            reconcile_library_metafeatures_with_ecore(&mut document.elements).unwrap(),
            0
        );
        for descriptor in &document.elements[..2] {
            assert_eq!(descriptor.properties["feature_kind"], "attribute");
            assert_eq!(descriptor.properties["x_source_feature_kind"], "reference");
            assert!(
                descriptor.properties["x_ecore_feature_id"]
                    .as_str()
                    .unwrap()
                    .ends_with("/visibility")
            );
        }
        let normalized = document.normalized_for_persistence();
        assert_eq!(normalized.elements[2].properties["visibility"], "private");
        normalized.validate_persisted().unwrap();
    }

    #[test]
    fn plural_ecore_attributes_have_scoped_scalar_list_shapes() {
        let mut document = KirDocument {
            metadata: BTreeMap::new(),
            elements: vec![
                KirElement {
                    id: "meta.alias_ids".into(),
                    kind: "MetamodelFeature".into(),
                    layer: 2,
                    properties: BTreeMap::from([
                        ("owner".into(), json!("KerML::Root::Element")),
                        ("kir_property".into(), json!("alias_ids")),
                        ("feature_kind".into(), json!("reference")),
                    ]),
                },
                KirElement {
                    id: "meta.issue.text".into(),
                    kind: "MetamodelFeature".into(),
                    layer: 2,
                    properties: BTreeMap::from([
                        ("owner".into(), json!("ModelingMetadata::Issue")),
                        ("kir_property".into(), json!("text")),
                        ("feature_kind".into(), json!("attribute")),
                    ]),
                },
                KirElement {
                    id: "meta.requirement.text".into(),
                    kind: "MetamodelFeature".into(),
                    layer: 2,
                    properties: BTreeMap::from([
                        (
                            "owner".into(),
                            json!("SysML::Systems::RequirementDefinition"),
                        ),
                        ("kir_property".into(), json!("text")),
                        ("feature_kind".into(), json!("attribute")),
                    ]),
                },
                KirElement {
                    id: "meta.requirement_usage.text".into(),
                    kind: "MetamodelFeature".into(),
                    layer: 2,
                    properties: BTreeMap::from([
                        ("owner".into(), json!("SysML::Systems::RequirementUsage")),
                        ("kir_property".into(), json!("text")),
                        ("feature_kind".into(), json!("attribute")),
                    ]),
                },
                KirElement {
                    id: "e".into(),
                    kind: "SysML::Element".into(),
                    layer: 2,
                    properties: BTreeMap::from([("alias_ids".into(), json!(["a", "b"]))]),
                },
                KirElement {
                    id: "issue".into(),
                    kind: "Issue".into(),
                    layer: 2,
                    properties: BTreeMap::from([("text".into(), json!("one"))]),
                },
                KirElement {
                    id: "concern".into(),
                    kind: "SysML::ConcernDefinition".into(),
                    layer: 2,
                    properties: BTreeMap::from([("text".into(), json!(["one", "two"]))]),
                },
                KirElement {
                    id: "satisfy".into(),
                    kind: "SysML::SatisfyRequirementUsage".into(),
                    layer: 2,
                    properties: BTreeMap::from([("text".into(), json!(["three"]))]),
                },
            ],
        };
        assert_eq!(
            reconcile_library_metafeatures_with_ecore(&mut document.elements).unwrap(),
            1
        );
        assert_eq!(document.elements[0].properties["feature_kind"], "attribute");
        assert_eq!(
            document.elements[0].properties["x_source_feature_kind"],
            "reference"
        );
        assert_eq!(document.elements[0].properties["upper"], -1);
        assert_eq!(
            reconcile_library_metafeatures_with_ecore(&mut document.elements).unwrap(),
            0
        );
        let registry = KirFieldRegistry::from_document(&document);
        assert_eq!(
            registry
                .field_for("SysML::Element", "alias_ids")
                .unwrap()
                .kind,
            KirFieldKind::ScalarList
        );
        assert_eq!(
            registry.field_for("Issue", "text").unwrap().kind,
            KirFieldKind::Scalar
        );
        assert_eq!(
            registry
                .field_for("SysML::ConcernDefinition", "text")
                .unwrap()
                .kind,
            KirFieldKind::ScalarList
        );
        assert_eq!(
            registry
                .field_for("SysML::SatisfyRequirementUsage", "text")
                .unwrap()
                .kind,
            KirFieldKind::ScalarList
        );
        let normalized = document.normalized_for_persistence();
        assert_eq!(
            normalized.elements[4].properties["alias_ids"],
            json!(["a", "b"])
        );
        assert_eq!(normalized.elements[5].properties["text"], "one");
        assert_eq!(
            normalized.elements[6].properties["text"],
            json!(["one", "two"])
        );
        normalized.validate_persisted().unwrap();
    }
}
