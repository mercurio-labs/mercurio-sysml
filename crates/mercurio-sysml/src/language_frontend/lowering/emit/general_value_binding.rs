//! Shared handwritten checkFeatureValueBindingConnector producer over resolved
//! Ecore valuation/result/ownership contracts and native typed semantic reads.
//! Exact normative sequences, default dispatch and initial global dependencies
//! are explicit. This stage never completes supplied Features or Expressions,
//! supplies a missing global fallback, or substitutes for lifecycle validation.
use super::*;
use super::binding_dependencies::{binding_dependencies_plan,verify_completed_binding};

struct ValueBinding {
    owner: String,
    valuation: String,
    expression: String,
    result: String,
    identity: String,
    source: String,
    initial: Option<(String,Vec<String>)>,
    ordinary_featuring: Vec<String>,
    present: bool,
}

fn exact_chain(query:&DefinitionNameQuery<'_, '_>,chain:&KirElement,expected:&[String])
    ->Result<bool,QueryFailure> {
    canonical_chain_lifecycle::exact(query,chain,expected)
}

/// Structural evidence admits this contribution for selector queries without
/// inferring completion. Every supplied relationship and endpoint stays checked.
pub(super) fn contribution(query:&DefinitionNameQuery<'_, '_>,owner:&KirElement,member:&KirElement)
    ->Result<bool,QueryFailure> {
    if !metaclass_conforms(&owner.kind,"Feature")
        || !matches!(member.kind.rsplit("::").next(),Some("OwningMembership"|"FeatureMembership"))
        || scope_container_query(query,member)?.is_none_or(|parent|parent.id!=owner.id) {return Ok(false);}
    let children=stored_children_projection_query(query,member,"owned_related_element")?;
    if children.len()!=1 {return Ok(false);}
    let binding=children[0];
    if binding.kind.rsplit("::").next()!=Some("BindingConnector") || !scope_boolean(binding,"is_implied")? {return Ok(false);}
    for field in ["is_end","is_variable","is_composite","is_portion"] {
        if scope_boolean(binding,field)? {return Ok(false);}
    }
    if binding.properties.get("direction").is_some_and(|value|!value.is_null()) {return Ok(false);}
    let ends=definition_owned_features_query(query,binding,true)?;
    if ends.len()!=2 {return Ok(false);}
    let mut targets=Vec::new();
    for end in &ends {
        if end.kind.rsplit("::").next()!=Some("Feature") || !scope_boolean(end,"is_end")? {return Ok(false);}
        let references=stored_children_projection_query(query,end,"owned_relationship")?.into_iter()
            .filter(|relation|relation.kind.rsplit("::").next()==Some("ReferenceSubsetting")).collect::<Vec<_>>();
        if references.len()!=1 || query_reference_in_view(query,references[0],"referencing_feature")?.id!=end.id {return Ok(false);}
        targets.push(query_reference_in_view(query,references[0],"referenced_feature")?);
    }
    if targets[1].id!=owner.id {return Ok(false);}
    let source=targets[0];
    let Some(reference)=scope_container_query(query,source)? else{return Ok(false);};
    if reference.kind.rsplit("::").next()!=Some("ReferenceSubsetting")
        || scope_container_query(query,reference)?.is_none_or(|parent|parent.id!=ends[0].id) {return Ok(false);}
    for value in general_value_provider::inputs(query,owner)? {
        if value.default {continue;}
        let Some(result)=definition_result_parameter_typed_query(query,value.expression)? else {continue;};
        if exact_chain(query,source,&[value.expression.id.clone(),result.id.clone()])? {return Ok(true);}
    }
    Ok(false)
}

pub(super) fn is_binding(query:&DefinitionNameQuery<'_, '_>,binding:&KirElement)->Result<bool,QueryFailure> {
    if binding.kind.rsplit("::").next()!=Some("BindingConnector") {return Ok(false);}
    let Some(member)=scope_container_query(query,binding)? else{return Ok(false);};
    let Some(owner)=scope_container_query(query,member)? else{return Ok(false);};
    contribution(query,owner,member)
}

fn initial_targets(query:&DefinitionNameQuery<'_, '_>)->Result<Vec<String>,QueryFailure> {
    let mut targets=Vec::new();
    for name in ["Base::things::that","Occurrences::Occurrence::startShot"] {
        let id=query_local_binding(standard_default_binding_query(query,name)?)?;
        let target=query.index.get(id.as_str()).copied().ok_or_else(||query_failure("initial context global target is absent"))?;
        ecore_model::validate_reference_endpoint("FeatureChaining","chaining_feature",&target.kind).map_err(query_failure)?;
        targets.push(id);
    }
    Ok(targets)
}

