use std::collections::BTreeMap;
use std::collections::BTreeSet;

use mercurio_core::{
    Atom, AttributePolicyAnswer, CORE_RULEPACK_VERSION, CapabilityAnswer, Concept, Fact, Graph,
    KirFieldKind, LanguageId, Rule, RulePack, SemanticCapabilityOracle, SemanticCapabilityProfile,
    SemanticElementAuthoring, SemanticElementForm, Term,
    TableSemanticCapabilityOracle, language::profile::LanguageProfile,
};

#[derive(Debug, Clone, Default)]
pub struct SysmlSemanticCapabilityOracle;

impl SemanticCapabilityOracle for SysmlSemanticCapabilityOracle {
    fn can_contain(&self, container_kind: &str, child_kind: &str) -> CapabilityAnswer {
        sysml_table_oracle().can_contain(container_kind, child_kind)
    }

    fn can_specialize(&self, source_kind: &str, target_kind: &str) -> CapabilityAnswer {
        sysml_table_oracle().can_specialize(source_kind, target_kind)
    }

    fn can_type_usage(&self, usage_kind: &str, definition_kind: &str) -> CapabilityAnswer {
        match sysml_table_oracle().can_type_usage(usage_kind, definition_kind) {
            CapabilityAnswer::Denied(_) if usage_kind == "part" => CapabilityAnswer::Denied(
                format!("part usages should be typed by part definitions, got `{definition_kind}`"),
            ),
            answer => answer,
        }
    }

    fn can_relate(
        &self,
        relationship_kind: &str,
        source_kind: &str,
        target_kind: &str,
    ) -> CapabilityAnswer {
        match sysml_table_oracle().can_relate(relationship_kind, source_kind, target_kind) {
            CapabilityAnswer::Denied(_) if relationship_kind.eq_ignore_ascii_case("satisfy") => {
                CapabilityAnswer::Denied(
                    "satisfy relationships must target a requirement-like element".to_string(),
                )
            }
            CapabilityAnswer::Denied(_) if relationship_kind.eq_ignore_ascii_case("verify") => {
                CapabilityAnswer::Denied(
                    "verify relationships must target a requirement-like element".to_string(),
                )
            }
            answer => answer,
        }
    }

    fn attribute_policy(&self, kind: &str, attribute: &str) -> AttributePolicyAnswer {
        sysml_table_oracle().attribute_policy(kind, attribute)
    }

    fn relationship_uses_owner_as_source(&self, relationship_kind: &str) -> bool {
        sysml_table_oracle().relationship_uses_owner_as_source(relationship_kind)
    }

    fn doc_id_attribute_aliases(&self) -> &'static [&'static str] {
        &["id", "requirement_id"]
    }

    fn supporting_definition_keyword_for_usage(&self, usage_kind: &str) -> Option<String> {
        sysml_table_oracle().supporting_definition_keyword_for_usage(usage_kind)
    }

    fn normalize_definition_keyword(&self, keyword: &str) -> String {
        sysml_table_oracle().normalize_definition_keyword(keyword)
    }

    fn authoring_for_element_kind(&self, kind: &str) -> Option<SemanticElementAuthoring> {
        sysml_table_oracle().authoring_for_element_kind(kind)
    }

    fn semantic_kind_for_definition_keyword(&self, keyword: &str) -> Option<String> {
        sysml_table_oracle().semantic_kind_for_definition_keyword(keyword)
    }

    fn semantic_kind_for_usage_keyword(&self, keyword: &str) -> Option<String> {
        sysml_table_oracle().semantic_kind_for_usage_keyword(keyword)
    }
}

pub const SYSML_LANGUAGE_PROFILE_ID: &str = "sysml-v2";

pub const SYSML_DEFINITION_KEYWORDS: &[&str] = &[
    "part",
    "attribute",
    "requirement",
    "item",
    "connection",
    "port",
    "action",
    "constraint",
    "calc",
    "state",
    "view",
    "verification",
];

pub const SYSML_USAGE_KEYWORDS: &[&str] = &[
    "part",
    "attribute",
    "requirement",
    "item",
    "connection",
    "port",
    "action",
    "constraint",
    "calc",
    "state",
    "satisfy",
    "verify",
    "ref",
    "reference",
];

