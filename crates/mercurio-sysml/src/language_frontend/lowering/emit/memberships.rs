//! Owning membership objects for explicit source declarations.
use super::*;
use super::super::relationship_declarations::metaclass_is;

pub(super) fn materialize(
    module: &ResolvedModule, ids: &BTreeMap<String, String>, source_file: &str,
    source_language: &str, elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    let visibility = |modifiers: &[String]| modifiers.iter().find(|m| matches!(m.as_str(), "public" | "private" | "protected"))
        .cloned().unwrap_or_else(|| "public".into());
    let mut visibilities = BTreeMap::new();
    for package in &module.packages { visibilities.insert(package.qualified_name.clone(), visibility(&package.modifiers)); }
    for definition in &module.definitions { visibilities.insert(definition.qualified_name.clone(), definition.visibility.clone()); }
    fn usages(values: &[ResolvedUsage], map: &mut BTreeMap<String, String>) {
        for value in values {
            map.insert(value.qualified_name.clone(), value.modifiers.iter().find(|m| matches!(m.as_str(), "public" | "private" | "protected"))
                .cloned().unwrap_or_else(|| "public".into()));
            usages(&value.members, map);
        }
    }
    usages(&module.usages, &mut visibilities);
    for definition in &module.definitions { usages(&definition.members, &mut visibilities); }
    let positions = elements.iter().enumerate().map(|(i,e)| (e.id.clone(), i)).collect::<BTreeMap<_, _>>();
    let mut root_needed = false;
    let mut additions = Vec::new();
    let mut declarations = ids.iter().collect::<Vec<_>>();
    declarations.sort_by_key(|(_, id)| positions.get(*id).map(|index| {
        let span = &elements[*index].properties["metadata"]["source_span"];
        (span["start_line"].as_u64().unwrap_or(0), span["start_col"].as_u64().unwrap_or(0))
    }));
    for (name, id) in declarations {
        let Some(&index) = positions.get(id) else { continue; };
        let element = &elements[index];
        if element.properties.contains_key("owning_membership") || element.properties.contains_key("owning_relationship") { continue; }
        let owner = element.properties.get("owner").and_then(Value::as_str).map(str::to_string)
            .or_else(|| name.rsplit_once('.').and_then(|(parent,_)| ids.get(parent).cloned()))
            .unwrap_or_else(|| "pkg.root".into());
        if owner == *id { continue; }
        let owner_kind = positions.get(&owner).map(|i| elements[*i].kind.as_str()).unwrap_or("Namespace");
        if !metaclass_is(owner_kind, "Namespace")? { continue; }
        let kind = if metaclass_is(&element.kind, "Feature")? && metaclass_is(owner_kind, "Type")? { "FeatureMembership" } else { "OwningMembership" };
        let membership = format!("membership.owned.{id}");
        let mut properties = BTreeMap::from([
            ("membership_owning_namespace".into(), json!(owner)),
            ("member_element".into(), json!(id)),
            ("owned_member_element".into(), json!(id)),
            ("source".into(), json!([owner])),
            ("target".into(), json!([id])),
            ("related_element".into(), json!([owner,id])),
            ("visibility".into(), json!(visibilities.get(name).map(String::as_str).unwrap_or("public"))),
            ("is_implied".into(), json!(false)),
        ]);
        for (source, targets) in [("declared_name", ["member_name", "owned_member_name"]), ("declared_short_name", ["member_short_name", "owned_member_short_name"])] {
            if let Some(value) = element.properties.get(source) { for target in targets { properties.insert(target.into(), value.clone()); } }
        }
        if kind == "FeatureMembership" { properties.insert("owning_type".into(), json!(owner)); }
        let mut metadata = element.properties.get("metadata").cloned().unwrap_or_else(|| json!({}));
        metadata["lowering"] = json!({"construct":kind,"metaclass":format!("SysML::{kind}")});
        properties.insert("metadata".into(), metadata);
        let mut relationship = KirElement { id: membership.clone(), kind: format!("SysML::{kind}"), layer: 2, properties };
        let element = &mut elements[index];
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut relationship, element)?;
        element.properties.insert("owner".into(), json!(owner));
        element.properties.insert("owning_namespace".into(), json!(owner));
        element.properties.insert("owning_membership".into(), json!(membership));
        if kind == "FeatureMembership" { element.properties.insert("owning_feature_membership".into(), json!(membership)); }
        if let Some(&parent) = positions.get(&owner) {
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, &mut elements[parent], &mut relationship)?;
            for field in ["owned_membership", "membership"] { append_unique_property_ref_list(&mut elements[parent].properties, field, &membership); }
        } else if owner == "pkg.root" { root_needed = true; }
        additions.push(relationship);
    }
    if root_needed && !positions.contains_key("pkg.root") {
        let owned = additions.iter().filter(|e| e.properties.get("membership_owning_namespace").is_some_and(|v| v == "pkg.root")).map(|e| e.id.clone()).collect::<Vec<_>>();
        elements.push(KirElement { id: "pkg.root".into(), kind: "SysML::Namespace".into(), layer: 2,
            properties: BTreeMap::from([("owned_relationship".into(), json!(owned)), ("owned_membership".into(), json!(owned)), ("membership".into(), json!(owned)),
                ("metadata".into(), json!({"source_file":source_file,"source_language":source_language,"generated":true,
                    "source_span":{"start_line":1,"start_col":1,"end_line":1,"end_col":1}}))]) });
    }
    if let Some(root) = elements.iter_mut().find(|e| e.id == "pkg.root") {
        for relationship in &mut additions {
            if relationship.properties.get("membership_owning_namespace").is_some_and(|v| v == "pkg.root") {
                ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS, root, relationship)?;
            }
        }
    }
    elements.extend(additions);
    Ok(())
}