/// Both adopted value and initial-context chains are recognized by canonical
/// producer context and exact endpoints, never by their generated identities.
pub(super) fn adopted_chain(query:&DefinitionNameQuery<'_, '_>,chain:&KirElement)->Result<bool,QueryFailure> {
    if chain.kind.rsplit("::").next()!=Some("Feature") {return Ok(false);}
    let Some(relation)=scope_container_query(query,chain)? else {return Ok(false);};
    let binding=match relation.kind.rsplit("::").next() {
        Some("ReferenceSubsetting")=>{
            let Some(end)=scope_container_query(query,relation)? else{return Ok(false);};
            let Some((_,binding))=definition_feature_owner_indexed(query.graph.len(),query.index,end)? else{return Ok(false);};
            binding
        },
        Some("TypeFeaturing")=>{
            let Some(binding)=scope_container_query(query,relation)? else{return Ok(false);};
            if query_reference_in_view(query,relation,"feature_of_type")?.id!=binding.id
                || query_reference_in_view(query,relation,"featuring_type")?.id!=chain.id {return Ok(false);}
            binding
        },
        _=>return Ok(false),
    };
    let Some(member)=scope_container_query(query,binding)? else{return Ok(false);};
    let Some(owner)=scope_container_query(query,member)? else{return Ok(false);};
    if !contribution(query,owner,member)? {return Ok(false);}
    if relation.kind.rsplit("::").next()==Some("TypeFeaturing") {
        if !general_value_provider::inputs(query,owner)?.iter().any(|value|value.initial&&!value.default)
            || !exact_chain(query,chain,&initial_targets(query)?)? {return Ok(false);}
    }
    Ok(true)
}

fn selected(query:&DefinitionNameQuery<'_, '_>,ids:&[String])
    ->Result<TransformationPlan<Vec<ValueBinding>>,QueryFailure> {
    let mut reads=TransformationReads::default();let mut plans=Vec::new();
    for id in ids {
        let owner=query.index.get(id.as_str()).copied().ok_or_else(||query_failure("value-binding receiver is absent"))?;
        for value in general_value_provider::inputs(query,owner)? {
            let identity=format!("{}.implicit.value-binding",value.valuation.id);
            if value.default {
                if query.index.contains_key(identity.as_str()) {
                    return Err(query_failure("default valuation has an incompatible owned value-binding contribution"));
                }
                continue;
            }
            let result_read=reads.capture(definition_result_parameter_typed_query(query,value.expression));
            let result=result_read.flatten();
            let featuring=if value.initial {Some(Vec::new())} else {
                reads.capture(definition_featuring_types_in_view(query,owner))
                    .map(|types|types.iter().map(|target|target.id.clone()).collect::<Vec<_>>())
            };
            reads.capture(definition_feature_types_in_view(query,owner));
            reads.capture(definition_featuring_types_in_view(query,value.expression));
            if let Some(result)=result {reads.capture_plan(binding_dependencies_plan(query,result));}
            let initial=if value.initial {
                reads.capture(initial_targets(query)).map(|targets|(format!("{identity}.initial-context"),targets))
            }else{None};
            if let Some((_,targets))=&initial {
                for id in targets {
                    let target=query.index[id.as_str()];
                    reads.capture(definition_feature_types_in_view(query,target));
                    reads.capture(definition_featuring_types_in_view(query,target));
                    reads.capture(definition_chain_reference_targets_typed_query(query,target,"chaining_feature"));
                }
            }
            if let Some(result)=result {
                let source=format!("{identity}.source");
                let present=query.index.contains_key(identity.as_str());
                if present {
                    let binding=query.index[identity.as_str()];
                    let chain=query.index.get(source.as_str()).copied().ok_or_else(||query_failure("value-binding replay source chain is absent"))?;
                    if !exact_chain(query,chain,&[value.expression.id.clone(),result.id.clone()])? {
                        return Err(query_failure("value-binding replay has incompatible ordered value/result endpoints"));
                    }
                    reads.capture(verify_completed_binding(query,owner,binding,chain,owner));
                    if let Some((chain_id,targets))=&initial {
                        let context=query.index.get(chain_id.as_str()).copied().ok_or_else(||query_failure("initial binding replay context chain is absent"))?;
                        if !exact_chain(query,context,targets)? || !adopted_chain(query,context)? {
                            return Err(query_failure("initial binding replay has incompatible canonical global context"));
                        }
                        if !definition_featuring_types_in_view(query,binding)?.iter().any(|node|node.id==*chain_id) {
                            return Err(query_failure("initial binding replay omits the required global context"));
                        }
                    }else if let Some(expected)=&featuring {
                        let actual=reads.capture(definition_featuring_types_in_view(query,binding))
                            .map(|types|types.iter().map(|target|target.id.clone()).collect::<Vec<_>>());
                        if actual.as_ref().is_some_and(|actual|actual!=expected) {
                            return Err(query_failure("ordinary binding replay has incompatible normative featuring"));
                        }
                    }
                }
                plans.push(ValueBinding{owner:id.clone(),valuation:value.valuation.id.clone(),expression:value.expression.id.clone(),
                    result:result.id.clone(),identity,source,initial,ordinary_featuring:featuring.unwrap_or_default(),present});
            }else if result_read.is_some() {
                reads.capture::<()>(Err(query_failure("value-binding Expression has no native result")));
            }
        }
    }
    reads.finish(||Ok(plans))
}

