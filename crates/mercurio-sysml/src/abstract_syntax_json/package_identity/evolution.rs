//! Explicit additions/deletions with deterministic allocation and cumulative retired IDs.
use super::*;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Changes {
    pub additions: Vec<String>,
    pub deletions: Vec<String>,
    pub retired_ids: Vec<String>,
}
fn changes(plan: &Continuity) -> Result<&Changes, String> {
    plan.changes
        .as_ref()
        .ok_or("Identity v2 requires explicit changes".into())
}
fn unique(values: &[String]) -> bool {
    values.iter().all(|s| !s.is_empty())
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
fn deleted(plan: &Continuity) -> Result<BTreeSet<String>, String> {
    let changes = changes(plan)?;
    if !unique(&changes.additions) || !unique(&changes.deletions) {
        return Err("Identity changes must be unique nonempty KIR IDs".into());
    }
    let mut requested: BTreeSet<_> = changes.deletions.iter().map(String::as_str).collect();
    let mut deleted = BTreeSet::new();
    for element in &plan.base.elements {
        if element["@type"] != "Package" {
            continue;
        }
        let id = element["@id"].as_str().ok_or("Base identity missing")?;
        let kir = plan
            .base
            .origins
            .get(id)
            .and_then(|o| o["kirId"].as_str())
            .ok_or("Base origin missing")?;
        if requested.remove(kir) {
            deleted.insert(id.to_string());
            if let Some(membership) = element["owningMembership"]["@id"].as_str() {
                deleted.insert(membership.to_string());
            }
        }
    }
    if !requested.is_empty() {
        return Err("Deletion source is absent from the base".into());
    }
    Ok(deleted)
}
pub(super) fn retired(plan: &Continuity) -> Result<Vec<String>, String> {
    let changes = changes(plan)?;
    let live: BTreeSet<_> = plan
        .base
        .elements
        .iter()
        .filter_map(|e| e["@id"].as_str())
        .collect();
    if changes
        .retired_ids
        .iter()
        .any(|id| !is_uuid_like(id) || live.contains(id.as_str()))
        || changes.retired_ids.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(
            "Retired identities must be sorted, unique and absent from the live base".into(),
        );
    }
    let mut result: BTreeSet<_> = changes.retired_ids.iter().cloned().collect();
    result.extend(deleted(plan)?);
    Ok(result.into_iter().collect())
}
fn allocated(plan: &Continuity, kir: &str, role: &str) -> String {
    deterministic_exchange_uuid(&format!(
        "mercurio-package-allocation-v1:{}:{role}:{kir}",
        plan.base_candidate_digest
    ))
}
pub(super) fn entries(plan: &Continuity) -> Result<BTreeMap<String, Entry>, String> {
    let changes = changes(plan)?;
    let deleted = deleted(plan)?;
    let retired = retired(plan)?;
    let mut original = plan.clone();
    original.format = "mercurio.sysml.package-identity.v1".into();
    original.changes = None;
    original.renames.clear();
    super::entries(&original)?;
    let mut preserved = plan.clone();
    preserved.format = "mercurio.sysml.package-identity.v1".into();
    preserved.changes = None;
    preserved
        .base
        .elements
        .retain(|e| !deleted.contains(e["@id"].as_str().unwrap_or("")));
    let mut entries = if preserved.base.elements.is_empty() && preserved.renames.is_empty() {
        BTreeMap::new()
    } else {
        super::entries(&preserved)?
    };
    let mut used: BTreeSet<String> = plan
        .base
        .elements
        .iter()
        .filter_map(|e| e["@id"].as_str().map(str::to_string))
        .collect();
    used.extend(retired);
    for kir in &changes.additions {
        let element_id = allocated(plan, kir, "package");
        let membership_id = allocated(plan, kir, "membership");
        if !used.insert(element_id.clone()) || !used.insert(membership_id.clone()) {
            return Err("Allocated identity collides with a live or retired identity".into());
        }
        if entries
            .insert(
                kir.clone(),
                Entry {
                    element_id,
                    membership_id: Some(membership_id),
                },
            )
            .is_some()
        {
            return Err("Addition collides with a surviving package mapping".into());
        }
    }
    Ok(entries)
}
pub(super) fn validate(plan: &Continuity, current: &IdentityBase) -> Result<(), String> {
    let entries = entries(plan)?;
    let retired: BTreeSet<_> = retired(plan)?.into_iter().collect();
    let previous: BTreeMap<_, _> = plan
        .base
        .elements
        .iter()
        .filter_map(|e| e["@id"].as_str().map(|id| (id, e)))
        .collect();
    let mut expected = BTreeSet::new();
    let mut packages = 0;
    for element in &current.elements {
        if element["@type"] != "Package" {
            continue;
        }
        packages += 1;
        let id = element["@id"].as_str().ok_or("Current identity missing")?;
        let origin = current.origins.get(id).ok_or("Current origin missing")?;
        let entry = origin["kirId"]
            .as_str()
            .and_then(|kir| entries.get(kir))
            .ok_or("Current package absent from identity mapping")?;
        if origin["kind"] != "authored"
            || entry.element_id != id
            || !expected.insert(id.to_string())
        {
            return Err("Current identity differs from explicit changes".into());
        }
        if let Some(old) = previous.get(id) {
            if old["@type"] != "Package" {
                return Err("Identity changed metaclass".into());
            }
            for key in [
                "owner",
                "owningNamespace",
                "owningMembership",
                "owningRelationship",
            ] {
                if element[key] != old[key] {
                    return Err("Identity changes cannot move surviving package ownership".into());
                }
            }
        }
        if let Some(owner) = element["owner"]["@id"].as_str() {
            let membership = entry
                .membership_id
                .as_deref()
                .ok_or("Membership identity missing")?;
            if element["owningMembership"]["@id"] != membership
                || !expected.insert(membership.to_string())
            {
                return Err("Membership identity differs from allocation".into());
            }
            let relation = current
                .elements
                .iter()
                .find(|e| e["@id"] == membership)
                .ok_or("Allocated membership absent")?;
            if relation["@type"] != "OwningMembership"
                || relation["source"] != json!([{"@id":owner}])
                || relation["target"] != json!([{"@id":id}])
                || current.origins.get(membership)
                    != Some(
                        &json!({"kind":"derived","rule":"package-ownership","sourceElements":[owner,id]}),
                    )
            {
                return Err("Membership endpoints or provenance differ".into());
            }
        } else if !element["owningMembership"].is_null() {
            return Err("Root cannot own a membership".into());
        }
    }
    let actual: BTreeSet<_> = current
        .elements
        .iter()
        .filter_map(|e| e["@id"].as_str().map(str::to_string))
        .collect();
    if packages != entries.len()
        || actual.len() != current.elements.len()
        || actual != expected
        || actual.iter().any(|id| retired.contains(id))
    {
        return Err(
            "Identity changes must cover all additions/deletions without reusing retired IDs"
                .into(),
        );
    }
    Ok(())
}

/// Bind cumulative state to the actual preceding candidate; no downgrade may discard it.
pub fn validate_state(plan: &Continuity, previous: &Value, current: &Value) -> Result<(), String> {
    let expected_base = previous.get("retiredIds").cloned().unwrap_or(json!([]));
    match &plan.changes {
        Some(changes) if plan.format == "mercurio.sysml.package-identity.v2" => {
            if json!(changes.retired_ids) != expected_base
                || *current != json!({"retiredIds":retired(plan)?})
            {
                return Err("Retired identity state differs from the preceding revision or explicit deletions".into());
            }
        }
        None if previous.is_null() && current.is_null() => {}
        _ => return Err("Retained identity state cannot be dropped or downgraded".into()),
    }
    Ok(())
}
