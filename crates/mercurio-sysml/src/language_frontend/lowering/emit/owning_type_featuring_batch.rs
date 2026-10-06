//! Handwritten fixed owning-Type lifecycle stage over the imported Feature
//! ownership, variability and TypeFeaturing contracts. Reuses the existing
//! owning-type producer; does not complete Features or semantic validation.
use super::*;

pub(super) fn materialize_batch(graph:&mut Vec<KirElement>,ids:&[String])->Result<usize,QueryFailure> {
    if ids.is_empty() || ids.windows(2).any(|pair|pair[0]>=pair[1]) {
        return Err(query_failure("owning-Type featuring batch requires sorted distinct nonempty receivers"));
    }
    let projections={
        let index=graph.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();
        if index.len()!=graph.len(){return Err(query_failure("duplicate owning-Type featuring identity"));}
        let query=DefinitionNameQuery::new(graph,&index);let mut reads=TransformationReads::default();let mut projections=Vec::new();
        for id in ids {
            let feature=index.get(id.as_str()).copied().ok_or_else(||query_failure("owning-Type featuring receiver is absent"))?;
            if let Some(input)=reads.capture(definition_owning_featuring_input_in_view(&query,feature)) {
                let Some((owner,variable))=input else {return Err(query_failure("owning-Type featuring requires canonical Type ownership"));};
                if variable {return Err(query_failure("fixed owning-Type batch excludes variable snapshot semantics"));}
                if owner.properties.get("owning_relationship").is_none_or(Value::is_null) {
                    return Err(query_failure("owning Type must be contained before featuring materialization"));
                }
                reads.capture(definition_owned_type_featuring_in_view(&query,feature));
                if let Some(selected)=reads.capture(definition_featuring_types_in_view(&query,feature)) {
                    projections.push((id.clone(),selected.iter().map(|node|node.id.clone()).collect::<Vec<_>>()));
                }
            }
        }
        reads.finish(||Ok(projections))?.into_query_result()?
    };
    reject_result_snapshots(graph)?;
    if let Some(issue)=ecore_model::validate_publication(graph,ecore_model::ReferenceCompleteness::Partial).first() {
        return Err(query_failure(format!("invalid owning-Type featuring input: {}",issue.message)));
    }
    let mut staged=graph.clone();let mut changed=0;
    for id in ids {
        changed+=usize::from(definition_materialize_owning_type_featuring_in_stage(&mut staged,id,&format!("{id}.implicit.owning-type-featuring"))?);
    }
    if let Some(issue)=ecore_model::validate_publication(&staged,ecore_model::ReferenceCompleteness::Partial).first() {
        return Err(query_failure(format!("invalid owning-Type featuring output: {}",issue.message)));
    }
    let index=staged.iter().map(|node|(node.id.as_str(),node)).collect::<BTreeMap<_,_>>();let query=DefinitionNameQuery::new(&staged,&index);
    for (id,expected) in projections {
        let feature=index.get(id.as_str()).copied().ok_or_else(||query_failure("owning-Type featuring verification receiver is absent"))?;
        let actual=definition_featuring_types_in_view(&query,feature)?.iter().map(|node|node.id.clone()).collect::<Vec<_>>();
        if actual!=expected {return Err(query_failure("physical owning-Type featuring changed the resolved projection"));}
    }
    *graph=staged;Ok(changed)
}
