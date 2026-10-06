//! Native adapters for contracts generated from the pinned Xtext model.
//!
//! This translates the existing authoring AST vocabulary to grammar metaclasses.
//! Name resolution and derived properties remain in semantic lowering.
mod generated {
    include!("namespace_grammar_generated.rs");
}

use mercurio_foundation::language_contracts::ast::Declaration;
use generated::*;

pub(crate) fn expose_visibility() -> &'static str {
    EXPOSE_VISIBILITY
}

pub(crate) fn query_kind(kerml: bool, expose: bool, namespace: bool) -> Option<&'static str> {
    match (kerml, expose, namespace) {
        (true, true, _) => None,
        (true, false, false) => Some(KERML_MEMBERSHIPIMPORT),
        (true, false, true) => Some(KERML_NAMESPACEIMPORT),
        (false, false, false) => Some(SYSML_MEMBERSHIPIMPORT),
        (false, false, true) => Some(SYSML_NAMESPACEIMPORT),
        (false, true, false) => Some(SYSML_MEMBERSHIPEXPOSE),
        (false, true, true) => Some(SYSML_NAMESPACEEXPOSE),
    }
}

pub(crate) fn permits_relationship_member(kerml: bool, declaration: &Declaration) -> bool {
    let (fields, annotating) = if kerml {
        (KERML_RELATIONSHIP_BODY_FIELDS, KERML_ANNOTATING_KINDS)
    } else {
        (SYSML_RELATIONSHIP_BODY_FIELDS, SYSML_ANNOTATING_KINDS)
    };
    // Alias/import declarations are namespace memberships, not productions of
    // either language's OwnedRelatedElement or AnnotatingElement rule.
    if matches!(declaration, Declaration::Alias(_) | Declaration::Import(_)) {
        return false;
    }
    let modifiers = match declaration {
        Declaration::Package(value) => &value.modifiers,
        Declaration::GenericDefinition(value) => &value.modifiers,
        Declaration::GenericUsage(value) => &value.modifiers,
        Declaration::Alias(_) | Declaration::Import(_) => unreachable!(),
    };
    // Direct relationship children have no namespace MemberPrefix production.
    if modifiers.iter().any(|m| matches!(m.as_str(), "private" | "protected" | "public")) {
        return false;
    }
    let kind = declaration.as_usage_like().and_then(|usage| match usage.keyword.as_str() {
        "comment" => Some("SysML::Comment"),
        "doc" => Some("SysML::Documentation"),
        "rep" => Some("SysML::TextualRepresentation"),
        "metadata" if kerml => Some("SysML::MetadataFeature"),
        "metadata" => Some("SysML::MetadataUsage"),
        _ => None,
    });
    if let Some(kind) = kind {
        return fields.contains(&"ownedRelationship") && annotating.contains(&kind);
    }
    fields.contains(&"ownedRelatedElement")
        && kerml_owned_element_rule(declaration)
            .is_some_and(|rule| KERML_OWNED_ELEMENT_RULES.contains(&rule))
}

// Handwritten AST-to-rule adapter. The permitted rule set itself comes from
// the pinned Xtext alternatives; unknown parser forms fail closed here.
fn kerml_owned_element_rule(declaration: &Declaration) -> Option<&'static str> {
    match declaration {
        Declaration::Package(package) => Some(if package.modifiers.iter().any(|m| m == "library") {
            "LibraryPackage"
        } else { "Package" }),
        Declaration::GenericDefinition(definition) => Some(match definition.keyword.as_str() {
            "type" => "Type", "classifier" => "Classifier", "class" => "Class",
            "struct" => "Structure", "metaclass" => "Metaclass", "datatype" => "DataType",
            "association" | "assoc" => "Association", "assoc-struct" => "AssociationStructure",
            "interaction" => "Interaction", "behavior" => "Behavior", "function" => "Function",
            "predicate" => "Predicate", "feature" => "Feature", "namespace" => "Namespace",
            _ => return None,
        }),
        Declaration::GenericUsage(usage) => Some(match usage.keyword.as_str() {
            "feature" => "Feature", "step" => "Step", "expr" => "Expression", "multiplicity" | "multiplicity-range" => "Multiplicity",
            "bool" => "BooleanExpression", "inv" => "Invariant",
            "connector" => "Connector", "binding" => "BindingConnector",
            "succession" => "Succession", "flow" | "kerml-flow" => "Flow", "succession-flow" | "kerml-succession-flow" => "SuccessionFlow",
            "dependency" => "Dependency", "specialization" => "Specialization",
            "conjugation" | "conjugate" => "Conjugation", "typing" => "FeatureTyping",
            "subtype" | "subclassifier" => "Subclassification", "disjoining" => "Disjoining",
            "inverting" | "inverse" => "FeatureInverting", "subset" => "Subsetting",
            "redefinition" => "Redefinition", "featuring" => "TypeFeaturing",
            _ => return None,
        }),
        Declaration::Alias(_) | Declaration::Import(_) => None,
    }
}

/// Select the direct containment arm of the pinned RelationshipBody production.
/// Annotating elements use OwnedAnnotation instead of direct element ownership.
pub(crate) fn relationship_owns_element(kerml: bool, kind: &str) -> bool {
    let (rule, annotating) = if kerml {
        ("org.omg.kerml.xtext.KerML::RelationshipBody", KERML_ANNOTATING_KINDS)
    } else {
        ("org.omg.sysml.xtext.SysML::RelationshipBody", SYSML_ANNOTATING_KINDS)
    };
    crate::xtext_assignment_contract::has_containment_slot(rule, "SysML::Relationship", "ownedRelatedElement", kind)
        && !annotating.iter().any(|candidate|
            candidate.rsplit("::").next() == kind.rsplit("::").next())
}
