//! Explicit handwritten reference-binding composition over imported result,
//! connector/default and Ecore contracts. All receivers share one dependency
//! preflight and one transaction. Original expression/resource completion,
//! self-reference fallback and whole-context validation remain separate.
use super::*;
use super::binding_dependencies::{binding_dependencies_plan,verify_completed_binding};

struct BindingPlan { owner: String, source: String, target: String, identity: String, present: bool }

fn plan(query: &DefinitionNameQuery<'_, '_>, owner: &KirElement)
    -> Result<TransformationPlan<BindingPlan>, QueryFailure> {
    if owner.kind.rsplit("::").next()!=Some(specialization_construction_generated::RESULT_PROVIDER)
        || scope_boolean(owner,"is_implied_included")? || scope_boolean(owner,"is_end")? {
        return Err(query_failure("reference binding requires incomplete non-end imported reference-expression dispatch"));
    }
    let mut reads=TransformationReads::default();
    let source=reads.capture(definition_explicit_expression_referent_in_view(query,owner));
    let result=reads.capture(definition_result_parameter_typed_query(query,owner));
    if let Some(false)=reads.capture(reference_result_scope::ready(query,owner)) {
        reads.capture::<()>(Err(QueryFailure::Required(Prerequisite::ReferenceResultSubsetting{owner_id:owner.id.clone()})));
    }
    if let Some(Some(result))=result {
        reads.capture_plan(binding_dependencies_plan(query,result));
        reads.capture(definition_featuring_types_in_view(query,result));
    }
    if let Some(Some(source))=source {
        if !metaclass_conforms(&source.kind,"Feature") {
            return Err(query_failure("reference binding self-reference fallback requires its separate semantic producer"));
        }
        reads.capture(definition_feature_types_in_view(query,source));
        reads.capture(definition_featuring_types_in_view(query,source));
        if let Some(Some(result))=result {
            reads.capture(definition_related_feature_context_in_view(query,&[source,result],true));
        }
    }
    reads.finish(|| {
        let source=source.flatten().ok_or_else(||query_failure("reference binding explicit Feature referent is absent; self-reference fallback is not implemented"))?;
        let target=result.flatten().ok_or_else(||query_failure("reference binding result producer is absent"))?;
        let identity=format!("{}.implicit.reference-binding",owner.id);
        let present=if let Some(binding)=query.index.get(identity.as_str()).copied() {
            verify_completed_binding(query,owner,binding,source,target)?;
            true
        } else {false};
        Ok(BindingPlan{owner:owner.id.clone(),source:source.id.clone(),target:target.id.clone(),identity,present})
    })
}

pub(super) fn materialize_batch(graph:&mut Vec<KirElement>,ids:&[String])->Result<usize,QueryFailure> {
    if ids.is_empty() || ids.windows(2).any(|pair|pair[0]>=pair[1]) {
        return Err(query_failure("reference binding batch requires sorted distinct nonempty receivers"));
    }
    let plans={
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        if index.len()!=graph.len() {return Err(query_failure("duplicate reference binding graph identity"));}
        let query=DefinitionNameQuery::new(graph,&index);let mut reads=TransformationReads::default();let mut plans=Vec::new();
        for id in ids {
            let owner=index.get(id.as_str()).copied().ok_or_else(||query_failure("reference binding owner is absent"))?;
            if let Some(plan)=reads.capture_plan(plan(&query,owner)) {plans.push(plan);}
        }
        reads.finish(||Ok(plans))?.into_query_result()?
    };
    let fresh=plans.iter().filter(|plan|!plan.present).collect::<Vec<_>>();
    if fresh.is_empty(){return Ok(0);}
    // Fresh writes cannot preserve stored derived/volatile snapshots.
    if graph.iter().any(|node|node.properties.keys().any(|field|
        ecore_model::feature(&node.kind,field).is_some_and(|contract|contract.derived||contract.volatile))) {
        return Err(query_failure("reference binding requires derived-value recomputation for stored snapshots"));
    }
    let mut staged=graph.clone();
    let specs=fresh.iter().map(|plan|(plan.owner.as_str(),plan.identity.as_str(),plan.source.as_str(),plan.target.as_str())).collect::<Vec<_>>();
    definition_finish_fresh_binding_batch_in_stage(&mut staged,&specs,ecore_model::ReferenceCompleteness::Closed)?;
    *graph=staged;Ok(fresh.len())
}
