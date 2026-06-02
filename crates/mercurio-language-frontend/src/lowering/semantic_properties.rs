use serde_json::Value;

use mercurio_kir::KirElement;

use crate::lowering::emit::{
    MappingBundle, append_unique_property_ref, modifier_value, sibling_state_id,
};
use crate::lowering::ir::ResolvedUsage;
use crate::lowering::semantic_defaults::UsagePropertyDefaultSeed;

pub(crate) fn apply_usage_property_defaults(
    element: &mut KirElement,
    usage: &ResolvedUsage,
    owner_id: &str,
    mappings: &MappingBundle,
) {
    for default in mappings.usage_property_defaults(usage) {
        if let Some(kir_kind) = &default.kir_kind {
            element.kind = kir_kind.clone();
        }
        for (property, refs) in &default.property_refs {
            for value in refs {
                append_unique_property_ref(&mut element.properties, property, value);
            }
        }
        for (property, value) in &default.property_values {
            if let Some(value) = resolve_usage_property_default_value(value, usage, owner_id) {
                element
                    .properties
                    .insert(property.clone(), Value::String(value));
            }
        }
    }
}

pub(crate) fn usage_property_default_applies(
    default: &UsagePropertyDefaultSeed,
    usage: &ResolvedUsage,
) -> bool {
    if let Some(owner_construct) = &default.owner_construct
        && owner_construct != &usage.owner_construct
    {
        return false;
    }
    default
        .present_modifiers
        .iter()
        .all(|present| usage.modifiers.iter().any(|modifier| modifier == present))
        && default
            .absent_modifiers
            .iter()
            .all(|absent| !usage.modifiers.iter().any(|modifier| modifier == absent))
}

fn resolve_usage_property_default_value(
    value: &str,
    usage: &ResolvedUsage,
    owner_id: &str,
) -> Option<String> {
    let mut resolved = value.to_string();
    for (placeholder, replacement) in [
        ("$owner_id", Some(owner_id.to_string())),
        ("$qualified_name", Some(usage.qualified_name.clone())),
        ("$declared_name", Some(usage.declared_name.clone())),
        ("$allocation_source", usage.allocation_source.clone()),
        ("$allocation_target", usage.allocation_target.clone()),
        ("$reference_target", usage.reference_target.clone()),
        (
            "$metadata_body",
            usage.metadata_properties.get("body").cloned(),
        ),
        (
            "$metadata_locale",
            usage.metadata_properties.get("locale").cloned(),
        ),
        (
            "$modifier_value_trigger_kind",
            modifier_value(&usage.modifiers, "trigger_kind").map(str::to_string),
        ),
        (
            "$modifier_value_trigger",
            modifier_value(&usage.modifiers, "trigger").map(str::to_string),
        ),
        (
            "$sibling_state_id_transition_target",
            modifier_value(&usage.modifiers, "transition_target")
                .and_then(|target| sibling_state_id(&usage.owner_qualified_name, target)),
        ),
    ] {
        if !resolved.contains(placeholder) {
            continue;
        }
        let replacement = replacement?;
        resolved = resolved.replace(placeholder, &replacement);
    }
    Some(resolved)
}