pub const SYSML_RELATIONSHIP_KINDS: &[&str] = &["satisfy", "verify", "trace", "refine"];

pub fn sysml_field_specs() -> &'static [(&'static str, KirFieldKind)] {
    &[
        ("type_label", KirFieldKind::Scalar),
        ("pilot_library_group", KirFieldKind::Scalar),
        ("direction", KirFieldKind::Scalar),
        ("multiplicity", KirFieldKind::Scalar),
        ("multiplicity_lower", KirFieldKind::Scalar),
        ("multiplicity_upper", KirFieldKind::Scalar),
        ("declared_multiplicity", KirFieldKind::Scalar),
        ("operator", KirFieldKind::Scalar),
        ("operator_expression", KirFieldKind::Scalar),
        ("trigger", KirFieldKind::Scalar),
        ("trigger_kind", KirFieldKind::Scalar),
        ("is_initial", KirFieldKind::Scalar),
        ("source_is_initial", KirFieldKind::Scalar),
        ("effect", KirFieldKind::Scalar),
        ("text", KirFieldKind::Scalar),
        ("requirement_id", KirFieldKind::Scalar),
        ("body", KirFieldKind::Scalar),
        ("locale", KirFieldKind::Scalar),
        ("language", KirFieldKind::Scalar),
        ("source_file", KirFieldKind::Scalar),
        ("source_language", KirFieldKind::Scalar),
        ("is_abstract", KirFieldKind::Scalar),
        ("is_conjugated", KirFieldKind::Scalar),
        ("is_derived", KirFieldKind::Scalar),
        ("is_end", KirFieldKind::Scalar),
        ("is_variable", KirFieldKind::Scalar),
        ("is_readonly", KirFieldKind::Scalar),
        ("is_ordered", KirFieldKind::Scalar),
        ("is_unique", KirFieldKind::Scalar),
        ("is_library_element", KirFieldKind::Scalar),
        ("is_implied", KirFieldKind::Scalar),
        ("definition", KirFieldKind::Reference),
        ("metatype", KirFieldKind::Reference),
        ("source_feature", KirFieldKind::Reference),
        ("allocated", KirFieldKind::Reference),
        ("allocated_to", KirFieldKind::Reference),
        ("parent_state", KirFieldKind::Reference),
        ("payload", KirFieldKind::Reference),
        ("result", KirFieldKind::Reference),
        ("original_definition", KirFieldKind::Reference),
        ("conjugated", KirFieldKind::Reference),
        ("opposite", KirFieldKind::Reference),
        ("documentedElement", KirFieldKind::Reference),
        ("annotatedElement", KirFieldKind::Reference),
        ("target_ref", KirFieldKind::Reference),
        ("documentation", KirFieldKind::ReferenceList),
        ("feature_typings", KirFieldKind::ReferenceList),
        ("subsets", KirFieldKind::ReferenceList),
        ("subsetted_features", KirFieldKind::ReferenceList),
        ("redefines", KirFieldKind::ReferenceList),
        ("redefined_features", KirFieldKind::ReferenceList),
        ("specialized_features", KirFieldKind::ReferenceList),
        ("featuring_type", KirFieldKind::ReferenceList),
        ("chaining_feature", KirFieldKind::ReferenceList),
        ("imports", KirFieldKind::ReferenceList),
        ("relationships", KirFieldKind::ReferenceList),
        ("sources", KirFieldKind::ReferenceList),
        ("targets", KirFieldKind::ReferenceList),
        ("parts", KirFieldKind::ReferenceList),
        ("items", KirFieldKind::ReferenceList),
        ("owned_feature", KirFieldKind::ReferenceList),
        ("verify", KirFieldKind::ReferenceList),
        ("satisfy", KirFieldKind::ReferenceList),
        ("related", KirFieldKind::ReferenceList),
        ("parameters", KirFieldKind::ReferenceList),
        ("arguments", KirFieldKind::ReferenceList),
        ("successions", KirFieldKind::ReferenceList),
        ("dependencies", KirFieldKind::ReferenceList),
        ("do_behavior", KirFieldKind::Metadata),
    ]
}