pub(super) fn materialize_batch(graph:&mut Vec<KirElement>,ids:&[String])->Result<usize,QueryFailure> {
    if ids.is_empty() || ids.windows(2).any(|pair|pair[0]>=pair[1]) {
        return Err(query_failure("general value-binding batch requires sorted distinct nonempty receivers"));
    }
    let plans={
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        if index.len()!=graph.len(){return Err(query_failure("duplicate value-binding graph identity"));}
        selected(&DefinitionNameQuery::new(graph,&index),ids)?.into_query_result()?
    };
    let fresh=plans.iter().filter(|plan|!plan.present).collect::<Vec<_>>();
    if fresh.is_empty(){return Ok(0);}
    reject_result_snapshots(graph)?;
    // Canonical construction previews participate in the same private
    // transaction as their later consumption. Failed/pending reads cannot
    // publish a chain or any other receiver's partial binding.
    let boundary=ecore_model::ReferenceCompleteness::Partial;
    let mut staged=graph.clone();
    for plan in &fresh {
        definition_append_direct_feature_chain_in_stage(&mut staged,&plan.source,
            &[plan.expression.clone(),plan.result.clone()],boundary)?;
        if let Some((id,targets))=&plan.initial {
            definition_append_direct_feature_chain_in_stage(&mut staged,id,targets,boundary)?;
        }
    }
    {
        let index=staged.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        let query=DefinitionNameQuery::new(&staged,&index);let mut reads=TransformationReads::default();
        for plan in &fresh {
            let source=index[plan.source.as_str()];let owner=index[plan.owner.as_str()];
            reads.capture(definition_feature_types_in_view(&query,source));
            reads.capture(definition_featuring_types_in_view(&query,source));
            reads.capture(definition_related_feature_context_in_view(&query,&[source,owner],true));
        }
        reads.finish(||Ok(()))?.into_query_result()?;
    }
    let specs=fresh.iter().map(|plan|(plan.owner.as_str(),plan.identity.as_str(),plan.source.as_str(),plan.owner.as_str())).collect::<Vec<_>>();
    definition_finish_fresh_binding_batch_in_stage(&mut staged,&specs,boundary)?;
    let mut featuring=Vec::new();
    {
        let index=staged.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        let query=DefinitionNameQuery::new(&staged,&index);
        for plan in &fresh {
            let binding=index[plan.identity.as_str()];
            let actual=definition_featuring_types_in_view(&query,binding)?.iter().map(|node|node.id.clone()).collect::<Vec<_>>();
            let expected=if let Some((chain,_))=&plan.initial {
                let mut expected=actual.clone();if !expected.contains(chain){expected.push(chain.clone());}expected
            }else{plan.ordinary_featuring.clone()};
            if actual.iter().any(|id|!expected.contains(id)) {
                return Err(query_failure("ordinary binding context introduces featuring outside the normative owner set"));
            }
            let missing=expected.iter().filter(|id|!actual.contains(id)).cloned().collect();
            featuring.push(expression_featuring::FeaturingPlan{owner:plan.identity.clone(),expected,missing});
        }
    }
    expression_featuring::materialize_plans(&mut staged,&featuring)?;
    {
        let index=staged.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        let query=DefinitionNameQuery::new(&staged,&index);
        for plan in &fresh {
            let owner=index[plan.owner.as_str()];let binding=index[plan.identity.as_str()];
            let member=scope_container_query(&query,binding)?.ok_or_else(||query_failure("value binding placement vanished"))?;
            if !contribution(&query,owner,member)? || !adopted_chain(&query,index[plan.source.as_str()])? {
                return Err(query_failure("value-binding output fails canonical valuation/result proof"));
            }
            let values=general_value_provider::inputs(&query,owner)?;
            if !values.iter().any(|value|value.valuation.id==plan.valuation) {
                return Err(query_failure("value-binding producer changed its valuation selection"));
            }
            verify_completed_binding(&query,owner,binding,index[plan.source.as_str()],owner)?;
            if let Some((context,targets))=&plan.initial {
                if !exact_chain(&query,index[context.as_str()],targets)? || !adopted_chain(&query,index[context.as_str()])? {
                    return Err(query_failure("initial binding output fails canonical global context proof"));
                }
            }
        }
    }
    if let Some(issue)=ecore_model::validate_publication(&staged,boundary).first() {
        return Err(query_failure(format!("invalid general value-binding output: {}",issue.message)));
    }
    *graph=staged;Ok(fresh.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(initial_library:bool)->(crate::KirDocument,Vec<String>) {
        let initial=if initial_library {"standard library package Occurrences { class Occurrence { feature startShot; } }"}
            else{"standard library package Occurrences { class Occurrence; }"};
        let source=format!("standard library package Base {{ abstract classifier Anything; feature things {{ feature that; }} }} \
            standard library package Links {{ assoc Link {{ feature participant; }} feature selfLinks; }} \
            standard library package Performances {{ abstract expr evaluations {{ return libraryResult; }} }} \
            {initial} package P {{ classifier T; class A {{ feature y : T; out feature x : T = y; out feature z : T = y; }} }}");
        let mut doc=crate::definition_document::parse_and_link(&source,crate::SourceLanguage::Kerml).unwrap();
        let mut expressions=doc.elements.iter().filter(|node|node.kind.rsplit("::").next()==Some("FeatureReferenceExpression"))
            .map(|node|node.id.clone()).collect::<Vec<_>>();expressions.sort();assert_eq!(expressions.len(),2);
        for (ordinal,id) in expressions.iter().enumerate() {
            crate::definition_document::materialize_expression_result(&mut doc,id,
                &format!("value.result.{ordinal}.member"),&format!("value.result.{ordinal}")).unwrap();
            reference_result_scope::materialize(&mut doc.elements,id,&format!("value.result.{ordinal}.subsetting")).unwrap();
        }
        let mut receivers=doc.elements.iter().filter(|node|["x","z"].iter().any(|name|
            node.properties.get("declared_name")==Some(&json!(name)))).map(|node|node.id.clone()).collect::<Vec<_>>();
        receivers.sort();assert_eq!(receivers.len(),2);(doc,receivers)
    }
    fn valuation(doc:&crate::KirDocument,id:&str)->String {
        let index=doc.elements.iter().map(|node|(node.id.as_str(),node)).collect();
        let query=DefinitionNameQuery::new(&doc.elements,&index);
        general_value_provider::inputs(&query,index[id]).unwrap()[0].valuation.id.clone()
    }
    fn identity(doc:&crate::KirDocument,id:&str)->String {format!("{}.implicit.value-binding",valuation(doc,id))}
    fn set_flag(doc:&mut crate::KirDocument,id:&str,flag:&str,value:bool) {
        let value_id=valuation(doc,id);
        doc.elements.iter_mut().find(|node|node.id==value_id).unwrap().properties.insert(flag.into(),json!(value));
    }

    #[test]
    fn definition_general_value_binding_dispatches_ordinary_default_initial_and_preserves_original_flags() {
        for (default,initial,expected) in [(false,false,2),(true,false,1),(false,true,2),(true,true,1)] {
            let (mut doc,ids)=fixture(true);set_flag(&mut doc,&ids[0],"is_default",default);set_flag(&mut doc,&ids[0],"is_initial",initial);
            let original=doc.elements.clone();
            let root=Prerequisite::GeneralValueBindingBatch{owner_ids:ids.clone()};
            let restored=serde_json::from_value::<Prerequisite>(serde_json::to_value(&root).unwrap()).unwrap();assert_eq!(root,restored);
            let (graph,pending)=link_generated_graph_planned(doc.elements,Vec::new(),
                &|_|Err(error("no lexical reads expected")),true,&[],&|_|{},Some(&[root])).unwrap();
            assert!(pending.is_empty());doc.elements=graph;
            for node in &original {
                let after=doc.elements.iter().find(|after|after.id==node.id).unwrap();
                assert_eq!(node.properties.get("is_implied_included"),after.properties.get("is_implied_included"));
            }
            assert_eq!(doc.elements.iter().filter(|node|node.kind.rsplit("::").next()==Some("BindingConnector")).count(),expected);
            assert!(ecore_model::validate_publication(&doc.elements,ecore_model::ReferenceCompleteness::Closed).is_empty());
            let index=doc.elements.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&doc.elements,&index);
            for (ordinal,id) in ids.iter().enumerate() {
                if default&&ordinal==0 {assert!(!index.contains_key(identity(&doc,id).as_str()));continue;}
                let binding=index[identity(&doc,id).as_str()];
                let owner=index[id.as_str()];
                let actual=definition_featuring_types_in_view(&query,binding).unwrap();
                if initial&&ordinal==0 {
                    assert!(actual.iter().any(|node|exact_chain(&query,node,&initial_targets(&query).unwrap()).unwrap()));
                }else {
                    assert_eq!(actual.iter().map(|node|node.id.as_str()).collect::<Vec<_>>(),
                        definition_featuring_types_in_view(&query,owner).unwrap().iter().map(|node|node.id.as_str()).collect::<Vec<_>>());
                }
            }
        }
    }

    #[test]
    fn definition_general_value_binding_replays_canonically_after_abstract_syntax_persistence() {
        let (mut doc,ids)=fixture(true);set_flag(&mut doc,&ids[0],"is_initial",true);
        assert_eq!(materialize_batch(&mut doc.elements,&ids).unwrap(),2);
        let exported=crate::abstract_syntax_json::export_sysml_abstract_syntax_value(&doc,Default::default()).unwrap();assert!(!exported.has_errors());
        let imported=crate::abstract_syntax_json::import_sysml_abstract_syntax_value(exported.value,Default::default()).unwrap();assert!(!imported.has_errors());
        let mut restored=imported.persistable_document().unwrap();restored.elements.reverse();
        let snapshot=serde_json::to_value(&restored).unwrap();
        assert_eq!(materialize_batch(&mut restored.elements,&ids).unwrap(),0);
        assert_eq!(snapshot,serde_json::to_value(&restored).unwrap());
    }

    #[test]
    fn definition_general_value_binding_keeps_exact_required_reads_and_all_receiver_atomicity() {
        let (original,ids)=fixture(true);
        let expression_id={
            let index=original.elements.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&original.elements,&index);
            general_value_provider::inputs(&query,index[ids[1].as_str()]).unwrap()[0].expression.id.clone()
        };
        let index=original.elements.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&original.elements,&index);
        let alias=stored_children_projection_query(&query,index[expression_id.as_str()],"owned_relationship").unwrap().into_iter()
            .find(|node|node.kind.rsplit("::").next()==Some("Membership")).unwrap().id.clone();
        let mut changed=original.clone();changed.elements.iter_mut().find(|node|node.id==alias).unwrap().properties.remove("member_element");
        let before=serde_json::to_value(&changed).unwrap();
        assert!(matches!(materialize_batch(&mut changed.elements,&ids),
            Err(QueryFailure::Required(Prerequisite::ReadField{owner_id,field})) if owner_id==alias&&field=="member_element"));
        assert_eq!(before,serde_json::to_value(&changed).unwrap());
        let failure=link_generated_graph_planned(changed.elements.clone(),Vec::new(),&|_|Err(error("unregistered source must not be invented")),
            true,&[],&|_|{},Some(&[Prerequisite::GeneralValueBindingBatch{owner_ids:ids.clone()}])).err().unwrap();
        assert!(failure.message.contains("not pending"));
        let mut changed=original.clone();
        changed.elements.push(fresh_definition_element(format!("{}.end.1.featuring",identity(&changed,&ids[1])),"Feature").unwrap());
        let before=serde_json::to_value(&changed).unwrap();
        let failure=materialize_batch(&mut changed.elements,&ids).unwrap_err();
        assert!(matches!(failure,QueryFailure::Rejected(ref diagnostic)
            if diagnostic.message.contains("collision")||diagnostic.message.contains("duplicate")),"{failure:?}");
        assert_eq!(before,serde_json::to_value(&changed).unwrap());
    }

    #[test]
    fn definition_general_value_binding_rejects_forged_replay_and_missing_initial_resources() {
        let (mut doc,ids)=fixture(true);set_flag(&mut doc,&ids[0],"is_initial",true);
        assert_eq!(materialize_batch(&mut doc.elements,&ids).unwrap(),2);
        for mutation in ["reordered_value_chain","reordered_initial_chain","endpoint","fixed_role","default_dispatch","missing_default"] {
            let mut changed=doc.clone();let binding=identity(&changed,&ids[0]);
            match mutation {
                "reordered_value_chain"|"reordered_initial_chain"=>{
                    let chain=if mutation=="reordered_value_chain" {format!("{binding}.source")}else{format!("{binding}.initial-context")};
                    changed.elements.iter_mut().find(|node|node.id==chain).unwrap().properties.get_mut("owned_relationship").unwrap().as_array_mut().unwrap().reverse();
                },
                "endpoint"=>{changed.elements.iter_mut().find(|node|node.id==format!("{binding}.end.1.reference")).unwrap().properties.insert("referenced_feature".into(),json!(format!("{binding}.end.0")));},
                "fixed_role"=>{changed.elements.iter_mut().find(|node|node.id==binding).unwrap().properties.insert("is_variable".into(),json!(true));},
                "default_dispatch"=>set_flag(&mut changed,&ids[0],"is_default",true),
                _=>{
                    let node=changed.elements.iter().find(|node|node.id==binding).unwrap();
                    let relation=node.properties["owned_relationship"].as_array().unwrap().iter().map(|value|value.as_str().unwrap())
                        .find(|id|changed.elements.iter().any(|node|node.id==*id&&node.kind.rsplit("::").next()==Some("Subsetting"))).unwrap().to_owned();
                    changed.elements.retain(|node|node.id!=relation);
                    changed.elements.iter_mut().find(|node|node.id==binding).unwrap().properties.get_mut("owned_relationship").unwrap().as_array_mut().unwrap().retain(|value|value.as_str()!=Some(relation.as_str()));
                }
            }
            let before=serde_json::to_value(&changed).unwrap();
            assert!(materialize_batch(&mut changed.elements,&ids).is_err(),"{mutation}");
            assert_eq!(before,serde_json::to_value(&changed).unwrap());
        }
        let (mut missing,ids)=fixture(false);set_flag(&mut missing,&ids[0],"is_initial",true);
        let before=serde_json::to_value(&missing).unwrap();
        assert!(materialize_batch(&mut missing.elements,&ids).is_err());
        assert_eq!(before,serde_json::to_value(&missing).unwrap());
    }

    #[test]
    fn definition_general_value_binding_preserves_each_valuation_for_separate_validation() {
        let (mut doc,ids)=fixture(true);let owner=&ids[0];
        let referent=doc.elements.iter().find(|node|node.properties.get("declared_name")==Some(&json!("y"))).unwrap().id.clone();
        let mut value=fresh_definition_element("second.value".into(),"FeatureValue").unwrap();
        let mut expression=fresh_definition_element("second.expression".into(),"FeatureReferenceExpression").unwrap();
        let mut alias=fresh_definition_element("second.alias".into(),"Membership").unwrap();
        alias.properties.insert("member_element".into(),json!(referent));
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,&mut expression,&mut alias).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut value,&mut expression).unwrap();
        ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,doc.elements.iter_mut().find(|node|node.id==*owner).unwrap(),&mut value).unwrap();
        doc.elements.extend([value,expression,alias]);
        crate::definition_document::materialize_expression_result(&mut doc,"second.expression","second.result.member","second.result").unwrap();
        reference_result_scope::materialize(&mut doc.elements,"second.expression","second.result.subsetting").unwrap();
        assert_eq!(materialize_batch(&mut doc.elements,&ids).unwrap(),3);
        let index=doc.elements.iter().map(|node|(node.id.as_str(),node)).collect();let query=DefinitionNameQuery::new(&doc.elements,&index);
        assert_eq!(general_value_provider::inputs(&query,index[owner.as_str()]).unwrap().len(),2);
        assert_eq!(materialize_batch(&mut doc.elements,&ids).unwrap(),0);
    }
}
