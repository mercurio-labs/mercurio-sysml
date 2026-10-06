//! Named handwritten NamespaceUtil/ExpressionUtil/FeatureUtil dependencies.
//! Resolved source programs supply selectors; imported Ecore supplies canonical
//! ownership and delegate contracts. No stored derived snapshots are consumed.
//! Conjugated callable inputs and constructor arguments remain explicit gaps.
use super::*;

fn delegate(owner:&KirElement,field:&str,candidate:&str)->Result<(),QueryFailure> {
 let c=ecore_model::feature(&owner.kind,field).ok_or_else(||query_failure("missing relative scope dependency contract"))?;
 if !c.derived || !c.setting_delegate.as_ref().is_some_and(|d|d.uri=="http://www.omg.org/spec/SysML"&&d.status=="custom_setting_delegate_source"&&d.candidates==[candidate]) {
  return Err(query_failure("unsupported relative scope delegate binding"));
 }
 Ok(())
}
fn canonical(query:&DefinitionNameQuery<'_, '_>,owner:&KirElement)->Result<(),QueryFailure> {
 if query.index.len()!=query.graph.len()||!query.index.get(owner.id.as_str()).is_some_and(|e|std::ptr::eq(*e,owner)) {return Err(query_failure("relative namespace requires a unique canonical graph owner"));}
 Ok(())
}
pub(super) fn value_expression<'g>(query:&DefinitionNameQuery<'g,'_>,feature:&'g KirElement)->Result<Option<&'g KirElement>,QueryFailure> {
 for r in stored_children_projection_query(query,feature,"owned_relationship")? {
  if !metaclass_conforms(&r.kind,"FeatureValue") {continue;}
  let children=stored_children_projection_query(query,r,"owned_related_element")?;
  if children.len()>1 {return Err(query_failure("valuation requires at most one canonical Expression"));}
  if let Some(value)=children.first() {
   ecore_model::validate_reference_endpoint(&r.kind,"value",&value.kind).map_err(error)?;
   return Ok(Some(*value));
  }
  return Ok(None);
 }
 Ok(None)
}
/// ExpressionUtil.addArguments is ordered by callable inputs, then owned
/// argument Features. Feature.redefines uses computed direct redefinitions,
/// not specialization or a transitive closure.
pub(super) fn arguments<'g>(query:&DefinitionNameQuery<'g,'_>,owner:&'g KirElement)->Result<Vec<&'g KirElement>,QueryFailure> {
 canonical(query,owner)?;
 if feature_chain_members::supports(&owner.kind)&&!feature_chain_members::ready_query(query,owner)? {return Err(QueryFailure::Required(Prerequisite::AdditionalMembers{owner_id:owner.id.clone()}));}
 delegate(owner,"argument","org.omg.sysml.delegate.setting.InstantiationExpression_argument_SettingDelegate")?;
 if !metaclass_conforms(&owner.kind,"InvocationExpression") {return Err(query_failure("constructor arguments require their result Feature strategy"));}
 let Some(callable)=definition_instantiated_type_query(query,owner)? else {return Ok(Vec::new());};
 let inputs=definition_effective_features_typed_query(query,callable)?;
 let features=definition_owned_features_query(query,owner,false)?;
 let mut result=Vec::new();
 for input in inputs {
  // effective_features already rejects conjugated ancestry. In this bounded
  // region Type.directionOf equals the canonical declaring Feature direction.
  let direction=input.properties.get("direction").filter(|v|!v.is_null());
  if let Some(value)=direction {ecore_model::validate_value(&input.kind,"direction",value).map_err(error)?;}
  if !relative_namespace_generated::input_direction(direction.and_then(Value::as_str)) {continue;}
  for feature in &features {
   if definition_redefined_features_for_arguments_query(query,feature,owner)?.iter().any(|f|f.id==input.id) {
    if let Some(value)=value_expression(query,feature)? {result.push(value);}
   }
  }
 }
 // Preserve the exact ordered collect in the pinned written invocation
 // derivation and resolved Java implementation, including repeated values.
 // Ecore's unique=true conflicts with this result for a parameter redefining
 // multiple inputs. That conflict is an explicit unqualified obligation;
 // neither cached Pilot expectations nor global acceptance is normalized.
 Ok(result)
}
pub(super) fn namespace<'g>(query:&DefinitionNameQuery<'g,'_>,owner:&'g KirElement)->Result<Option<&'g KirElement>,QueryFailure> {
 canonical(query,owner)?;
 let _definition=relative_namespace_generated::DEFINITION_SHA;
 let Some(assignment)=relative_namespace_generated::dispatch(metaclass_conforms(&owner.kind,"AssignmentActionUsage"),metaclass_conforms(&owner.kind,"FeatureChainExpression")) else {return Ok(None);};
 let first=if assignment {
  delegate(owner,"target_argument","org.omg.sysml.delegate.setting.AssignmentActionUsage_targetArgument_SettingDelegate")?;
  // The setting delegate selects the first owned parameter, including OUT;
  // it does not select the first input nor skip an absent valuation.
  let parameters=definition_parameter_collection_query(query,owner,true)?;
  match parameters.first() {Some(p)=>value_expression(query,p)?,None=>None}
 } else {arguments(query,owner)?.first().copied()};
 let Some(expression)=first else {return Ok(None);};
 if !definition_additional_members_ready(query.graph,expression)? {
  return Err(QueryFailure::Required(Prerequisite::AdditionalMembers{owner_id:expression.id.clone()}));
 }
 if !reference_result_scope::ready(query,expression)? {return Err(QueryFailure::Required(Prerequisite::ReferenceResultSubsetting{owner_id:expression.id.clone()}));}
 definition_result_parameter_typed_query(query,expression)
}