pub fn sysml_trace_rulepack() -> RulePack {
    RulePack {
        id: "sysml.trace".to_string(),
        version: CORE_RULEPACK_VERSION.to_string(),
        metadata: BTreeMap::from([(
            "description".to_string(),
            serde_json::Value::String("SysML requirement trace reasoning rules".to_string()),
        )]),
        facts: Vec::new(),
        rules: vec![
            rule(
                "sysml.requirement.kind",
                atom("requirement", [var("Element")]),
                [
                    atom("requirement_kind", [var("Kind")]),
                    atom("kind", [var("Element"), var("Kind")]),
                ],
            ),
            rule(
                "sysml.requirement.specialization",
                atom("requirement", [var("Element")]),
                [
                    atom("subtype", [var("Element"), var("Parent")]),
                    atom("requirement", [var("Parent")]),
                ],
            ),
            rule(
                "sysml.satisfies.direct.satisfy",
                atom("satisfies", [var("Source"), var("Requirement")]),
                [atom(
                    "edge",
                    [var("Source"), constant("satisfy"), var("Requirement")],
                )],
            ),
            rule(
                "sysml.satisfies.direct.satisfies",
                atom("satisfies", [var("Source"), var("Requirement")]),
                [atom(
                    "edge",
                    [var("Source"), constant("satisfies"), var("Requirement")],
                )],
            ),
            rule(
                "sysml.satisfies.relationship",
                atom("satisfies", [var("Source"), var("Requirement")]),
                [
                    atom("relationship_kind", [var("Rel"), constant("satisfy")]),
                    atom("kind", [var("Relationship"), var("Rel")]),
                    atom("edge", [var("Relationship"), constant("source"), var("Source")]),
                    atom(
                        "edge",
                        [var("Relationship"), constant("target"), var("Requirement")],
                    ),
                ],
            ),
            rule(
                "sysml.verifies.direct.verify",
                atom("verifies", [var("Source"), var("Requirement")]),
                [atom(
                    "edge",
                    [var("Source"), constant("verify"), var("Requirement")],
                )],
            ),
            rule(
                "sysml.verifies.direct.verifies",
                atom("verifies", [var("Source"), var("Requirement")]),
                [atom(
                    "edge",
                    [var("Source"), constant("verifies"), var("Requirement")],
                )],
            ),
            rule(
                "sysml.verifies.relationship",
                atom("verifies", [var("Source"), var("Requirement")]),
                [
                    atom("relationship_kind", [var("Rel"), constant("verify")]),
                    atom("kind", [var("Relationship"), var("Rel")]),
                    atom("edge", [var("Relationship"), constant("source"), var("Source")]),
                    atom(
                        "edge",
                        [var("Relationship"), constant("target"), var("Requirement")],
                    ),
                ],
            ),
        ],
        diagnostics: Vec::new(),
    }
}

pub fn sysml_metamodel_adapter_from_graph(graph: &Graph) -> RulePack {
    let mut facts = BTreeSet::new();
    for element in graph.elements() {
        if sysml_is_requirement_kind(&element.kind) && sysml_trace_relationship_role(&element.kind).is_none() {
            facts.insert(Fact::new("requirement_kind", [element.kind.to_string()]));
        }
        if let Some(role) = sysml_trace_relationship_role(&element.kind) {
            facts.insert(Fact::new(
                "relationship_kind",
                [element.kind.to_string(), role.to_string()],
            ));
        }
    }

    RulePack {
        id: "sysml.metamodel.adapter".to_string(),
        version: CORE_RULEPACK_VERSION.to_string(),
        metadata: BTreeMap::from([
            (
                "description".to_string(),
                serde_json::Value::String(
                    "Generated SysML metamodel adapter facts for stable Mercurio predicates"
                        .to_string(),
                ),
            ),
            (
                "elementCount".to_string(),
                serde_json::json!(graph.elements().len()),
            ),
        ]),
        facts: facts.into_iter().collect(),
        rules: Vec::new(),
        diagnostics: Vec::new(),
    }
}

