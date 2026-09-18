//! Explicit package identity continuity. No matching by names, spans or similarity.
use super::*;
mod evolution;
pub use evolution::{Changes, validate_state};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rename {
    pub from: String,
    pub to: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityBase {
    pub elements: Vec<Value>,
    pub origins: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Continuity {
    pub format: String,
    pub base_candidate_digest: String,
    pub base: IdentityBase,
    pub renames: Vec<Rename>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changes: Option<Changes>,
}
pub(super) struct Entry {
    pub element_id: String,
    pub membership_id: Option<String>,
}

fn entries(plan: &Continuity) -> Result<BTreeMap<String, Entry>, String> {
    if plan.format == "mercurio.sysml.package-identity.v2" {
        return evolution::entries(plan);
    }
    if plan.format != "mercurio.sysml.package-identity.v1" || plan.changes.is_some() {
        return Err("Unsupported identity format".into());
    }
    let mut renames = BTreeMap::new();
    for rename in &plan.renames {
        if rename.from == rename.to
            || renames
                .insert(rename.from.as_str(), rename.to.as_str())
                .is_some()
        {
            return Err("Rename sources must be unique and change the KIR ID".into());
        }
    }
    let mut result = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for element in &plan.base.elements {
        let id = element["@id"]
            .as_str()
            .ok_or("Base element identity missing")?;
        if !is_uuid_like(id) || !ids.insert(id) {
            return Err("Invalid or duplicate base identity".into());
        }
        if element["@type"] == "OwningMembership" {
            continue;
        }
        if element["@type"] != "Package" {
            return Err("Identity continuity supports package trees only".into());
        }
        let origin = plan.base.origins.get(id).ok_or("Base origin missing")?;
        if origin["kind"] != "authored" {
            return Err("Package must have an authored origin".into());
        }
        let kir = origin["kirId"].as_str().ok_or("Base KIR ID missing")?;
        let target = renames.remove(kir).unwrap_or(kir);
        let membership_id = element["owningMembership"]["@id"]
            .as_str()
            .map(str::to_string);
        if result
            .insert(
                target.to_string(),
                Entry {
                    element_id: id.to_string(),
                    membership_id,
                },
            )
            .is_some()
        {
            return Err("Rename targets collide with another package".into());
        }
    }
    if !renames.is_empty() || result.is_empty() {
        return Err("Rename source is absent from the base".into());
    }
    Ok(result)
}

/// Validate retained evidence against the actual preceding revision, without compiling source.
pub fn validate_lineage(
    plan: &Continuity,
    base_digest: &str,
    base: &IdentityBase,
    current: &IdentityBase,
) -> Result<(), String> {
    if plan.base_candidate_digest != base_digest
        || plan.base.elements != base.elements
        || plan.base.origins != base.origins
    {
        return Err("Identity base does not match the preceding candidate".into());
    }
    if plan.format == "mercurio.sysml.package-identity.v2" {
        return evolution::validate(plan, current);
    }
    let expected = entries(plan)?;
    let previous: BTreeMap<_, _> = base
        .elements
        .iter()
        .filter_map(|e| e["@id"].as_str().map(|id| (id, e)))
        .collect();
    if current.elements.len() != previous.len() {
        return Err("Identity continuity cannot add or remove elements".into());
    }
    let mut seen = BTreeSet::new();
    for element in &current.elements {
        let id = element["@id"].as_str().ok_or("Current identity missing")?;
        let old = previous
            .get(id)
            .ok_or("Identity continuity introduced an element")?;
        if !seen.insert(id) || element["@type"] != old["@type"] {
            return Err("Identity continuity changed a metaclass or duplicated an identity".into());
        }
        if element["@type"] == "Package" {
            let origin = current.origins.get(id).ok_or("Current origin missing")?;
            let kir = origin["kirId"].as_str().ok_or("Current KIR ID missing")?;
            if origin["kind"] != "authored"
                || expected.get(kir).map(|entry| entry.element_id.as_str()) != Some(id)
            {
                return Err("Current identity differs from the explicit rename mapping".into());
            }
        }
        if element["@type"] == "OwningMembership" && current.origins.get(id) != base.origins.get(id)
        {
            return Err("Derived identity origin differs from the base".into());
        }
        for property in [
            "owner",
            "owningMembership",
            "owningNamespace",
            "owningRelationship",
            "ownedElement",
            "ownedMember",
            "member",
            "ownedMembership",
            "membership",
            "ownedRelationship",
            "owningRelatedElement",
            "membershipOwningNamespace",
            "memberElement",
            "ownedMemberElement",
            "memberElementId",
            "ownedMemberElementId",
            "source",
            "target",
            "relatedElement",
            "ownedRelatedElement",
        ] {
            if element[property] != old[property] {
                return Err("Renames cannot change package ownership".into());
            }
        }
    }
    Ok(())
}

/// Pure shared export path; HTTP, CLI and desktop can consume the same identity rules.
pub fn export(document: &KirDocument, plan: &Continuity) -> Result<SysmlJsonExportReport, String> {
    let identities = entries(plan)?;
    let (elements, origins) =
        package_projection::project_with_identity(document, Some(&identities))?;
    let current = IdentityBase {
        elements: elements.clone(),
        origins: serde_json::from_value(origins.clone()).map_err(|e| e.to_string())?,
    };
    validate_lineage(plan, &plan.base_candidate_digest, &plan.base, &current)?;
    let mut report = export_sysml_abstract_syntax_value(
        document,
        SysmlJsonExportOptions {
            schema_profile: Some(package_projection::PROFILE.into()),
            include_mercurio_extensions: false,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    if report.has_errors() {
        return Err("Current document is outside the package profile".into());
    }
    if plan.changes.is_some() {
        report.metadata.insert(
            "identity_state".into(),
            json!({"retiredIds":evolution::retired(plan)?}),
        );
    }
    report.value["elements"] = json!(elements);
    report.metadata.insert("origins".into(), origins);
    report.metadata.insert(
        "reference_closure".into(),
        reference_closure::audit(&current.elements),
    );
    Ok(report)
}
