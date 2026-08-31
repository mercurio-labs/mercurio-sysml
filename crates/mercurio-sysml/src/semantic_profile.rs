use std::collections::BTreeMap;
use std::collections::BTreeSet;

use mercurio_foundation::{
    Atom, AttributePolicyAnswer, CORE_RULEPACK_VERSION, CapabilityAnswer, Concept, Fact, Graph,
    KirFieldKind, LanguageId, Rule, RulePack, SemanticCapabilityOracle, SemanticCapabilityProfile,
    SemanticElementAuthoring, SemanticElementForm, TableSemanticCapabilityOracle, Term,
    language::profile::LanguageProfile,
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

include!(concat!(env!("OUT_DIR"), "/sysml_field_specs.rs"));
include!(concat!(env!("OUT_DIR"), "/sysml_writable_vocabulary.rs"));
include!(concat!(
    env!("OUT_DIR"),
    "/sysml_metamodel_generalizations.rs"
));
include!(concat!(env!("OUT_DIR"), "/sysml_grammar_containment.rs"));

const SYSML_EXTENSION_RELATIONSHIP_KEYWORDS: &[(&str, &str)] = &[
    ("trace", "mercurio-extension"),
    ("refine", "mercurio-extension"),
];

pub fn sysml_extension_relationship_keywords() -> &'static [(&'static str, &'static str)] {
    SYSML_EXTENSION_RELATIONSHIP_KEYWORDS
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
                    atom(
                        "edge",
                        [var("Relationship"), constant("source"), var("Source")],
                    ),
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
                    atom(
                        "edge",
                        [var("Relationship"), constant("source"), var("Source")],
                    ),
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
        if sysml_is_requirement_kind(&element.kind)
            && sysml_trace_relationship_role(&element.kind).is_none()
        {
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
    let canonical = sysml_canonical_kind_name(kind);
    sysml_trace_relationship_role(canonical).is_none()
        && (sysml_kind_specializes(canonical, "RequirementDefinition")
            || sysml_kind_specializes(canonical, "RequirementUsage"))
}

pub fn sysml_trace_relationship_role(kind: &str) -> Option<&'static str> {
    match sysml_canonical_kind_name(kind) {
        "SatisfyRequirementUsage" | "SatisfyUsage" => Some("satisfy"),
        "VerificationCaseUsage" | "VerifyRequirementUsage" | "VerifyUsage" => Some("verify"),
        _ => None,
    }
}

fn sysml_canonical_kind_name(kind: &str) -> &str {
    kind.rsplit("::").next().unwrap_or(kind).trim()
}