pub fn sysml_is_requirement_kind(kind: &str) -> bool {
    kind.contains("Requirement")
}

pub fn sysml_trace_relationship_role(kind: &str) -> Option<&'static str> {
    let lower = kind.to_ascii_lowercase();
    if lower.contains("satisfy") {
        Some("satisfy")
    } else if lower.contains("verify") {
        Some("verify")
    } else {
        None
    }
}

fn rule<const N: usize>(id: &str, head: Atom, body: [Atom; N]) -> Rule {
    Rule {
        id: id.to_string(),
        head,
        body: body.into_iter().collect(),
    }
}

fn atom<const N: usize>(predicate: &str, terms: [Term; N]) -> Atom {
    Atom {
        predicate: predicate.to_string(),
        terms: terms.into_iter().collect(),
    }
}

fn var(name: &str) -> Term {
    Term::Var(name.to_string())
}

fn constant(value: &str) -> Term {
    Term::Const(value.to_string())
}

pub fn sysml_semantic_capability_profile() -> SemanticCapabilityProfile {
    let mut profile = SemanticCapabilityProfile::default()
        .relationship_uses_owner_as_source("satisfy")
        .relationship_uses_owner_as_source("verify")
        .relationship_uses_owner_as_source("refine")
        .definition_keyword_alias("part def", "part")
        .definition_keyword_alias("requirement def", "requirement")
        .definition_keyword_alias("attribute def", "attribute")
        .definition_keyword_alias("action def", "action")
        .definition_keyword_alias("constraint def", "constraint")
        .definition_keyword_alias("verification def", "verification");
    profile.doc_id_attribute_aliases = vec!["id", "requirement_id"];

    for (keyword, kind) in sysml_definition_element_kinds() {
        profile = profile
            .element_kind_authoring(kind, SemanticElementForm::Definition, keyword)
            .definition_keyword_element_kind(keyword, kind);
    }

    for (keyword, kind) in sysml_usage_element_kinds() {
        profile = profile
            .element_kind_authoring(kind, SemanticElementForm::Usage, keyword)
            .usage_keyword_element_kind(keyword, kind);
    }

    for usage in SYSML_USAGE_KEYWORDS {
        if let Some(definition) = sysml_definition_keyword_for_usage(usage) {
            profile = profile
                .allow_usage_typing(usage, &format!("{definition} def"))
                .supporting_definition_keyword(usage, definition);
        }
    }

    for (usage_keyword, usage_kind) in sysml_usage_element_kinds() {
        if let Some(definition_keyword) = sysml_definition_keyword_for_usage(usage_keyword) {
            if let Some(definition_kind) = sysml_definition_element_kind(definition_keyword) {
                profile = profile.allow_usage_typing(usage_kind, definition_kind);
            }
        }
    }

    let semantic_containers = std::iter::once("Package")
        .chain(
            sysml_definition_element_kinds()
                .into_iter()
                .map(|(_, kind)| kind),
        )
        .chain(
            sysml_usage_element_kinds()
                .into_iter()
                .map(|(_, kind)| kind),
        )
        .collect::<Vec<_>>();
    let semantic_children = std::iter::once("Package")
        .chain(
            sysml_definition_element_kinds()
                .into_iter()
                .map(|(_, kind)| kind),
        )
        .chain(
            sysml_usage_element_kinds()
                .into_iter()
                .map(|(_, kind)| kind),
        )
        .collect::<Vec<_>>();

    for container in ["package"]
        .into_iter()
        .chain(SYSML_DEFINITION_KEYWORDS.iter().map(|kind| *kind))
        .chain(SYSML_DEFINITION_KEYWORDS.iter().map(|kind| match *kind {
            "part" => "part def",
            "attribute" => "attribute def",
            "requirement" => "requirement def",
            "item" => "item def",
            "connection" => "connection def",
            "port" => "port def",
            "action" => "action def",
            "constraint" => "constraint def",
            "calc" => "calc def",
            "state" => "state def",
            "view" => "view def",
            "verification" => "verification def",
            other => other,
        }))
    {
        for child in SYSML_DEFINITION_KEYWORDS
            .iter()
            .chain(SYSML_USAGE_KEYWORDS.iter())
            .copied()
            .chain(["package"])
        {
            profile = profile.allow_containment(container, child);
        }
        for child in SYSML_DEFINITION_KEYWORDS {
            profile = profile.allow_containment(container, &format!("{child} def"));
        }
        for child in &semantic_children {
            profile = profile.allow_containment(container, child);
        }
    }

    for container in &semantic_containers {
        for child in &semantic_children {
            profile = profile.allow_containment(container, child);
        }
    }

    for kind in SYSML_DEFINITION_KEYWORDS
        .iter()
        .chain(SYSML_USAGE_KEYWORDS.iter())
        .copied()
    {
        profile = profile
            .allow_specialization(kind, kind)
            .allow_specialization(kind, &format!("{kind} def"));
    }

    for (_, kind) in sysml_definition_element_kinds()
        .into_iter()
        .chain(sysml_usage_element_kinds())
    {
        profile = profile
            .allow_specialization(kind, kind)
            .allow_specialization(kind, "*");
    }

    for source in SYSML_DEFINITION_KEYWORDS
        .iter()
        .chain(SYSML_USAGE_KEYWORDS.iter())
        .copied()
        .chain(SYSML_DEFINITION_KEYWORDS.iter().map(|kind| match *kind {
            "part" => "part def",
            "attribute" => "attribute def",
            "requirement" => "requirement def",
            "item" => "item def",
            "connection" => "connection def",
            "port" => "port def",
            "action" => "action def",
            "constraint" => "constraint def",
            "calc" => "calc def",
            "state" => "state def",
            "view" => "view def",
            "verification" => "verification def",
            other => other,
        }))
    {
        profile = profile
            .allow_relationship("trace", source, "*")
            .allow_relationship("refine", source, "*")
            .allow_relationship("satisfy", source, "requirement")
            .allow_relationship("satisfy", source, "requirement def")
            .allow_relationship("verify", source, "requirement")
            .allow_relationship("verify", source, "requirement def");
    }

    let semantic_sources = sysml_definition_element_kinds()
        .into_iter()
        .chain(sysml_usage_element_kinds())
        .map(|(_, kind)| kind)
        .collect::<Vec<_>>();
    for source in semantic_sources {
        profile = profile
            .allow_relationship("trace", source, "*")
            .allow_relationship("refine", source, "*")
            .allow_relationship("satisfy", source, "RequirementUsage")
            .allow_relationship("satisfy", source, "RequirementDefinition")
            .allow_relationship("verify", source, "RequirementUsage")
            .allow_relationship("verify", source, "RequirementDefinition");
    }

    for attribute in [
        "declared_name",
        "specializes",
        "type",
        "is_abstract",
        "is_end",
        "direction",
        "target",
        "imports",
        "expression",
        "doc",
        "text",
        "id",
        "requirement_id",
    ] {
        profile = profile.attribute_policy(
            "*",
            attribute,
            AttributePolicyAnswer {
                writable: true,
                reason: None,
            },
        );
    }
    profile
}

