//! Handwritten owned-multiplicity context consumed by result default queries.
//! This preserves bound identities; evaluation, numeric legality, inherited
//! multiplicities and complete expression validation remain separate services.
use super::*;
pub(super) fn is_owned_context(graph:&[KirElement],owner:&KirElement,member:&KirElement)->Result<bool,Diagnostic> {
 if member.kind.rsplit("::").next()!=Some("OwningMembership") {return Ok(false);}
 let contract=ecore_model::feature(&owner.kind,"multiplicity").ok_or_else(||error("missing Type multiplicity contract"))?;
 if !contract.setting_delegate.as_ref().is_some_and(|d|d.uri=="http://www.omg.org/spec/SysML"&&d.status=="custom_setting_delegate_source"&&d.candidates==["org.omg.sysml.delegate.setting.Type_multiplicity_SettingDelegate"]) {return Err(error("unassessed multiplicity delegate"));}
 let _resolved_program=result_multiplicity_generated::PROGRAM_SHA;
 let StoredMembershipEndpoint::Resolved(target)=stored_membership_endpoint(graph,member,"member_element")? else {return Err(error("multiplicity context requires a resolved owned member"));};
 if !metaclass_conforms(&target.kind,result_multiplicity_generated::TARGET) {return Ok(false);}
 if target.kind.rsplit("::").next()!=Some("MultiplicityRange") {return Err(error("unassessed owned multiplicity implementation"));}
 ecore_model::validate_reference_endpoint(&owner.kind,"multiplicity",&target.kind).map_err(error)?;
 let index=graph.iter().map(|e|(e.id.as_str(),e)).collect::<BTreeMap<_,_>>();
 if index.len()!=graph.len()||scope_container(&index,member)?.is_none_or(|p|p.id!=owner.id)||scope_container(&index,target)?.is_none_or(|p|p.id!=member.id) {return Err(error("multiplicity context requires canonical unique ownership"));}
 // Consume the existing Ecore-bound projection rather than cache derived bounds.
 definition_reference_targets(graph,target,"bound")?;
 Ok(true)
}
