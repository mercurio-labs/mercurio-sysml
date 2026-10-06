//! RequirementConstraintMember ownership, with compatibility aliases in a curated overlay.
use super::*;

#[derive(Deserialize)]
struct Memberships {
    metaclass: String,
    construct_tokens: BTreeMap<String, String>,
    compatibility_id_prefixes: BTreeMap<String, String>,
}

pub(super) fn emit(
    usage: &ResolvedUsage,
    owner_id: &str,
    element: &mut KirElement,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    let rules = rules()?;
    let Some(token) = rules.construct_tokens.get(&usage.construct) else {
        return Ok(());
    };
    let kind = crate::enum_grammar::lookup(false, "RequirementConstraintKind", token)
        .ok_or_else(|| Diagnostic::new(format!("unmapped requirement constraint enum token: {token}"), Some(usage.span.clone())))?
        .literal;
    let id = format!("membership.requirement.{}", element.id);
    let visibility = usage
        .modifiers
        .iter()
        .find(|m| matches!(m.as_str(), "private" | "protected" | "public"))
        .map(String::as_str)
        .unwrap_or("public");
    let mut properties = BTreeMap::from([
        ("kind".into(), json!(kind)),
        ("visibility".into(), json!(visibility)),
        ("is_implied".into(), json!(false)),
        ("membership_owning_namespace".into(), json!(owner_id)),
        ("owning_type".into(), json!(owner_id)),
        ("member_element".into(), json!(element.id)),
        ("source".into(), json!([owner_id])),
        ("target".into(), json!([element.id])),
        ("related_element".into(), json!([owner_id, element.id])),
    ]);
    if !usage.is_implicit_name {
        properties.insert("member_name".into(), json!(usage.declared_name));
        properties.insert("owned_member_name".into(), json!(usage.declared_name));
    }
    if let Some(metadata) = element.properties.get("metadata") {
        let mut metadata = metadata.clone();
        if let Some(lowering) = metadata.get_mut("lowering").and_then(Value::as_object_mut) {
            lowering.insert("construct".into(), json!("RequirementConstraintMember"));
            lowering.insert("metaclass".into(), json!(rules.metaclass));
        }
        properties.insert("metadata".into(), metadata);
    }
    let mut relationship = KirElement { id: id.clone(), kind: rules.metaclass.clone(), layer: 2, properties };
    ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut relationship, element)?;
    ecore_ownership::attach_to_owner(owner_id, &mut relationship, elements)?;
    element
        .properties
        .insert("owning_membership".into(), json!(id));
    element
        .properties
        .insert("owning_feature_membership".into(), json!(id));
    if let Some(owner) = elements.iter_mut().find(|e| e.id == owner_id) {
        append_unique_property_ref_list(&mut owner.properties, "owned_membership", &id);
    }
    elements.push(relationship);
    Ok(())
}

fn rules() -> Result<&'static Memberships, Diagnostic> {
    static RULES: OnceLock<Result<Memberships, String>> = OnceLock::new();
    RULES.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../resources/metamodels/sysml-2.0-pilot-2026-08/requirement-memberships.overlay.json"
        )).map_err(|error| error.to_string())
    }).as_ref().map_err(|error| Diagnostic::new(format!("invalid requirement membership overlay: {error}"), None))
}

pub(super) fn compatibility_id_prefix(construct: &str) -> Result<Option<&'static str>, Diagnostic> {
    Ok(rules()?
        .compatibility_id_prefixes
        .get(construct)
        .map(String::as_str))
}