fn sysml_definition_element_kinds() -> Vec<(&'static str, &'static str)> {
    vec![
        ("part", "PartDefinition"),
        ("attribute", "AttributeDefinition"),
        ("requirement", "RequirementDefinition"),
        ("item", "ItemDefinition"),
        ("connection", "ConnectionDefinition"),
        ("port", "PortDefinition"),
        ("action", "ActionDefinition"),
        ("constraint", "ConstraintDefinition"),
        ("calc", "CalculationDefinition"),
        ("state", "StateDefinition"),
        ("view", "ViewDefinition"),
        ("verification", "VerificationCaseDefinition"),
    ]
}

fn sysml_usage_element_kinds() -> Vec<(&'static str, &'static str)> {
    vec![
        ("part", "PartUsage"),
        ("attribute", "AttributeUsage"),
        ("requirement", "RequirementUsage"),
        ("item", "ItemUsage"),
        ("connection", "ConnectionUsage"),
        ("port", "PortUsage"),
        ("action", "ActionUsage"),
        ("constraint", "ConstraintUsage"),
        ("calc", "CalculationUsage"),
        ("state", "StateUsage"),
        ("satisfy", "SatisfyRequirementUsage"),
        ("verify", "VerificationCaseUsage"),
        ("ref", "ReferenceUsage"),
        ("reference", "ReferenceUsage"),
    ]
}