fn sysml_kind_specializes(kind: &str, target: &str) -> bool {
    let kind = sysml_canonical_kind_name(kind);
    let target = sysml_canonical_kind_name(target);
    if kind.eq_ignore_ascii_case(target) {
        return true;
    }

    let mut stack = vec![kind.to_string()];
    let mut visited = BTreeSet::new();
    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        for (specific, general) in sysml_metamodel_generalizations() {
            if sysml_canonical_kind_name(specific).eq_ignore_ascii_case(&current) {
                let parent = sysml_canonical_kind_name(general);
                if parent.eq_ignore_ascii_case(target) {
                    return true;
                }
                stack.push(parent.to_string());
            }
        }
    }
    false
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
    let mut profile = SemanticCapabilityProfile::default();
    profile.doc_id_attribute_aliases = vec!["id", "requirement_id"];

    for relationship in sysml_relationship_keywords().iter().copied().chain(
        sysml_extension_relationship_keywords()
            .iter()
            .map(|(kind, _)| *kind),
    ) {
        profile = profile.relationship_uses_owner_as_source(relationship);
    }

    for (keyword, kind) in sysml_definition_element_kinds() {
        let alias = format!("{keyword} def");
        profile = profile
            .element_kind_authoring(kind, SemanticElementForm::Definition, keyword)
            .definition_keyword_element_kind(keyword, kind)
            .definition_keyword_alias(&alias, keyword);
    }

    for (keyword, kind) in sysml_usage_element_kinds() {
        profile = profile
            .element_kind_authoring(kind, SemanticElementForm::Usage, keyword)
            .usage_keyword_element_kind(keyword, kind);
    }

    for usage in sysml_usage_keywords() {
        if let Some(definition) = sysml_definition_keyword_for_usage(usage) {
            profile = profile.supporting_definition_keyword(usage, definition);
        }
        let allowed_definitions = sysml_allowed_definition_keywords_for_usage(usage);
        for definition in allowed_definitions {
            profile = profile.allow_usage_typing(usage, &format!("{definition} def"));
        }
    }

    for (usage_keyword, usage_kind) in sysml_usage_element_kinds() {
        for definition_keyword in sysml_allowed_definition_keywords_for_usage(usage_keyword) {
            if let Some(definition_kind) = sysml_definition_element_kind(definition_keyword) {
                profile = profile.allow_usage_typing(usage_kind, definition_kind);
            }
        }
    }

    let semantic_containers = std::iter::once("Package")
        .chain(
            sysml_definition_element_kinds()
                .iter()
                .map(|(_, kind)| *kind),
        )
        .chain(sysml_usage_element_kinds().iter().map(|(_, kind)| *kind))
        .collect::<Vec<_>>();
    let semantic_children = std::iter::once("Package")
        .chain(
            sysml_definition_element_kinds()
                .iter()
                .map(|(_, kind)| *kind),
        )
        .chain(sysml_usage_element_kinds().iter().map(|(_, kind)| *kind))
        .collect::<Vec<_>>();

    let mut keyword_containers = vec!["package".to_string()];
    keyword_containers.extend(
        sysml_definition_keywords()
            .iter()
            .map(|keyword| keyword.to_string()),
    );
    keyword_containers.extend(
        sysml_definition_keywords()
            .iter()
            .map(|keyword| format!("{keyword} def")),
    );

    for container in &keyword_containers {
        for child in sysml_definition_keywords()
            .iter()
            .chain(sysml_usage_keywords().iter())
            .copied()
            .chain(["package"])
        {
            profile = profile.allow_containment(container, child);
        }
        for child in sysml_definition_keywords() {
            let child = format!("{child} def");
            profile = profile.allow_containment(container, &child);
        }
        for child in &semantic_children {
            profile = profile.allow_containment(container, child);
        }
    }

    for container in &semantic_containers {
        profile = profile.allow_containment(container, "package");
        for child in sysml_definition_keywords()
            .iter()
            .chain(sysml_usage_keywords().iter())
            .copied()
        {
            profile = profile.allow_containment(container, child);
        }
        for child in sysml_definition_keywords() {
            let child = format!("{child} def");
            profile = profile.allow_containment(container, &child);
        }
        for child in &semantic_children {
            profile = profile.allow_containment(container, child);
        }
    }

    for kind in sysml_definition_keywords()
        .iter()
        .chain(sysml_usage_keywords().iter())
        .copied()
    {
        profile = profile
            .allow_specialization(kind, kind)
            .allow_specialization(kind, &format!("{kind} def"));
    }

    for (_, kind) in sysml_definition_element_kinds()
        .iter()
        .chain(sysml_usage_element_kinds().iter())
    {
        profile = profile
            .allow_specialization(kind, kind)
            .allow_specialization(kind, "*");
    }
    for (specific, general) in sysml_metamodel_generalizations() {
        profile = profile
            .allow_specialization(specific, general)
            .allow_specialization(
                sysml_canonical_kind_name(specific),
                sysml_canonical_kind_name(general),
            );
    }

    let mut keyword_relationship_sources = Vec::new();
    keyword_relationship_sources.extend(
        sysml_definition_keywords()
            .iter()
            .map(|keyword| keyword.to_string()),
    );
    keyword_relationship_sources.extend(
        sysml_usage_keywords()
            .iter()
            .map(|keyword| keyword.to_string()),
    );
    keyword_relationship_sources.extend(
        sysml_definition_keywords()
            .iter()
            .map(|keyword| format!("{keyword} def")),
    );
    let requirement_like_keyword_targets = sysml_requirement_like_keyword_targets();
    let requirement_like_element_targets = sysml_requirement_like_element_targets();

    for source in &keyword_relationship_sources {
        for relationship in sysml_relationship_keywords() {
            if sysml_relationship_requires_requirement_target(relationship) {
                for target in &requirement_like_keyword_targets {
                    profile = profile.allow_relationship(relationship, source, target);
                }
            } else {
                profile = profile.allow_relationship(relationship, source, "*");
            }
        }
        for (relationship, _) in sysml_extension_relationship_keywords() {
            profile = profile.allow_relationship(relationship, source, "*");
        }
    }

    let semantic_sources = sysml_definition_element_kinds()
        .iter()
        .chain(sysml_usage_element_kinds().iter())
        .map(|(_, kind)| *kind)
        .collect::<Vec<_>>();
    for source in semantic_sources {
        for relationship in sysml_relationship_keywords() {
            if sysml_relationship_requires_requirement_target(relationship) {
                for target in &requirement_like_element_targets {
                    profile = profile.allow_relationship(relationship, source, target);
                }
            } else {
                profile = profile.allow_relationship(relationship, source, "*");
            }
        }
        for (relationship, _) in sysml_extension_relationship_keywords() {
            profile = profile.allow_relationship(relationship, source, "*");
        }
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

fn sysml_relationship_requires_requirement_target(keyword: &str) -> bool {
    matches!(
        keyword.trim().to_ascii_lowercase().as_str(),
        "satisfy" | "verify"
    )
}

fn sysml_requirement_like_keyword_targets() -> Vec<String> {
    let mut targets = BTreeSet::new();
    for (keyword, kind) in sysml_usage_element_kinds() {
        if sysml_is_requirement_kind(kind) {
            targets.insert((*keyword).to_string());
        }
    }
    for (keyword, kind) in sysml_definition_element_kinds() {
        if sysml_is_requirement_kind(kind) {
            targets.insert((*keyword).to_string());
            targets.insert(format!("{keyword} def"));
        }
    }
    targets.into_iter().collect()
}

fn sysml_requirement_like_element_targets() -> Vec<&'static str> {
    let mut targets = BTreeSet::new();
    for (_, kind) in sysml_usage_element_kinds()
        .iter()
        .chain(sysml_definition_element_kinds().iter())
    {
        if sysml_is_requirement_kind(kind) {
            targets.insert(*kind);
        }
    }
    targets.into_iter().collect()
}

