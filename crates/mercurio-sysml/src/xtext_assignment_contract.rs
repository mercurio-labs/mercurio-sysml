//! Static construction slots resolved from the pinned Xtext and effective Ecore.
//!
//! Traversal of unassigned calls comes from Xtext. Class conformance comes from
//! effective Ecore. Parser control flow and semantic linking remain handwritten.
use std::collections::HashSet;

use crate::language_frontend::lowering::relationship_declarations::metaclass_conforms;

fn local_class(value: &str) -> &str {
    value.rsplit("::").next().map_or(value, |last| last)
}

#[derive(Debug)]
pub(super) struct AssignmentContract {
    pub rule: &'static str,
    pub feature: &'static str,
    pub operator: &'static str,
    pub ecore_owner: &'static str,
    pub ecore_target: &'static str,
    pub kind: &'static str,
    pub containment: bool,
}

#[path = "xtext_assignment_contract_generated.rs"]
mod generated;

/// Check a containment slot reachable through Xtext's unassigned rule calls.
/// The KIR child kind is checked against the resolved Ecore target hierarchy.
pub(crate) fn has_containment_slot(rule: &str, owner_kind: &str, feature: &str, child_kind: &str) -> bool {
    let mut pending = vec![rule];
    let mut seen = HashSet::new();
    while let Some(current) = pending.pop() {
        if !seen.insert(current) {
            continue;
        }
        if generated::ASSIGNMENTS.iter().any(|slot| {
            slot.rule == current
                && slot.feature == feature
                && slot.operator == "+="
                && slot.kind == "reference"
                && slot.containment
                && metaclass_conforms(child_kind, local_class(slot.ecore_target))
                && metaclass_conforms(owner_kind, local_class(slot.ecore_owner))
        }) {
            return true;
        }
        pending.extend(
            generated::UNASSIGNED_CALLS.iter()
                .filter_map(|&(source, target)| (source == current).then_some(target)),
        );
    }
    false
}

#[cfg(test)]
mod tests {
    use super::has_containment_slot;

    #[test]
    fn relationship_body_uses_transitive_xtext_slot_and_ecore_target() {
        let kerml = "org.omg.kerml.xtext.KerML::RelationshipBody";
        let sysml = "org.omg.sysml.xtext.SysML::RelationshipBody";
        assert!(has_containment_slot(kerml, "SysML::Relationship", "ownedRelatedElement", "SysML::Feature"));
        assert!(!has_containment_slot(sysml, "SysML::Relationship", "ownedRelatedElement", "SysML::Feature"));
        assert!(has_containment_slot(sysml, "SysML::Relationship", "ownedRelationship", "SysML::Annotation"));
        assert!(!has_containment_slot(kerml, "SysML::Relationship", "ownedRelatedElement", "UnknownClass"));
        assert!(!has_containment_slot(kerml, "SysML::Class", "ownedRelatedElement", "SysML::Feature"));
        assert!(!has_containment_slot(kerml, "SysML::Relationship", "unknownField", "SysML::Feature"));
        assert!(crate::namespace_grammar::relationship_owns_element(true, "SysML::Feature"));
        assert!(!crate::namespace_grammar::relationship_owns_element(false, "SysML::Feature"));
        assert!(!crate::namespace_grammar::relationship_owns_element(true, "SysML::Comment"));
    }
}