fn sysml_definition_element_kind(keyword: &str) -> Option<&'static str> {
    sysml_definition_element_kinds()
        .into_iter()
        .find_map(|(candidate, kind)| (candidate == keyword).then_some(kind))
}

fn sysml_table_oracle() -> TableSemanticCapabilityOracle {
    TableSemanticCapabilityOracle::new(sysml_semantic_capability_profile())
}

pub fn sysml_language_profile() -> LanguageProfile {
    LanguageProfile {
        id: SYSML_LANGUAGE_PROFILE_ID.to_string(),
        language: LanguageId::from("model"),
        language_version: "2.0".to_string(),
        metamodel_version: "sysml-2.0".to_string(),
        stdlib_version: "sysml-2.0".to_string(),
        stdlib_path: "resources/sysml/sysml-library.kir.json".to_string(),
        kir_schema_version: mercurio_core::ir::KIR_SCHEMA_VERSION.to_string(),
        canonical_kinds: BTreeMap::from([
            (
                Concept::PACKAGE,
                "KerML::Kernel::Package".to_string(),
            ),
            (Concept::TYPE, "KerML::Kernel::Type".to_string()),
        ]),
        semantic_anchors: BTreeMap::from([
            (
                "attribute_usage".to_string(),
                "SysML::Systems::AttributeUsage".to_string(),
            ),
            (
                "constraint_usage".to_string(),
                "SysML::Systems::ConstraintUsage".to_string(),
            ),
            (
                "part_definition".to_string(),
                "SysML::Systems::PartDefinition".to_string(),
            ),
            (
                "part_usage".to_string(),
                "SysML::Systems::PartUsage".to_string(),
            ),
            (
                "requirement_usage".to_string(),
                "SysML::Requirements::RequirementUsage".to_string(),
            ),
            (
                "verification_case_usage".to_string(),
                "SysML::Verification::VerificationCaseUsage".to_string(),
            ),
        ]),
        aliases: BTreeMap::from([
            (
                "Model::PartDefinition".to_string(),
                "SysML::Systems::PartDefinition".to_string(),
            ),
            (
                "Model::PartUsage".to_string(),
                "SysML::Systems::PartUsage".to_string(),
            ),
            (
                "Model::RequirementUsage".to_string(),
                "SysML::Requirements::RequirementUsage".to_string(),
            ),
        ]),
    }
}

pub fn sysml_trace_relationship_uses_owner_source(keyword: &str) -> bool {
    matches!(
        keyword.to_ascii_lowercase().as_str(),
        "satisfy" | "verify" | "refine"
    )
}

pub fn sysml_is_satisfy_relationship(kind: &str) -> bool {
    matches!(
        kind.trim().to_ascii_lowercase().as_str(),
        "satisfy" | "satisfies"
    )
}

pub fn sysml_relationship_usage_keyword(kind: &str) -> Option<&'static str> {
    sysml_is_satisfy_relationship(kind).then_some("satisfy")
}

pub fn sysml_definition_keyword_for_usage(keyword: &str) -> Option<&'static str> {
    match keyword.trim().to_ascii_lowercase().as_str() {
        "part" => Some("part"),
        "attribute" => Some("attribute"),
        "requirement" => Some("requirement"),
        "item" => Some("item"),
        "connection" => Some("connection"),
        "port" => Some("port"),
        "action" => Some("action"),
        "constraint" => Some("constraint"),
        "calc" => Some("calc"),
        "state" => Some("state"),
        "view" => Some("view"),
        "verification" => Some("verification"),
        _ => None,
    }
}

pub fn sysml_definition_kind(keyword: &str) -> &'static str {
    match keyword {
        "requirement" => "model.RequirementDefinition",
        "action" => "model.ActionDefinition",
        "metadata" => "model.MetadataDefinition",
        "attribute" => "model.AttributeDefinition",
        _ => "model.PartDefinition",
    }
}

