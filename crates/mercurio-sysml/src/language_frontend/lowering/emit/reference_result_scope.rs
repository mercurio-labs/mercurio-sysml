//! Handwritten scheduling around the imported reference-result Subsetting producer.
//! Physical inheritance of the referent is distinct from result construction.
use super::*;
pub(super) fn ready(query:&DefinitionNameQuery<'_, '_>,expression:&KirElement)->Result<bool,QueryFailure> {
 if expression.kind.rsplit("::").next()!=Some(specialization_construction_generated::RESULT_PROVIDER) {return Ok(true);}
 let mut referent=None;
 for member in stored_children_projection_query(query,expression,"owned_relationship")? {
  if !metaclass_conforms(&member.kind,"Membership")||metaclass_conforms(&member.kind,"ParameterMembership") {continue;}
  referent=Some(if metaclass_conforms(&member.kind,"OwningMembership") {
   match stored_membership_endpoint_indexed(query.graph.len(),query.index,member,"member_element")? {StoredMembershipEndpoint::Resolved(e)=>e,_=>return Err(query_failure("reference scope requires a canonical owned referent"))}
  } else {query_reference_in_view(query,member,"member_element")?});break;
 }
 let Some(referent)=referent.filter(|e|metaclass_conforms(&e.kind,"Feature")) else {return Ok(true);};
 let result=definition_result_parameter_typed_query(query,expression)?.ok_or_else(||query_failure("reference scope result must be constructed first"))?;
 if result.id==referent.id {return Ok(true);}
 for relationship in stored_children_projection_query(query,result,"owned_relationship")? {
  if relationship.kind.rsplit("::").next()==Some(specialization_construction_generated::RESULT_KIND) {
   if query_reference_in_view(query,relationship,"general")?.id==referent.id {
    if query_reference_in_view(query,relationship,"specific")?.id!=result.id {return Err(query_failure("reference-result specialization disagrees with ownership"));}
    return Ok(true);
   }
  }
 }
 Ok(false)
}
pub(super) fn materialize(graph:&mut Vec<KirElement>,expression_id:&str,prefix:&str)->Result<bool,QueryFailure> {
 let index=graph.iter().map(|e|(e.id.as_str(),e)).collect::<BTreeMap<_,_>>();
 if index.len()!=graph.len() {return Err(query_failure("duplicate reference scope identity"));}
 let expression=index.get(expression_id).copied().ok_or_else(||query_failure("reference scope expression is absent"))?;
 // Construction is a dependency of this physical producer, not an empty
 // result projection. Let the typed scheduler establish the actual return first.
 if expression.kind.rsplit("::").next()==Some(specialization_construction_generated::RESULT_PROVIDER)
  && !definition_additional_members_ready(graph,expression)? {
  return Err(QueryFailure::Required(Prerequisite::AdditionalMembers{owner_id:expression.id.clone()}));
 }
 if ready(&DefinitionNameQuery::new(graph,&index),expression)? {return Ok(false);}
 let mut staged=graph.clone();
 definition_materialize_reference_result_subsetting(&mut staged,expression_id,prefix)?;
 let index=staged.iter().map(|e|(e.id.as_str(),e)).collect::<BTreeMap<_,_>>();
 if !ready(&DefinitionNameQuery::new(&staged,&index),index[expression_id])? {return Err(query_failure("reference scope producer did not establish inheritance"));}
 *graph=staged;Ok(true)
}
