use serde_json::{Map, Value};

use mercurio_kir::KirElement;

use crate::lowering::emit::MappingBundle;
use crate::lowering::ir::ResolvedUsage;
use crate::lowering::semantic_defaults::UsageActionSeed;

pub(crate) fn usage_action_applies(action: &UsageActionSeed, usage: &ResolvedUsage) -> bool {
    !action.requires_metadata_properties || !usage.metadata_properties.is_empty()
}

pub(crate) fn apply_usage_actions(
    elements: &mut [KirElement],
    element: &mut KirElement,
    usage: &ResolvedUsage,
    owner_id: &str,
    previous_state_id: Option<&str>,
    mappings: &MappingBundle,
) {
    for action in mappings.usage_actions(usage) {
        match action.action.as_str() {
            "attach_metadata_application" => {
                let Some(target) = action
                    .target
                    .as_deref()
                    .and_then(|target| resolve_usage_action_target(target, usage, owner_id))
                else {
                    continue;
                };
                attach_metadata_application(elements, &target, usage);
            }
            "source_from_previous_sibling_state" => {
                let Some(source_id) = previous_state_id else {
                    continue;
                };
                element
                    .properties
                    .insert("source".to_string(), Value::String(source_id.to_string()));
            }
            _ => {}
        }
    }
}

fn resolve_usage_action_target(
    value: &str,
    usage: &ResolvedUsage,
    owner_id: &str,
) -> Option<String> {
    match value {
        "$reference_target_or_owner" => Some(
            usage
                .reference_target
                .clone()
                .unwrap_or_else(|| owner_id.to_string()),
        ),
        "$reference_target" => usage.reference_target.clone(),
        "$owner_id" => Some(owner_id.to_string()),
        _ => Some(value.to_string()),
    }
}

fn attach_metadata_application(
    elements: &mut [KirElement],
    target_id: &str,
    usage: &ResolvedUsage,
) {
    let Some(target) = elements.iter_mut().find(|element| element.id == target_id) else {
        return;
    };

    let properties = usage
        .metadata_properties
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect::<Map<_, _>>();
    let mut annotation = Map::new();
    annotation.insert(
        "type".to_string(),
        Value::String(usage.declared_name.clone()),
    );
    annotation.insert("properties".to_string(), Value::Object(properties));

    let metadata = target
        .properties
        .entry("metadata".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !metadata.is_object() {
        *metadata = Value::Object(Map::new());
    }
    let Some(metadata_object) = metadata.as_object_mut() else {
        return;
    };
    metadata_object.insert(usage.declared_name.clone(), Value::Object(annotation));
}