/// Resource namespaces are anonymous and distinct even when documents are merged.
pub(super) fn qualify_resource_namespace(elements: &mut [KirElement], source_file: &str) {
    if !elements.iter().any(|e| e.id == "pkg.root" && e.kind == "SysML::Namespace"
        && e.properties["metadata"]["generated"] == true) { return; }
    let resource_id = format!("namespace.resource.{}", source_file.as_bytes().iter()
        .map(|byte| format!("{byte:02x}")).collect::<String>());
    fn replace(value: &mut Value, resource_id: &str) {
        match value {
            Value::String(reference) if reference == "pkg.root" => *reference = resource_id.to_string(),
            Value::Array(values) => { for value in values { replace(value, resource_id); } }
            _ => {}
        }
    }
    let mut fields = mercurio_foundation::kir::KirFieldRegistry::structural();
    fields.register_fields(crate::sysml_field_specs().iter().copied());
    for element in elements {
        if element.id == "pkg.root" { element.id = resource_id.clone(); }
        for (field, value) in &mut element.properties {
            if fields.field(field).is_some_and(|spec| matches!(spec.kind,
                mercurio_foundation::kir::KirFieldKind::Reference | mercurio_foundation::kir::KirFieldKind::ReferenceList)) {
                replace(value, &resource_id);
            }
        }
    }
}

/// Emission runs by declaration family; textual ownership is ordered by source.
pub(super) fn order_source_relationships(elements: &mut [KirElement]) {
    let positions = elements.iter().map(|element| {
        let span = &element.properties["metadata"]["source_span"];
        (element.id.clone(), (span["start_line"].as_u64().unwrap_or(u64::MAX),
            span["start_col"].as_u64().unwrap_or(u64::MAX)))
    }).collect::<BTreeMap<_, _>>();
    for element in elements {
        for field in ["owned_relationship", "owned_membership", "owned_import", "membership"] {
            if let Some(Value::Array(values)) = element.properties.get_mut(field) {
                values.sort_by_key(|value| value.as_str().and_then(|id| positions.get(id)).copied().unwrap_or((u64::MAX, u64::MAX)));
            }
        }
    }
}
