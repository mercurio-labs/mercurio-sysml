//! Handwritten UsageUtil.getPreviousFeature and TransitionUsage.computeSource.
//! Imported Ecore contracts govern ancestry, reference ranges and ordered ownership.
//! Source membership is only one stage: never advertise additional-members readiness.
use super::*;
#[path = "transition_source_generated.rs"]
mod policy;
pub(super) fn member<'g>(graph:&'g [KirElement],m:&'g KirElement)->Result<Option<&'g KirElement>,Diagnostic> {
    match member_query(graph,m) {
        Err(QueryFailure::Required(Prerequisite::ReadField{..}))=>Err(error("transition alias reference is pending")),
        other=>other.map_err(QueryFailure::into_diagnostic),
    }
}
// Retain the actual imported stored endpoint as a link prerequisite. Missing
// storage is distinct from explicit null; unsupported delegates still reject.
pub(super) fn member_query<'g>(
    graph: &'g [KirElement],
    m: &'g KirElement,
) -> Result<Option<&'g KirElement>, QueryFailure> {
    let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
    member_query_in_view(&DefinitionNameQuery::new(graph,&index),m)
}

pub(super) fn member_query_in_view<'g>(query: &DefinitionNameQuery<'g, '_>, m: &'g KirElement) -> Result<Option<&'g KirElement>, QueryFailure> {
    if metaclass_conforms(&m.kind, "OwningMembership") {
        return match stored_membership_endpoint_indexed(query.graph.len(),query.index,m, "member_element")? {
            StoredMembershipEndpoint::Resolved(e) => Ok(Some(e)),
            _ => Err(query_failure(
                "transition source requires a resolved owned membership",
            )),
        };
    }
    let c = ecore_model::redefined_feature(&m.kind, "member_element").map_err(error)?;
    if c.derived || c.upper != 1 {
        return Err(query_failure("unassessed transition alias delegate"));
    }
    let value = m
        .properties
        .get(c.field)
        .ok_or_else(|| QueryFailure::Required(Prerequisite::ReadField {owner_id:m.id.clone(),field:c.field.into()}))?;
    if value.is_null() {
        return Ok(None);
    }
    let id = value
        .as_str()
        .ok_or_else(|| error("invalid source alias reference"))?;
    let e = if query.index.len()==query.graph.len() {query.index.get(id).copied()} else {query.graph.iter().find(|e|e.id==id)}
        .ok_or_else(|| error("source alias endpoint is absent"))?;
    ecore_model::validate_reference_endpoint(&m.kind, c.field, &e.kind).map_err(error)?;
    Ok(Some(e))

}
// Producer-disjointness projection of the guarded computeSource body. A
// resolved first nonparameter membership makes that producer a no-op; this
// proves only source stability, not completion of link/connector construction.
pub(super) fn membership_stable_query(graph:&[KirElement],owner:&KirElement)->Result<bool,QueryFailure> {
    let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
    membership_stable_in_view(&DefinitionNameQuery::new(graph,&index),owner)
}