fn sysml_definition_element_kind(keyword: &str) -> Option<&'static str> {
    sysml_definition_element_kinds()
        .iter()
        .find_map(|(candidate, kind)| (candidate == &keyword).then_some(*kind))
}

/// The full SysML capability profile is expensive to assemble (combinatorial
/// keyword/kind tables plus generalization-lattice walks), and every oracle
/// query needs it — build it once per process instead of per call.
fn sysml_table_oracle() -> &'static TableSemanticCapabilityOracle {
    static ORACLE: std::sync::OnceLock<TableSemanticCapabilityOracle> = std::sync::OnceLock::new();
    ORACLE.get_or_init(|| TableSemanticCapabilityOracle::new(sysml_semantic_capability_profile()))
}

/// Builds the process-wide cached SysML capability tables eagerly. Hosts that
/// serve interactive requests (console API, desktop) call this at startup so
/// the first legality or next-actions request does not pay the one-time
/// profile construction cost (~1.5s on a debug build).
pub fn warm_sysml_semantic_capability_cache() {
    let _ = sysml_table_oracle();
}

pub fn sysml_language_profile() -> LanguageProfile {
    LanguageProfile {
        id: SYSML_LANGUAGE_PROFILE_ID.to_string(),
        language: LanguageId::from("model"),
        language_version: "2.0".to_string(),
        metamodel_version: "sysml-2.0".to_string(),
        stdlib_version: "sysml-2.0".to_string(),
        stdlib_path: "resources/sysml/sysml-library.kir.json".to_string(),
        kir_schema_version: mercurio_foundation::ir::KIR_SCHEMA_VERSION.to_string(),
        canonical_kinds: BTreeMap::from([
            (Concept::PACKAGE, "KerML::Kernel::Package".to_string()),
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
    let normalized = keyword.trim().to_ascii_lowercase();
    sysml_relationship_keywords()
        .iter()
        .any(|relationship| *relationship == normalized.as_str())
        || sysml_extension_relationship_keywords()
            .iter()
            .any(|(relationship, _)| *relationship == normalized.as_str())
}

pub fn sysml_is_satisfy_relationship(kind: &str) -> bool {
    matches!(
        kind.trim().to_ascii_lowercase().as_str(),
        "satisfy" | "satisfies"
    )
}

pub fn sysml_relationship_usage_keyword(kind: &str) -> Option<&'static str> {
    let normalized = kind.trim().to_ascii_lowercase();
    sysml_relationship_keywords()
        .iter()
        .copied()
        .find(|keyword| normalized == *keyword || normalized.contains(keyword))
}

pub fn sysml_definition_keyword_for_usage(keyword: &str) -> Option<&'static str> {
    let normalized = keyword.trim().to_ascii_lowercase();
    sysml_usage_definition_pairs()
        .iter()
        .find_map(|(usage, definition)| (usage == &normalized).then_some(*definition))
}

pub fn sysml_allowed_definition_keywords_for_usage(keyword: &str) -> Vec<&'static str> {
    let Some(definition_keyword) = sysml_definition_keyword_for_usage(keyword) else {
        return Vec::new();
    };
    let Some(definition_kind) = sysml_definition_element_kind(definition_keyword) else {
        return vec![definition_keyword];
    };
    let mut allowed = BTreeSet::from([definition_keyword]);
    for (candidate_keyword, candidate_kind) in sysml_definition_element_kinds() {
        if sysml_kind_specializes(definition_kind, candidate_kind) {
            allowed.insert(*candidate_keyword);
        }
    }
    allowed.into_iter().collect()
}

