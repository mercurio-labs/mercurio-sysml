//! Explicit handwritten execution of reviewed type-set delegates and six written
//! KerML predicates. Generated Ecore/Xtext roles select inputs; signatures alone
//! never admit a contribution. This does not evaluate set interpretation semantics.
use super::*;
#[path = "type_set_roles_generated.rs"]
mod roles;

fn delegate(contract: &ecore_model::FeatureContract) -> Result<(), QueryFailure> {
    let expected = format!("org.omg.sysml.delegate.setting.{}_{}_SettingDelegate", contract.owner, contract.name);
    if !contract.setting_delegate.as_ref().is_some_and(|d| d.uri == "http://www.omg.org/spec/SysML"
        && d.status == "custom_setting_delegate_source" && d.candidates == [expected.as_str()]) {
        return Err(query_failure("unreviewed type-set delegate"));
    }
    Ok(())
}

fn owning_type<'g>(query: &DefinitionNameQuery<'g, '_>, relation: &KirElement)
    -> Result<Option<&'g KirElement>, QueryFailure> {
    Ok(scope_container_query(query, relation)?.filter(|owner| metaclass_conforms(&owner.kind, "Type")))
}

#[cfg(test)]
fn endpoints<'g>(query: &DefinitionNameQuery<'g, '_>, relation: &KirElement, role: &roles::Binding)
    -> Result<(&'g KirElement, &'g KirElement), QueryFailure> {
    let source = if role.derived_source {
        let contract = ecore_model::feature(&relation.kind, role.source).ok_or_else(|| query_failure("missing type-set source contract"))?;
        delegate(contract)?;
        owning_type(query, relation)?.ok_or_else(|| query_failure("type-set source requires canonical Type ownership"))?
    } else { query_reference_in_view(query, relation, role.source)? };
    let target = query_reference_in_view(query, relation, role.target)?;
    ecore_model::validate_reference_endpoint(&relation.kind, role.source, &source.kind).map_err(query_failure)?;
    Ok((source, target))
}

/// Non-typing admission for the ordinary Feature/Connector provider only.
/// Keep explicitness, reciprocal ownership and adopted-subtree guards independent
/// of the public getter's broader structural domain.
pub(super) fn assess_contribution(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement, relation: &KirElement)
    -> Result<bool, QueryFailure> {
    let Some(role) = roles::binding(&relation.kind) else { return Ok(false); };
    if scope_boolean(relation, "is_implied")? {
        return Err(query_failure("unassessed implied type-set contribution"));
    }
    if owning_type(query, relation)?.map(|e| e.id.as_str()) != Some(owner.id.as_str()) {
        return Err(query_failure("type-set contribution requires reciprocal canonical Type ownership"));
    }
    if !relation.properties.contains_key("owned_related_element")
        || !stored_children_projection_query(query, relation, "owned_related_element")?.is_empty() {
        return Err(query_failure("type-set contribution has unassessed adopted subtree"));
    }
    let source = if role.derived_source {
        // Already checked reciprocal canonical Type ownership above.
        owner
    } else { query_reference_in_view(query, relation, role.source)? };
    if source.id != owner.id {
        return Err(query_failure("type-set contribution source differs from owning Type"));
    }
    if !relation.properties.contains_key(role.target) {
        let scope_only = if let Some((origin_id, field)) = query.type_set_scope_site {
            let origin = query.index.get(origin_id).copied().ok_or_else(|| query_failure("type-set scope origin is absent"))?;
            roles::binding(&origin.kind).is_some_and(|binding| field == binding.target)
                && owning_type(query, origin)?.is_some_and(|source| source.id == owner.id)
        } else { false };
        if !scope_only {
            query_reference_in_view(query, relation, role.target)?;
        }
        // This is solely the non-typing input classifier for an endpoint's
        // inherited name lookup. No getter result or validation is fabricated.
        // Candidate linking and required-field assessment still require targets.
    } else { query_reference_in_view(query, relation, role.target)?; }
    Ok(true)
}