pub(super) fn membership_stable_in_view(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement) -> Result<bool, QueryFailure> {
    if owner.kind.rsplit("::").next()!=Some("TransitionUsage"){return Err(query_failure("source stage requires exact Transition dispatch"));}
    let first=stored_children_projection_query(query,owner,"owned_relationship")?.into_iter().find(|e|metaclass_conforms(&e.kind,"Membership"));
    if policy::insert(first.is_none(),first.is_some_and(|e|metaclass_conforms(&e.kind,"ParameterMembership"))){return Ok(false);}
    Ok(member_query_in_view(query,first.ok_or_else(||query_failure("source stability requires the selected membership"))?)?.is_some())

}
fn previous<'g>(
    graph: &'g [KirElement],
    target: &KirElement,
) -> Result<Option<&'g KirElement>, Diagnostic> {
    let index = graph
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect::<BTreeMap<_, _>>();
    let Some(owning) = scope_container(&index, target)? else {
        return Ok(None);
    };
    if !metaclass_conforms(&owning.kind, "Membership") {
        return Ok(None);
    }
    let Some(owner) = scope_container(&index, owning)? else {
        return Ok(None);
    };
    if !metaclass_conforms(&owner.kind, "Type") {
        return Ok(None);
    }
    let members = stored_children(graph, owner, "owned_relationship")?
        .into_iter()
        .filter(|e| metaclass_conforms(&e.kind, "Membership"))
        .collect::<Vec<_>>();
    let at = members
        .iter()
        .position(|m| m.id == owning.id)
        .ok_or_else(|| error("owning membership is absent from namespace order"))?;
    for m in members[..at].iter().rev() {
        let Some(e) = member(graph, m)? else {
            continue;
        };
        let feature = metaclass_conforms(&e.kind, "Feature");
        let parameter = if feature {
            let owner = definition_feature_owner(graph, e)?;
            owner.is_some_and(|(_, o)| {
                metaclass_conforms(&o.kind, "Behavior") || metaclass_conforms(&o.kind, "Step")
            }) && definition_has_direction(e)?
        } else {
            false
        };
        let connector = metaclass_conforms(&e.kind, "Connector");
        let message = if metaclass_conforms(&e.kind, "FlowUsage") {
            definition_owned_features(graph, e, true)?.is_empty()
        } else {
            false
        };
        if policy::eligible(
            feature,
            parameter,
            metaclass_conforms(&e.kind, "TransitionUsage"),
            connector,
            message,
        ) {
            return Ok(Some(e));
        }
    }
    Ok(None)
}
fn append(
    graph: &mut Vec<KirElement>,
    target_id: &str,
    membership_id: &str,
) -> Result<bool, Diagnostic> {
    let target = graph
        .iter()
        .find(|e| e.id == target_id)
        .ok_or_else(|| error("transition source owner is absent"))?;
    if target.kind.rsplit("::").next() != Some("TransitionUsage") {
        return Err(error("unassessed transition source dispatch"));
    }
    let first = stored_children(graph, target, "owned_relationship")?
        .into_iter()
        .find(|e| metaclass_conforms(&e.kind, "Membership"));
    let insert = policy::insert(
        first.is_none(),
        first.is_some_and(|e| metaclass_conforms(&e.kind, "ParameterMembership")),
    );
    if !insert && member(graph, first.unwrap())?.is_some() {
        return Ok(false);
    }
    let selected=previous(graph,target)?.map(|e|e.id.clone()).ok_or_else(||error("transition source is unavailable; null Membership.member_element violates imported lower bound"))?;
    if insert {
        if membership_id.is_empty() || graph.iter().any(|e| e.id == membership_id) {
            return Err(error(
                "source construction requires a fresh nonempty membership identity",
            ));
        }
        let mut m = fresh_definition_element(membership_id.into(), policy::MEMBERSHIP)?;
        m.properties
            .insert("member_element".into(), Value::String(selected.clone()));
        let target = graph.iter_mut().find(|e| e.id == target_id).unwrap();
        ecore_ownership::attach(
            &ecore_ownership_generated::OWNED_RELATIONSHIPS,
            target,
            &mut m,
        )?;
        // Pilot inserts before all relationships, not just before memberships.
        let list = target
            .properties
            .get_mut("owned_relationship")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| error("source owner order is unavailable"))?;
        let id = list
            .pop()
            .ok_or_else(|| error("source membership was not attached"))?;
        list.insert(0, id);
        graph.push(m);
        return Ok(true);
    }
    let id = first.unwrap().id.clone();
    graph
        .iter_mut()
        .find(|e| e.id == id)
        .unwrap()
        .properties
        .insert("member_element".into(), json!(selected));
    Ok(true)
}
/// Atomic source stage. Does not construct transition links, connector transforms,
/// generalization contributions, featureTarget or validate transition constraints.
pub(crate) fn definition_materialize_transition_source(
    graph: &mut Vec<KirElement>,
    owner: &str,
    identity: &str,
) -> Result<bool, Diagnostic> {
    Ok(materialize(graph, identity, &[owner.to_owned()], true)? == 1)
}
pub(crate) fn definition_materialize_transition_sources(
    graph: &mut Vec<KirElement>,
    prefix: &str,
) -> Result<usize, Diagnostic> {
    let mut owners = graph
        .iter()
        .filter(|e| e.kind.rsplit("::").next() == Some("TransitionUsage"))
        .map(|e| e.id.clone())
        .collect::<Vec<_>>();
    owners.sort();
    materialize(graph, prefix, &owners, false)
}
fn materialize(
    graph: &mut Vec<KirElement>,
    prefix: &str,
    owners: &[String],
    single: bool,
) -> Result<usize, Diagnostic> {
    if prefix.is_empty() {
        return Err(error("source batch requires an identity prefix"));
    }
    // Only the explicitly null first plain source slot is admitted for repair.
    // All other invalid fields fail preflight, and every output is publication checked.
    let index = graph
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect::<BTreeMap<_, _>>();
    for issue in
        ecore_model::validate_publication(graph, ecore_model::ReferenceCompleteness::Closed)
    {
        let pending = if issue.field == "member_element" {
            if let Some(m) = graph.iter().find(|e| {
                e.id == issue.element_id
                    && e.kind.rsplit("::").next() == Some("Membership")
                    && e.properties.get("member_element") == Some(&Value::Null)
            }) {
                if let Some(t) = scope_container(&index, m)?
                    .filter(|e| e.kind.rsplit("::").next() == Some("TransitionUsage"))
                {
                    stored_children(graph, t, "owned_relationship")?
                        .into_iter()
                        .find(|e| metaclass_conforms(&e.kind, "Membership"))
                        .is_some_and(|first| first.id == m.id)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        if !pending {
            return Err(error(format!("invalid source input: {}", issue.message)));
        }
    }
    reject_result_snapshots(graph)?;
    let mut staged = graph.clone();
    let mut count = 0;
    for owner in owners {
        count += usize::from(append(
            &mut staged,
            owner,
            &if single {
                prefix.to_owned()
            } else {
                format!("{prefix}.{owner}.source")
            },
        )?);
    }
    if let Some(issue) =
        ecore_model::validate_publication(&staged, ecore_model::ReferenceCompleteness::Closed)
            .first()
    {
        return Err(error(format!("invalid source output: {}", issue.message)));
    }
    *graph = staged;
    Ok(count)
}
#[cfg(test)]
mod tests {
    #[test]
    fn definition_transition_alias_prerequisites_distinguish_null_and_invalid_storage() {
        use super::*;
        let mut graph=vec![fresh_definition_element("alias".into(),"Membership").unwrap(),fresh_definition_element("source".into(),"StateUsage").unwrap(),fresh_definition_element("invalid".into(),"Comment").unwrap()];
        assert!(matches!(member_query(&graph,&graph[0]),Err(QueryFailure::Required(Prerequisite::ReadField {owner_id,field})) if owner_id=="alias" && field=="member_element"));
        graph[0].properties.insert("member_element".into(),Value::Null);assert!(member_query(&graph,&graph[0]).unwrap().is_none());
        graph[0].properties.insert("member_element".into(),json!("source"));assert_eq!(member_query(&graph,&graph[0]).unwrap().unwrap().id,"source");
        for value in [json!("missing"),json!(42),json!(false)] {graph[0].properties.insert("member_element".into(),value);assert!(matches!(member_query(&graph,&graph[0]),Err(QueryFailure::Rejected(_))));}
        graph[0].kind="SysML::FeatureMembership".into();graph[0].properties.clear();assert!(matches!(member_query(&graph,&graph[0]),Err(QueryFailure::Rejected(_))));
    }

    use super::*;
    fn own(g: &mut Vec<KirElement>, owner: &str, mut e: KirElement, kind: &str) {
        let mut m = fresh_definition_element(format!("{}.membership", e.id), kind).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS, &mut m, &mut e)
            .unwrap();
        ecore_ownership::attach(
            &ecore_ownership_generated::OWNED_RELATIONSHIPS,
            g.iter_mut().find(|e| e.id == owner).unwrap(),
            &mut m,
        )
        .unwrap();
        g.extend([m, e]);
    }
    fn signature(g: &[KirElement]) -> Value {
        let t = g.iter().find(|e| e.id == "transition").unwrap();
        Value::Array(stored_children(g,t,"owned_relationship").unwrap().into_iter().filter(|e|metaclass_conforms(&e.kind,"Membership")).map(|m|json!({"membership":m.kind.rsplit("::").next().unwrap(),"member":member(g,m).unwrap().and_then(|e|e.properties.get("declared_name")).cloned().unwrap_or(Value::Null)})).collect())
    }
    fn fixture(c: &Value) -> Vec<KirElement> {
        let mut t = fresh_definition_element("transition".into(), "TransitionUsage").unwrap();
        t.properties
            .insert("declared_name".into(), json!("transition"));
        let mut g = vec![];
        let owner = c["owner"].as_str().unwrap();
        let prev = c["previous"].as_str().unwrap();
        if owner == "detached" {
            g.push(t);
        } else {
            g.push(fresh_definition_element("owner".into(), owner).unwrap());
            if prev != "none" {
                let kind = match prev {
                    "directed" => "Feature",
                    "message" | "end_flow" => "FlowUsage",
                    "nonfeature" => "Comment",
                    _ => prev,
                };
                let mut e = fresh_definition_element("previous".into(), kind).unwrap();
                e.properties
                    .insert("declared_name".into(), json!("previous"));
                if prev == "directed" {
                    e.properties.insert("direction".into(), json!("in"));
                }
                own(
                    &mut g,
                    "owner",
                    e,
                    if kind == "Comment" {
                        "OwningMembership"
                    } else {
                        "FeatureMembership"
                    },
                );
                if prev == "end_flow" {
                    let mut end = fresh_definition_element("end".into(), "Feature").unwrap();
                    end.properties.insert("is_end".into(), json!(true));
                    own(&mut g, "previous", end, "FeatureMembership");
                }
            }
            own(&mut g, "owner", t, "FeatureMembership");
        }
        match c["first"].as_str().unwrap() {
            "parameter" => {
                let mut p = fresh_definition_element("parameter".into(), "ReferenceUsage").unwrap();
                p.properties
                    .insert("declared_name".into(), json!("parameter"));
                own(&mut g, "transition", p, "ParameterMembership");
            }
            "empty_alias" | "explicit_alias" => {
                let mut m = fresh_definition_element("alias".into(), "Membership").unwrap();
                m.properties.insert("member_element".into(), Value::Null);
                if c["first"] == "explicit_alias" {
                    let mut e = fresh_definition_element("explicit".into(), "StateUsage").unwrap();
                    e.properties
                        .insert("declared_name".into(), json!("explicit"));
                    g.push(e);
                    m.properties
                        .insert("member_element".into(), json!("explicit"));
                }
                ecore_ownership::attach(
                    &ecore_ownership_generated::OWNED_RELATIONSHIPS,
                    g.iter_mut().find(|e| e.id == "transition").unwrap(),
                    &mut m,
                )
                .unwrap();
                g.push(m);
            }
            _ => {}
        }
        g
    }
    #[test]
    fn definition_transition_source_matches_pilot_and_persisted_replay() {
        let d: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/conformance/2026-08-support/transition-source-pilot-controls.json"
        )))
        .unwrap();
        assert_eq!(d["controls"].as_array().unwrap().len(), 180);
        for c in d["controls"].as_array().unwrap() {
            let mut g = fixture(c);
            assert_eq!(signature(&g), c["before"], "{c}");
            if c["after"][0]["member"].is_null() {
                let before = serde_json::to_value(&g).unwrap();
                assert!(
                    definition_materialize_transition_source(&mut g, "transition", "generated")
                        .unwrap_err()
                        .message
                        .contains("source is unavailable"),
                    "{c}"
                );
                assert_eq!(serde_json::to_value(g).unwrap(), before);
                continue;
            }
            definition_materialize_transition_source(&mut g, "transition", "generated")
                .unwrap_or_else(|e| panic!("{c}: {e:?}"));
            assert_eq!(signature(&g), c["after"], "{c}");
            g.reverse();
            let mut g: Vec<KirElement> =
                serde_json::from_value(serde_json::to_value(g).unwrap()).unwrap();
            let before = serde_json::to_value(&g).unwrap();
            assert_eq!(
                definition_materialize_transition_source(&mut g, "transition", "replay").unwrap(),
                false,
                "{c}"
            );
            assert_eq!(signature(&g), c["replay"], "{c}");
            assert_eq!(serde_json::to_value(g).unwrap(), before);
        }
    }
    #[test]
    fn definition_transition_source_skips_intervening_ineligible_members() {
        let c = json!({"owner":"StateDefinition","previous":"StateUsage","first":"none"});
        let mut g = fixture(&c);
        // Move three ineligible members before transition in canonical owner order.
        for (id, kind) in [
            ("skip_transition", "TransitionUsage"),
            ("skip_connector", "BindingConnector"),
            ("skip_comment", "Comment"),
        ] {
            own(
                &mut g,
                "owner",
                fresh_definition_element(id.into(), kind).unwrap(),
                if kind == "Comment" {
                    "OwningMembership"
                } else {
                    "FeatureMembership"
                },
            );
        }
        let list = g
            .iter_mut()
            .find(|e| e.id == "owner")
            .unwrap()
            .properties
            .get_mut("owned_relationship")
            .unwrap()
            .as_array_mut()
            .unwrap();
        let t = list.remove(1);
        list.push(t);
        definition_materialize_transition_sources(&mut g, "batch").unwrap();
        assert_eq!(signature(&g)[0]["member"], "previous");
    }
    #[test]
    fn definition_transition_source_rejects_snapshots_pending_and_rolls_back() {
        let c = json!({"owner":"Class","previous":"Feature","first":"none"});
        let mut g = fixture(&c);
        g.push(fresh_definition_element("batch.transition.source".into(), "Membership").unwrap());
        let before = serde_json::to_value(&g).unwrap();
        assert!(definition_materialize_transition_sources(&mut g, "batch").is_err());
        assert_eq!(serde_json::to_value(&g).unwrap(), before);
        let mut g = fixture(&c);
        g.iter_mut()
            .find(|e| e.id == "transition")
            .unwrap()
            .properties
            .insert("source".into(), json!("previous"));
        assert!(definition_materialize_transition_sources(&mut g, "batch").is_err());
        let mut g = fixture(&json!({"owner":"Class","previous":"Feature","first":"empty_alias"}));
        g.iter_mut()
            .find(|e| e.id == "alias")
            .unwrap()
            .properties
            .remove("member_element");
        assert!(definition_materialize_transition_sources(&mut g, "batch").is_err());
        // No readiness promotion: connector/additional-member lifecycle remains required.
        let mut g = fixture(&c);
        definition_materialize_transition_sources(&mut g, "batch").unwrap();
        own(
            &mut g,
            "transition",
            fresh_definition_element("succession".into(), "Succession").unwrap(),
            "OwningMembership",
        );
        assert!(!definition_additional_members_ready(
            &g,
            g.iter().find(|e| e.id == "transition").unwrap()
        )
        .unwrap());
    }
}
