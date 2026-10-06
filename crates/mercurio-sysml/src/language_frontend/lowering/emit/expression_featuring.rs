//! Shared physical featuring stage over the resolved featuringType delegate.
//! The existing native query supplies the algorithm and typed dependencies;
//! imported Ecore contracts govern stored TypeFeaturing and containment. This
//! reifies FeatureValue, parameter and bound contexts without completing any
//! expression, prototype, receiver, resource, or validation lifecycle.
use super::*;

pub(super) struct FeaturingPlan { pub(super) owner: String, pub(super) expected: Vec<String>, pub(super) missing: Vec<String> }

pub(super) fn materialize_batch(graph:&mut Vec<KirElement>,ids:&[String])->Result<usize,QueryFailure> {
    if ids.is_empty() || ids.windows(2).any(|pair|pair[0]>=pair[1]) {
        return Err(query_failure("expression featuring batch requires sorted distinct nonempty receivers"));
    }
    let plans={
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        if index.len()!=graph.len(){return Err(query_failure("duplicate expression featuring identity"));}
        let query=DefinitionNameQuery::new(graph,&index);let mut reads=TransformationReads::default();let mut plans=Vec::new();
        for id in ids {
            let owner=index.get(id.as_str()).copied().ok_or_else(||query_failure("expression featuring receiver is absent"))?;
            if !metaclass_conforms(&owner.kind,"Expression") || scope_boolean(owner,"is_implied_included")? {
                return Err(query_failure("expression featuring requires an incomplete canonical Expression"));
            }
            if let Some(range)=reads.capture(definition_bound_expression_range_in_view(&query,owner)).flatten() {
                if let Some(false)=reads.capture(multiplicity_featuring::ready(&query,range)) {
                    reads.capture::<()>(Err(QueryFailure::Required(Prerequisite::MultiplicityFeaturing{owner_id:range.id.clone()})));
                }
            }
            let existing=definition_owned_type_featuring_in_view(&query,owner)?.iter()
                .map(|relation|query_reference_in_view(&query,relation,"featuring_type").map(|target|target.id.clone()))
                .collect::<Result<BTreeSet<_>,_>>()?;
            if let Some(selected)=reads.capture(definition_featuring_types_in_view(&query,owner)) {
                for target in &selected {
                    ecore_model::validate_reference_endpoint("TypeFeaturing","featuring_type",&target.kind).map_err(query_failure)?;
                }
                let expected=selected.iter().map(|target|target.id.clone()).collect::<Vec<_>>();
                let missing=expected.iter().filter(|target|!existing.contains(*target)).cloned().collect();
                plans.push(FeaturingPlan{owner:id.clone(),expected,missing});
            }
        }
        reads.finish(||Ok(plans))?.into_query_result()?
    };
    materialize_plans(graph,&plans)
}

// The caller must establish the selected field's semantic inputs. Both expression
// and multiplicity producers use this one checked Ecore construction transaction.
pub(super) fn materialize_plans(graph:&mut Vec<KirElement>,plans:&[FeaturingPlan])->Result<usize,QueryFailure> {
    let count=plans.iter().map(|plan|plan.missing.len()).sum::<usize>();
    if count==0 {return Ok(0);}
    reject_result_snapshots(graph)?;
    if let Some(issue)=ecore_model::validate_publication(graph,ecore_model::ReferenceCompleteness::Closed).first() {
        return Err(query_failure(format!("invalid expression featuring input: {}",issue.message)));
    }
    let mut staged=graph.clone();
    let original_index=graph.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
    for plan in plans {
        for (ordinal,target_id) in plan.missing.iter().enumerate() {
            let id=format!("{}.implicit.expression-featuring.{ordinal}",plan.owner);
            if staged.iter().any(|node|node.id==id) {return Err(query_failure("expression featuring identity collision"));}
            let mut relation=fresh_definition_element(id,"TypeFeaturing")?;
            relation.properties.insert("is_implied".into(),json!(true));
            relation.properties.insert("feature_of_type".into(),json!(plan.owner));
            relation.properties.insert("featuring_type".into(),json!(target_id));
            let detached=staged.iter().find(|node|node.id==*target_id)
                .ok_or_else(||query_failure("expression featuring target is absent"))?
                .properties.get("owning_relationship").is_none_or(Value::is_null);
            if detached {
                let mut ancestor=original_index.get(plan.owner.as_str()).copied();
                while let Some(node)=ancestor {
                    if node.id==*target_id {return Err(query_failure("expression featuring adoption would create an ownership cycle"));}
                    ancestor=scope_container(&original_index,node)?;
                }
                let target=staged.iter_mut().find(|node|node.id==*target_id).ok_or_else(||query_failure("expression featuring target disappeared"))?;
                ecore_ownership::attach(&ecore_ownership_generated::OWNED_ELEMENTS,&mut relation,target)?;
            }
            let owner=staged.iter_mut().find(|node|node.id==plan.owner).ok_or_else(||query_failure("expression featuring owner disappeared"))?;
            ecore_ownership::attach(&ecore_ownership_generated::OWNED_RELATIONSHIPS,owner,&mut relation)?;
            staged.push(relation);
        }
    }
    if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Closed).first() {
        return Err(query_failure(format!("invalid expression featuring output: {}",issue.message)));
    }
    let index=staged.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();let query=DefinitionNameQuery::new(&staged,&index);
    for plan in plans {
        let owner=index.get(plan.owner.as_str()).copied().ok_or_else(||query_failure("expression featuring verification owner is absent"))?;
        let actual=definition_featuring_types_in_view(&query,owner)?.iter().map(|target|target.id.clone()).collect::<Vec<_>>();
        if actual!=plan.expected {return Err(query_failure("physical expression featuring changed its resolved semantic projection"));}
    }
    *graph=staged;Ok(count)
}