fn owned<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &'g KirElement, role: &roles::Binding)
    -> Result<Vec<&'g KirElement>, QueryFailure> {
    let contract = ecore_model::feature(&owner.kind, role.owned).ok_or_else(|| query_failure("missing type-set owned projection"))?;
    if role.derived_source {
        if !contract.setting_delegate.as_ref().is_some_and(|d| d.uri == "http://www.omg.org/spec/SysML"
            && d.status == "default_setting_delegate_fallback_source" && d.candidates.is_empty())
            || !ecore_model::has_subset_contract("Type", role.owned, "owned_relationship") {
            return Err(query_failure("unreviewed owned type-set fallback/subset contract"));
        }
    } else { delegate(contract)?; }
    if !owner.properties.contains_key("owned_relationship") {
        return Err(query_failure("type-set projection requires complete canonical relationships"));
    }
    let mut result = Vec::new();
    for relation in stored_children_projection_query(query, owner, "owned_relationship")? {
        if !metaclass_conforms(&relation.kind, contract.target) { continue; }
        // Disjoining has a stored source, independently of its canonical owner.
        // Its upstream owned getter excludes foreign sources; derived source
        // roles follow canonical containment instead.
        if !role.derived_source && query_reference_in_view(query, relation, role.source)?.id != owner.id { continue; }
        result.push(relation);
    }
    Ok(result)
}

/// Handwritten owned-relationship setter dependency: assigning the imported
/// Disjoining.owningType role also initializes its stored typeDisjoined source.
/// Explicit source values and pending grammar references are never overwritten.
pub(super) fn automatic_source<'g>(query: &DefinitionNameQuery<'g, '_>, relation: &KirElement)
    -> Result<Option<(&'static str, String)>, Diagnostic> {
    let Some(role) = roles::binding(&relation.kind).filter(|r| !r.derived_source) else { return Ok(None); };
    if relation.properties.contains_key(role.source) { return Ok(None); }
    let contract = ecore_model::feature(&relation.kind, role.owning).ok_or_else(|| error("missing type-set setter role"))?;
    delegate(contract).map_err(|e| e.into_diagnostic_at("type-set source setter"))?;
    let Some(owner) = scope_container_query(query, relation)?.filter(|owner| metaclass_conforms(&owner.kind, "Type")) else { return Ok(None); };
    ecore_model::validate_reference_endpoint(&relation.kind, role.source, &owner.kind).map_err(error)?;
    Ok(Some((role.source, owner.id.clone())))
}

/// Public getters derive from canonical storage each time. Implied/subtree
/// lifecycle qualification is deliberately separate from structural projection.
pub(super) fn project<'g>(query: &DefinitionNameQuery<'g, '_>, owner: &'g KirElement,
    contract: &ecore_model::FeatureContract) -> Result<Option<Vec<&'g KirElement>>, QueryFailure> {
    if let Some(role) = roles::binding(&owner.kind) {
        if contract.field == role.owning || role.derived_source && contract.field == role.source {
            delegate(contract)?;
            return Ok(Some(owning_type(query, owner)?.into_iter().collect()));
        }
    }
    if !metaclass_conforms(&owner.kind, "Type") { return Ok(None); }
    for role in roles::BINDINGS {
        if contract.field == role.owned { return owned(query, owner, role).map(Some); }
        if role.endpoints == Some(contract.field) {
            delegate(contract)?;
            let mut targets = owned(query, owner, role)?.into_iter()
                .map(|r| query_reference_in_view(query, r, role.target)).collect::<Result<Vec<_>, _>>()?;
            if contract.unique {
                let mut seen = BTreeSet::new();
                targets.retain(|e| seen.insert(e.id.as_str()));
            }
            return Ok(Some(targets));
        }
    }
    Ok(None)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TypeSetIssue {
    pub type_id: String,
    pub constraint: &'static str,
    pub message: String,
    pub unverified: bool,
}

