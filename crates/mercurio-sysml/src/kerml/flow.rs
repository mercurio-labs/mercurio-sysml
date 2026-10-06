//! Derived Flow references from pinned owned-membership and typing graphs.
use mercurio_foundation::kir::KirElement;

pub fn derived_flow_end_ids(flow: &KirElement, elements: &[KirElement]) -> Vec<String> {
    if !matches!(flow.kind.rsplit("::").next(), Some("Flow" | "SuccessionFlow")) {
        return Vec::new();
    }
    flow.properties.get("owned_relationship").and_then(|value| value.as_array())
        .into_iter().flatten().filter_map(|id| {
            let membership = elements.iter().find(|element| Some(element.id.as_str()) == id.as_str())?;
            if membership.kind.rsplit("::").next() != Some("EndFeatureMembership") { return None; }
            let member_id = membership.properties.get("member_element")?.as_str()?;
            let member = elements.iter().find(|element| element.id == member_id)?;
            (member.kind.rsplit("::").next() == Some("FlowEnd")
                && member.properties.get("owning_relationship")?.as_str()? == membership.id)
                .then(|| member_id.to_owned())
        }).collect()
}

pub fn derived_flow_payload_feature_id(flow: &KirElement, elements: &[KirElement]) -> Option<String> {
    if !matches!(flow.kind.rsplit("::").next(), Some("Flow" | "SuccessionFlow")) {
        return None;
    }
    flow.properties.get("owned_relationship")?.as_array()?.iter().find_map(|id| {
        let membership = elements.iter().find(|element| Some(element.id.as_str()) == id.as_str())?;
        if membership.kind.rsplit("::").next() != Some("FeatureMembership") { return None; }
        let member_id = membership.properties.get("member_element")?.as_str()?;
        let member = elements.iter().find(|element| element.id == member_id)?;
        (member.kind.rsplit("::").next() == Some("PayloadFeature")
            && member.properties.get("owning_relationship")?.as_str()? == membership.id)
            .then(|| member_id.to_owned())
    })
}

pub fn derived_flow_payload_type_ids(flow: &KirElement, elements: &[KirElement]) -> Vec<String> {
    let Some(feature_id) = derived_flow_payload_feature_id(flow, elements) else { return Vec::new() };
    let Some(feature) = elements.iter().find(|element| element.id == feature_id) else { return Vec::new() };
    feature.properties.get("type").and_then(|value| value.as_str())
        .map(|id| vec![id.to_owned()]).unwrap_or_default()
}

pub fn derived_flow_value_expression_id(flow: &KirElement, elements: &[KirElement]) -> Option<String> {
    if !matches!(flow.kind.rsplit("::").next(), Some("Flow" | "SuccessionFlow")) {
        return None;
    }
    flow.properties.get("owned_relationship")?.as_array()?.iter().find_map(|id| {
        let relation = elements.iter().find(|element| Some(element.id.as_str()) == id.as_str())?;
        if relation.kind.rsplit("::").next() != Some("FeatureValue") { return None; }
        let expression_id = relation.properties.get("owned_related_element")?.as_array()?
            .iter().find_map(|item| item.as_str())?;
        let expression = elements.iter().find(|element| element.id == expression_id)?;
        (expression.properties.get("owning_relationship")?.as_str()? == relation.id)
            .then(|| expression_id.to_owned())
    })
}