pub fn sysml_usage_kind(keyword: &str) -> &'static str {
    match keyword {
        "requirement" => "model.RequirementUsage",
        "attribute" => "model.AttributeUsage",
        "satisfy" => "model.SatisfyRelationship",
        "action" => "model.ActionUsage",
        _ => "model.PartUsage",
    }
}

pub fn sysml_is_container_kind(kind: &str) -> bool {
    let lower = kind.to_ascii_lowercase();
    lower == "package" || lower.contains("def") || lower.contains("usage") || lower == "part"
}

pub fn sysml_is_definition_keyword(kind: &str) -> bool {
    SYSML_DEFINITION_KEYWORDS.contains(&kind) || kind.ends_with(" def")
}

pub fn sysml_is_usage_keyword(kind: &str) -> bool {
    SYSML_USAGE_KEYWORDS.contains(&kind)
}

pub fn normalize_definition_keyword(keyword: &str) -> String {
    keyword
        .strip_suffix(" def")
        .unwrap_or(keyword)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_core::{Graph, KirDocument, KirElement, materialize_core_indexes};
    use serde_json::json;

    #[test]
    fn sysml_profile_owns_domain_semantic_anchors() {
        let profile = sysml_language_profile();

        assert_eq!(profile.id, SYSML_LANGUAGE_PROFILE_ID);
        assert_eq!(
            profile.semantic_anchors["requirement_usage"],
            "SysML::Requirements::RequirementUsage"
        );
        assert_eq!(
            profile.canonical_kinds[&Concept::PACKAGE],
            "KerML::Kernel::Package"
        );
    }

    #[test]
    fn sysml_trace_rulepack_derives_requirement_indexes() {
        let graph = Graph::from_document(KirDocument {
            metadata: Default::default(),
            elements: vec![
                KirElement {
                    id: "metafeature.SysML.Trace.verify".to_string(),
                    kind: "MetamodelFeature".to_string(),
                    layer: 1,
                    properties: [
                        (
                            "kir_property".to_string(),
                            json!("verify"),
                        ),
                        (
                            "feature_kind".to_string(),
                            json!("reference"),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                },
                KirElement {
                    id: "req.Braking".to_string(),
                    kind: "SysML::Requirements::RequirementUsage".to_string(),
                    layer: 2,
                    properties: Default::default(),
                },
                KirElement {
                    id: "case.BrakeTest".to_string(),
                    kind: "SysML::Verification::VerificationCaseUsage".to_string(),
                    layer: 2,
                    properties: [("verify".to_string(), json!("req.Braking"))]
                        .into_iter()
                        .collect(),
                },
            ],
        })
        .unwrap();
        let indexes = materialize_core_indexes(
            &graph,
            &[
                sysml_trace_rulepack(),
                sysml_metamodel_adapter_from_graph(&graph),
            ],
        )
        .unwrap();

        assert!(indexes.requirements.contains("req.Braking"));
        assert_eq!(
            indexes.verified_by["req.Braking"],
            ["case.BrakeTest".to_string()].into_iter().collect()
        );
    }

    #[test]
    fn sysml_oracle_blocks_satisfy_to_non_requirement() {
        let oracle = SysmlSemanticCapabilityOracle;

        let answer = oracle.can_relate("satisfy", "part", "part");

        assert!(matches!(
            answer,
            CapabilityAnswer::Denied(message) if message.contains("must target a requirement")
        ));
    }

    #[test]
    fn sysml_oracle_blocks_part_usage_typed_by_requirement_definition() {
        let oracle = SysmlSemanticCapabilityOracle;

        let answer = oracle.can_type_usage("part", "requirement def");

        assert!(matches!(
            answer,
            CapabilityAnswer::Denied(message)
                if message.contains("part usages should be typed by part definitions")
        ));
    }

    #[test]
    fn sysml_oracle_allows_requirement_id_and_text_attributes() {
        let oracle = SysmlSemanticCapabilityOracle;

        assert!(oracle.attribute_policy("requirement", "id").writable);
        assert!(oracle.attribute_policy("requirement", "text").writable);
        assert!(
            oracle
                .attribute_policy("requirement", "requirement_id")
                .writable
        );
    }
}