pub fn sysml_definition_kind(keyword: &str) -> Option<String> {
    let normalized = normalize_definition_keyword(keyword);
    sysml_definition_element_kind(&normalized).map(|kind| format!("model.{kind}"))
}

pub fn sysml_usage_kind(keyword: &str) -> Option<String> {
    let normalized = keyword.trim().to_ascii_lowercase();
    sysml_usage_element_kinds()
        .iter()
        .find_map(|(candidate, kind)| (candidate == &normalized).then(|| format!("model.{kind}")))
}

pub fn sysml_is_container_kind(kind: &str) -> bool {
    let normalized = kind.trim().to_ascii_lowercase();
    let canonical = normalized
        .strip_prefix("model.")
        .unwrap_or(&normalized)
        .rsplit("::")
        .next()
        .unwrap_or(&normalized);
    canonical == "package"
        || sysml_is_definition_keyword(canonical)
        || sysml_is_usage_keyword(canonical)
        || sysml_definition_element_kinds()
            .iter()
            .any(|(_, element_kind)| element_kind.eq_ignore_ascii_case(canonical))
        || sysml_usage_element_kinds()
            .iter()
            .any(|(_, element_kind)| element_kind.eq_ignore_ascii_case(canonical))
}

pub fn sysml_is_definition_keyword(kind: &str) -> bool {
    let normalized = normalize_definition_keyword(kind);
    sysml_definition_keywords()
        .iter()
        .any(|keyword| *keyword == normalized)
}