/// Partial semantic validation of exactly the six normative Type-set predicates.
/// Passing this assessor is not full validation of Type or relationship semantics.
pub(crate) fn assess(elements: &[KirElement]) -> Result<Vec<TypeSetIssue>, Diagnostic> {
    if let Some(issue) = ecore_model::validate_publication(elements, ecore_model::ReferenceCompleteness::Closed).first() {
        return Err(error(format!("invalid Type-set assessment input: {}.{}: {}", issue.element_id, issue.field, issue.message)));
    }
    let index = elements.iter().map(|e| (e.id.as_str(), e)).collect::<BTreeMap<_, _>>();
    let query = DefinitionNameQuery::new(elements, &index);
    let mut issues = Vec::new();
    for owner in elements.iter().filter(|e| metaclass_conforms(&e.kind, "Type")) {
        for role in roles::BINDINGS {
            let (Some(count_constraint), Some(self_constraint)) = (role.count_constraint, role.self_constraint) else { continue; };
            let mut add = |constraint, message: String, unverified| issues.push(TypeSetIssue {
                type_id: owner.id.clone(), constraint, message, unverified,
            });
            match owned(&query, owner, role) {
                Ok(relations) => {
                    if relations.len() == 1 { add(count_constraint, "Type must not own exactly one type-set relationship of this kind".into(), false); }
                    let mut targets = Vec::new();
                    for relation in relations {
                        match query_reference_in_view(&query, relation, role.target) {
                            Ok(target) => targets.push(target),
                            Err(failure) => add(self_constraint, failure.into_diagnostic_at("Type-set self predicate").message, true),
                        }
                    }
                    if targets.iter().any(|e| e.id == owner.id) { add(self_constraint, "Type-set endpoints must exclude the source Type itself".into(), false); }
                },
                Err(failure) => {
                    let message = failure.into_diagnostic_at("Type-set owned predicate").message;
                    add(count_constraint, message.clone(), true); add(self_constraint, message, true);
                },
            }
        }
    }
    issues.sort_by(|a,b| (&a.type_id,a.constraint,&a.message,a.unverified).cmp(&(&b.type_id,b.constraint,&b.message,b.unverified)));
    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(kind: &str, receiver: &str, mask: u64, owner_kind: &str, composite: bool, portion: bool, target_kind: &str) -> Vec<KirElement> {
        let role = roles::binding(kind).unwrap();
        let mut subject = fresh_definition_element("receiver".into(), receiver).unwrap();
        subject.properties.insert("is_implied_included".into(), json!(true));
        subject.properties.insert("is_composite".into(), json!(composite));
        subject.properties.insert("is_portion".into(), json!(portion));
        let mut graph = Vec::new();
        for (bit,kind) in ["Class","Structure","DataType"].iter().enumerate() {
            if mask & (1 << bit) == 0 { continue; }
            let mut target = fresh_definition_element(format!("typing{bit}"),kind).unwrap();
            target.properties.insert("declared_name".into(),json!(format!("typing{bit}")));
            target.properties.insert("is_implied_included".into(),json!(true));
            let mut relation = fresh_definition_element(format!("typing.relation{bit}"),"FeatureTyping").unwrap();
            relation.properties.insert("typed_feature".into(),json!(subject.id));
            relation.properties.insert("type".into(),json!(target.id));
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,&mut subject,&mut relation).unwrap();
            graph.extend([target,relation]);
        }
        let mut relation = fresh_definition_element("constraint".into(),kind).unwrap();
        let mut target = fresh_definition_element("constraintTarget".into(),target_kind).unwrap();
        target.properties.insert("declared_name".into(),json!("constraintTarget"));
        target.properties.insert("is_implied_included".into(),json!(true));
        relation.properties.insert(role.target.into(),json!(target.id));
        if !role.derived_source { relation.properties.insert(role.source.into(),json!(subject.id)); }
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,&mut subject,&mut relation).unwrap();
        let mut owner = fresh_definition_element("owner".into(),owner_kind).unwrap();
        let mut member = fresh_definition_element("member".into(),if owner_kind=="Package" {"OwningMembership"} else {"FeatureMembership"}).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut member,&mut subject).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,&mut owner,&mut member).unwrap();
        graph.extend([subject,owner,member,relation,target]); graph
    }

    fn find<'g>(graph: &'g [KirElement], id: &str) -> &'g KirElement { graph.iter().find(|e|e.id==id).unwrap() }
    fn names(elements: Vec<&KirElement>) -> Value {
        json!(elements.iter().map(|e| e.properties["declared_name"].as_str().unwrap()).collect::<Vec<_>>())
    }

    #[test]
    fn definition_type_set_contributions_match_all_pinned_getter_controls() {
        let cache: Value = serde_json::from_str(include_str!("../../../../../../docs/conformance/2026-08-support/type-set-contribution-pilot-controls.json")).unwrap();
        let cases = cache["cases"].as_array().unwrap(); assert_eq!(cases.len(),2304);
        for case in cases {
            let receiver=case["receiver_kind"].as_str().unwrap();
            let kind=case["relation"].as_str().unwrap();
            let graph=fixture(kind,receiver,case["typing_mask"].as_u64().unwrap(),case["owner_kind"].as_str().unwrap(),
                case["composite"].as_bool().unwrap(),case["portion"].as_bool().unwrap(),case["target_kind"].as_str().unwrap());
            let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
            let query=DefinitionNameQuery::new(&graph,&index);
            let subject=find(&graph,"receiver"); let relation=find(&graph,"constraint");
            let before=serde_json::to_value(&graph).unwrap();
            assert!(assess_contribution(&query,subject,relation).unwrap(),"{case}");
            let (source,target)=endpoints(&query,relation,roles::binding(kind).unwrap()).unwrap();
            assert_eq!(source.id,subject.id); assert_eq!(target.kind.rsplit("::").next(),case["target_kind"].as_str());
            assert_eq!(names(definition_feature_types_in_view(&query,subject).unwrap()),case["observation"]["adapter_all_types"],"{case}");
            assert_eq!(names(definition_reference_targets_typed_query(&query,subject,"type").unwrap()),case["observation"]["feature_property_types"],"{case}");
            let generals=definition_general_type_inputs_with_view(&query,subject,"").unwrap();
            assert_eq!(names(generals.iter().map(|id|find(&graph,id)).collect()),case["observation"]["general_types"],"{case}");
            let inputs=definition_ordinary_feature_inputs_in_view(&query,subject).unwrap();
            let default = if receiver=="Feature" { definition_feature_default_from_inputs(subject,inputs).unwrap() }
                else { connector_defaults_generated::default_for(receiver,0,inputs.0).unwrap() };
            // Upstream controls have zero-end supplied-completed bindings.
            // Compare the imported selector, not the guarded cold lifecycle API.
            assert_eq!(default,case["observation"]["default_supertype"].as_str().unwrap(),"{case}");
            assert_eq!(serde_json::to_value(&graph).unwrap(),before);
        }
    }

    #[test]
    fn definition_type_set_contributions_preserve_boundaries_and_recompute() {
        for role in roles::BINDINGS {
            let base=fixture(role.kind,"Feature",4,"Package",false,false,"Class");
            for boundary in ["pending","wrong_kind","bad_scalar","source_mismatch","implied","subtree","missing_subtree","broken_owner"] {
                if boundary=="source_mismatch" && role.derived_source { continue; }
                let mut graph=base.clone();
                let relation=graph.iter_mut().find(|e|e.id=="constraint").unwrap();
                match boundary {
                    "pending"=>{relation.properties.remove(role.target);},
                    "wrong_kind"=>{relation.properties.insert(role.target.into(),json!("owner"));},
                    "bad_scalar"=>{relation.properties.insert(role.target.into(),json!(["constraintTarget"]));},
                    "source_mismatch"=>{relation.properties.insert(role.source.into(),json!("constraintTarget"));},
                    "implied"=>{relation.properties.insert("is_implied".into(),json!(true));},
                    "missing_subtree"=>{relation.properties.remove("owned_related_element");},
                    "broken_owner"=>{relation.properties.insert("owning_related_element".into(),json!("constraintTarget"));},
                    _=>{
                        let mut adopted=fresh_definition_element("adopted".into(),"Feature").unwrap();
                        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,relation,&mut adopted).unwrap();graph.push(adopted);
                    },
                }
                let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
                let query=DefinitionNameQuery::new(&graph,&index);let before=serde_json::to_value(&graph).unwrap();
                let result=assess_contribution(&query,find(&graph,"receiver"),find(&graph,"constraint"));
                if boundary=="pending" {
                    assert!(matches!(result,Err(QueryFailure::Required(Prerequisite::ReadField{owner_id,field})) if owner_id=="constraint" && field==role.target));
                    let mut scoped=DefinitionNameQuery::new(&graph,&index);
                    scoped.type_set_scope_site=Some((find(&graph,"constraint").id.as_str(),role.target));
                    assert!(assess_contribution(&scoped,find(&graph,"receiver"),find(&graph,"constraint")).unwrap());
                    // The public endpoint projection remains required even in
                    // this scope view, and the required-feature assessor still
                    // reports the missing stored endpoint.
                    if let Some(field)=role.endpoints {
                        assert!(matches!(definition_reference_targets_typed_query(&scoped,find(&graph,"receiver"),field),Err(QueryFailure::Required(_))));
                    }
                    assert!(ecore_model::assess_required_features(&graph).iter().any(|issue|
                        issue.element_id=="constraint" && issue.field==role.target && !issue.unverified));
                    scoped.type_set_scope_site=Some((find(&graph,"constraint").id.as_str(),role.source));
                    assert!(matches!(assess_contribution(&scoped,find(&graph,"receiver"),find(&graph,"constraint")),Err(QueryFailure::Required(_))));
                } else {assert!(matches!(result,Err(QueryFailure::Rejected(_))),"{} {boundary}",role.kind);}
                assert_eq!(before,serde_json::to_value(&graph).unwrap());
                // Repair into a fresh view; failed/missing inputs must not cache success.
                let mut restored:Vec<KirElement>=serde_json::from_value(serde_json::to_value(&base).unwrap()).unwrap();
                restored.reverse();
                let index=restored.iter().map(|e|(e.id.as_str(),e)).collect();
                let query=DefinitionNameQuery::new(&restored,&index);
                assert!(assess_contribution(&query,find(&restored,"receiver"),find(&restored,"constraint")).unwrap());
            }
        }
    }

    #[test]
    fn definition_type_set_normative_count_self_and_projection_boundaries() {
        // KerML PDF pp. 175-178: owned cardinality excludes exactly one;
        // endpoint types exclude self. Relationship count and unique endpoint
        // count are distinct, including repeated targets on distinct relations.
        for role in roles::BINDINGS.iter().filter(|b|b.endpoints.is_some()) {
            for count in 0..4 { for self_target in [false,true] { for repeated in [false,true] {
                let mut owner=fresh_definition_element("owner".into(),"Class").unwrap();
                let mut graph=Vec::new();
                for ordinal in 0..count {
                    let mut relation=fresh_definition_element(format!("relation{ordinal}"),role.kind).unwrap();
                    let target_id=if self_target && ordinal+1==count {"owner".to_owned()}
                        else if repeated {"target0".to_owned()} else {format!("target{ordinal}")};
                    relation.properties.insert(role.target.into(),json!(target_id));
                    if !graph.iter().any(|e:&KirElement|e.id==target_id) && target_id!="owner" {
                        graph.push(fresh_definition_element(target_id.clone(),"Class").unwrap());
                    }
                    ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,&mut owner,&mut relation).unwrap();
                    graph.push(relation);
                }
                graph.push(owner);
                let expected_count=count==1;let expected_self=count>0 && self_target;
                for mut model in [graph.clone(),serde_json::from_str::<Vec<KirElement>>(&serde_json::to_string(&graph).unwrap()).unwrap()] {
                    model.reverse();
                    let index=model.iter().map(|e|(e.id.as_str(),e)).collect();
                    let query=DefinitionNameQuery::new(&model,&index);let owner=find(&model,"owner");
                    assert_eq!(definition_reference_targets_typed_query(&query,owner,role.owned).unwrap().len(),count);
                    let targets=definition_reference_targets_typed_query(&query,owner,role.endpoints.unwrap()).unwrap();
                    let expected_len=if repeated {usize::from(count>usize::from(self_target)) + usize::from(expected_self)} else {count};
                    assert_eq!(targets.len(),expected_len,"{} {count} {self_target} {repeated}",role.kind);
                    let issues=assess(&model).unwrap();
                    assert!(!issues.iter().any(|e|e.unverified));
                    assert_eq!(issues.iter().any(|e|e.type_id=="owner" && Some(e.constraint)==role.count_constraint),expected_count);
                    assert_eq!(issues.iter().any(|e|e.type_id=="owner" && Some(e.constraint)==role.self_constraint),expected_self);
                }
            }}}
        }
        // Disjoining.owningType follows canonical ownership, while its stored
        // source and custom ownedDisjoining selection remain independent.
        let mut graph=fixture("Disjoining","Feature",0,"Package",false,false,"Class");
        graph.iter_mut().find(|e|e.id=="constraint").unwrap().properties.insert("type_disjoined".into(),json!("constraintTarget"));
        let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
        let query=DefinitionNameQuery::new(&graph,&index);
        assert!(definition_reference_targets_typed_query(&query,find(&graph,"receiver"),"owned_disjoining").unwrap().is_empty());
        assert_eq!(definition_reference_targets_typed_query(&query,find(&graph,"constraint"),"owning_type").unwrap()[0].id,"receiver");
    }

    #[test]
    fn definition_type_set_validation_and_projections_match_independent_pilot() {
        let cache:Value=serde_json::from_str(include_str!("../../../../../../docs/conformance/2026-08-support/type-set-validation-pilot-controls.json")).unwrap();
        let cases=cache["cases"].as_array().unwrap();assert_eq!(cases.len(),108);
        for case in cases {
            let role=roles::binding(case["relation"].as_str().unwrap()).unwrap();
            let mut subject=fresh_definition_element("receiver".into(),case["receiver_kind"].as_str().unwrap()).unwrap();
            subject.properties.insert("declared_name".into(),json!("receiver"));
            subject.properties.insert("is_implied_included".into(),json!(true));
            let count=case["count"].as_u64().unwrap() as usize;
            let pattern=case["pattern"].as_str().unwrap();let mut graph=Vec::new();
            for ordinal in 0..count {
                let target_id=if pattern=="self" && ordinal+1==count {"receiver".to_owned()}
                    else if pattern=="duplicate" {"target0".to_owned()} else {format!("target{ordinal}")};
                if target_id!="receiver" && !graph.iter().any(|e:&KirElement|e.id==target_id) {
                    let mut target=fresh_definition_element(target_id.clone(),"Class").unwrap();
                    target.properties.insert("declared_name".into(),json!(target_id));graph.push(target);
                }
                let mut relation=fresh_definition_element(format!("relation{ordinal}"),role.kind).unwrap();
                relation.properties.insert("declared_name".into(),json!(relation.id));relation.properties.insert(role.target.into(),json!(target_id));
                ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,&mut subject,&mut relation).unwrap();graph.push(relation);
            }
            graph.push(subject);
            let expected_codes=case["diagnostics"].as_array().unwrap().iter().map(|d|d["code"].as_str().unwrap().to_owned()).collect::<BTreeSet<_>>();
            for mut model in [graph.clone(),serde_json::from_str::<Vec<KirElement>>(&serde_json::to_string(&graph).unwrap()).unwrap()] {
                model.reverse();let before=serde_json::to_value(&model).unwrap();
                let index=model.iter().map(|e|(e.id.as_str(),e)).collect();let query=DefinitionNameQuery::new(&model,&index);
                let subject=find(&model,"receiver");
                assert_eq!(names(definition_reference_targets_typed_query(&query,subject,role.owned).unwrap()),case["owned"],"{case}");
                assert_eq!(names(definition_reference_targets_typed_query(&query,subject,role.endpoints.unwrap()).unwrap()),case["endpoints"],"{case}");
                let issues=assess(&model).unwrap();assert!(!issues.iter().any(|e|e.unverified));
                // Pilot cardinality diagnostic codes omit the normative "Type"
                // prefix. Keep that exact naming disposition explicit.
                let observed_codes=issues.iter().filter(|e|e.type_id=="receiver").map(|e|
                    e.constraint.replacen("validateTypeOwned","validateOwned",1)).collect::<BTreeSet<_>>();
                assert_eq!(observed_codes,expected_codes,"{case}");
                assert_eq!(serde_json::to_value(&model).unwrap(),before);
            }
        }
        for case in cache["boundaries"].as_array().unwrap() {
            let mut graph=fixture("Disjoining","Feature",0,"Package",false,false,"Class");
            match case["mode"].as_str().unwrap() {
                "foreign_source"=>{graph.iter_mut().find(|e|e.id=="constraint").unwrap().properties.insert("type_disjoined".into(),json!("constraintTarget"));},
                "package_owner"=>{
                    graph.iter_mut().find(|e|e.id=="receiver").unwrap().properties.insert("owned_relationship".into(),json!([]));
                    graph.iter_mut().find(|e|e.id=="owner").unwrap().properties.insert("owned_relationship".into(),json!(["member","constraint"]));
                    graph.iter_mut().find(|e|e.id=="constraint").unwrap().properties.insert("owning_related_element".into(),json!("owner"));
                },
                _=>{},
            }
            let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();let query=DefinitionNameQuery::new(&graph,&index);
            let owned=definition_reference_targets_typed_query(&query,find(&graph,"receiver"),"owned_disjoining").unwrap();
            let owning=definition_reference_targets_typed_query(&query,find(&graph,"constraint"),"owning_type").unwrap();
            assert_eq!(owned.len(),case["owned_count"].as_u64().unwrap() as usize);
            assert_eq!(owning.first().is_some_and(|e|e.id=="receiver"),case["owning_is_receiver"].as_bool().unwrap());
            assert_eq!(!owning.is_empty(),case["owning_present"].as_bool().unwrap());
            assert_eq!(find(&graph,"constraint").properties["type_disjoined"]==json!("receiver"),case["stored_source_is_receiver"].as_bool().unwrap());
        }
    }

    #[test]
    fn definition_type_set_parsed_source_consumption_and_persistence() {
        for (kind,keyword) in [("Unioning","unions"),("Intersecting","intersects"),("Differencing","differences"),("Disjoining","disjoint from")] {
            let source=format!("standard library package Base {{ feature things; }} package P {{ feature a; feature b; feature host {keyword} a, b; }}");
            let document=crate::definition_document::parse_and_link(&source,crate::SourceLanguage::Kerml).unwrap_or_else(|e|panic!("{source}: {e}"));
            let restored:crate::KirDocument=serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();
            for doc in [&document,&restored] {
                let graph=&doc.elements;let role=roles::binding(kind).unwrap();
                let host=graph.iter().find(|e|e.properties.get("declared_name")==Some(&json!("host"))).unwrap();
                let index=graph.iter().map(|e|(e.id.as_str(),e)).collect();
                let query=DefinitionNameQuery::new(graph,&index);
                assert_eq!(definition_reference_targets_typed_query(&query,host,role.owned).unwrap().len(),2);
                for relation in owned(&query,host,role).unwrap() {assert!(assess_contribution(&query,host,relation).unwrap());}
                assert_eq!(definition_feature_default_name_in_view(&query,host).unwrap(),"Base::things");
                let targets=owned(&query,host,role).unwrap().into_iter().map(|r|query_reference_in_view(&query,r,role.target).unwrap()).collect();
                assert_eq!(names(targets),json!(["a","b"]));
                assert!(crate::definition_document::assess_type_sets(doc).unwrap().is_empty());
                assert_eq!(doc.metadata["semantic_validation"],json!("not_assessed"));
            }
            if kind!="Disjoining" {
                for (targets,expected_constraint) in [("a",roles::binding(kind).unwrap().count_constraint.unwrap()),("host, b",roles::binding(kind).unwrap().self_constraint.unwrap())] {
                    let source=format!("standard library package Base {{ feature things; }} package P {{ feature a; feature b; feature host {keyword} {targets}; }}");
                    let doc=crate::definition_document::parse_and_link(&source,crate::SourceLanguage::Kerml).unwrap_or_else(|e|panic!("{source}: {e}"));
                    let issues=crate::definition_document::assess_type_sets(&doc).unwrap();
                    assert_eq!(issues.len(),1,"{source}: {issues:?}");
                    assert_eq!(issues[0].constraint,expected_constraint);
                    assert!(!issues[0].unverified);
                }
            }
        }
    }
}