pub fn sysml_is_usage_keyword(kind: &str) -> bool {
    let normalized = kind.trim().to_ascii_lowercase();
    sysml_usage_keywords()
        .iter()
        .any(|keyword| *keyword == normalized)
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
    use mercurio_foundation::{Graph, KirDocument, KirElement, materialize_core_indexes};
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
    fn sysml_field_specs_are_generated_with_expected_shape() {
        let fields = sysml_field_specs();

        assert!(fields.len() >= 69);
        assert!(
            fields
                .iter()
                .any(|(field, kind)| *field == "type_label" && *kind == KirFieldKind::Scalar)
        );
        assert_eq!(
            fields.iter().find(|(field, _)| *field == "definition"),
            Some(&("definition", KirFieldKind::ReferenceList))
        );
        assert_eq!(
            fields.iter().find(|(field, _)| *field == "body"),
            Some(&("body", KirFieldKind::Scalar))
        );
        assert_eq!(
            fields.iter().find(|(field, _)| *field == "direction"),
            Some(&("direction", KirFieldKind::Scalar))
        );
        assert_eq!(
            fields.iter().find(|(field, _)| *field == "owned_feature"),
            Some(&("owned_feature", KirFieldKind::ReferenceList))
        );
        assert_eq!(
            fields.iter().find(|(field, _)| *field == "do_behavior"),
            Some(&("do_behavior", KirFieldKind::Metadata))
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
                        ("kir_property".to_string(), json!("verify")),
                        ("feature_kind".to_string(), json!("reference")),
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
    fn sysml_requirement_kind_detection_is_exact_and_registry_backed() {
        assert!(sysml_is_requirement_kind("RequirementUsage"));
        assert!(sysml_is_requirement_kind(
            "SysML::Requirements::RequirementDefinition"
        ));
        assert!(sysml_is_requirement_kind("ConcernUsage"));
        assert!(sysml_is_requirement_kind(
            "SysML::Views::ViewpointDefinition"
        ));
        assert!(!sysml_is_requirement_kind("SatisfyRequirementUsage"));
        assert!(!sysml_is_requirement_kind("NonRequirementThing"));
    }

    #[test]
    fn sysml_oracle_uses_generated_metamodel_specialization_lattice() {
        let oracle = SysmlSemanticCapabilityOracle;

        assert_eq!(
            oracle.can_specialize("ConcernUsage", "RequirementUsage"),
            CapabilityAnswer::Allowed
        );
        assert_eq!(
            oracle.can_specialize("SysML::ConnectionDefinition", "SysML::PartDefinition"),
            CapabilityAnswer::Allowed
        );
    }

    #[test]
    fn sysml_trace_relationship_roles_use_explicit_kind_aliases() {
        assert_eq!(
            sysml_trace_relationship_role("SysML::Requirements::SatisfyRequirementUsage"),
            Some("satisfy")
        );
        assert_eq!(
            sysml_trace_relationship_role("SysML::Requirements::VerifyRequirementUsage"),
            Some("verify")
        );
        assert_eq!(
            sysml_trace_relationship_role("SysML::Verification::VerificationCaseUsage"),
            Some("verify")
        );
        assert_eq!(sysml_trace_relationship_role("RequirementUsage"), None);
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
    fn sysml_oracle_allows_satisfy_to_requirement_like_specializations() {
        let oracle = SysmlSemanticCapabilityOracle;

        assert_eq!(
            oracle.can_relate("satisfy", "PartUsage", "ConcernUsage"),
            CapabilityAnswer::Allowed
        );
        assert_eq!(
            oracle.can_relate("verify", "PartUsage", "ViewpointDefinition"),
            CapabilityAnswer::Allowed
        );
        assert_eq!(
            oracle.can_relate("satisfy", "part", "concern"),
            CapabilityAnswer::Allowed
        );
        assert_eq!(
            oracle.can_relate("verify", "part", "viewpoint def"),
            CapabilityAnswer::Allowed
        );
    }

    #[test]
    fn sysml_oracle_allows_generated_relationship_families() {
        let oracle = SysmlSemanticCapabilityOracle;

        let answer = oracle.can_relate("allocate", "part", "part");

        assert_eq!(answer, CapabilityAnswer::Allowed);
        assert!(oracle.relationship_uses_owner_as_source("allocate"));
        assert!(oracle.relationship_uses_owner_as_source("connect"));
    }

    #[test]
    fn sysml_extension_relationships_are_explicit_compatibility_entries() {
        let oracle = SysmlSemanticCapabilityOracle;

        assert_eq!(
            sysml_extension_relationship_keywords(),
            &[
                ("trace", "mercurio-extension"),
                ("refine", "mercurio-extension"),
            ]
        );
        assert!(!sysml_relationship_keywords().contains(&"trace"));
        assert_eq!(
            oracle.can_relate("trace", "part", "part"),
            CapabilityAnswer::Allowed
        );
        assert!(oracle.relationship_uses_owner_as_source("refine"));
    }

    #[test]
    fn sysml_container_kind_detection_is_exact_and_registry_backed() {
        assert!(sysml_is_container_kind("package"));
        assert!(sysml_is_container_kind("part"));
        assert!(sysml_is_container_kind("part def"));
        assert!(sysml_is_container_kind("model.PartDefinition"));
        assert!(sysml_is_container_kind(
            "SysML::Requirements::RequirementUsage"
        ));
        assert!(!sysml_is_container_kind("NonUsageThing"));
        assert!(!sysml_is_container_kind("undefined"));
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
    fn sysml_oracle_allows_usage_typing_by_definition_ancestors() {
        let oracle = SysmlSemanticCapabilityOracle;

        assert_eq!(
            oracle.can_type_usage("ConcernUsage", "RequirementDefinition"),
            CapabilityAnswer::Allowed
        );
        assert_eq!(
            oracle.can_type_usage("ViewpointUsage", "RequirementDefinition"),
            CapabilityAnswer::Allowed
        );
        assert_eq!(
            oracle.can_type_usage("concern", "requirement def"),
            CapabilityAnswer::Allowed
        );
        assert_eq!(
            oracle.can_type_usage("viewpoint", "requirement def"),
            CapabilityAnswer::Allowed
        );
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
