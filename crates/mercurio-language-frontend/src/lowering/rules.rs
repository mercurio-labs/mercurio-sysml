//! Declarative lowering rule seed schema.
//!
//! These rules are profile data, not yet the runtime execution engine. They
//! give Pilot-derived grammar/Ecore/transform facts a stable shape that audits
//! and future generated lowering code can consume.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use mercurio_language_contracts::SourceLanguage;
use mercurio_language_contracts::diagnostics::Diagnostic;

pub const RUNTIME_ELABORATION_RULE_IDS: &[&str] = &[
    "comment-annotation-target",
    "conjugated-port-definition-name",
    "connection-end-direction",
    "implicit-ref-redefines-target",
    "satisfy-name-as-reference-target",
    "verify-name-as-reference-target",
];

pub fn has_runtime_elaboration_hook(rule_id: &str) -> bool {
    RUNTIME_ELABORATION_RULE_IDS.contains(&rule_id)
}

pub fn has_runtime_collect_expression(expression: &str) -> bool {
    matches!(
        expression,
        "definition"
            | "import"
            | "package"
            | "usage"
            | "true"
            | "$ast.allocation_source"
            | "$ast.allocation_target"
            | "$ast.body_members[usage]"
            | "$ast.docs"
            | "$ast.expression"
            | "$ast.members"
            | "$ast.members[modifier=end]"
            | "$ast.members[usage]"
            | "$ast.modifiers + end"
            | "$ast.modifiers contains abstract"
            | "$ast.multiplicity"
            | "$ast.name"
            | "$ast.path"
            | "$ast.redefines"
            | "$ast.reference_target"
            | "$ast.reference_target or $ast.name"
            | "$ast.specializes"
            | "$ast.specializes or semantic_default"
            | "$ast.subsets"
            | "$ast.ty"
            | "$scope.owner"
            | "$scope.package"
            | "$scope.package_or_definition"
            | "~$ast.name"
    )
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoweringRuleSeed {
    pub schema_version: u32,
    #[serde(default)]
    pub source: BTreeMap<String, Value>,
    #[serde(default)]
    pub rules: Vec<LoweringRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoweringRule {
    pub construct: String,
    pub metaclass: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub ast: LoweringAstPattern,
    pub collect: LoweringCollectRule,
    #[serde(default)]
    pub elaborate: Vec<LoweringElaborationRule>,
    pub emit: LoweringEmitRule,
    #[serde(default)]
    pub pilot_sources: LoweringPilotSources,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoweringAstPattern {
    pub node: String,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoweringCollectRule {
    pub element: String,
    pub name: String,
    pub owner: String,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoweringElaborationRule {
    pub id: String,
    pub when: Option<String>,
    #[serde(default)]
    pub set: BTreeMap<String, String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoweringEmitRule {
    pub id_template: String,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoweringPilotSources {
    #[serde(default)]
    pub grammar_rules: Vec<String>,
    pub ecore_class: Option<String>,
    #[serde(default)]
    pub transform_observations: Vec<String>,
}

impl LoweringRuleSeed {
    pub fn load_for_language(
        language: SourceLanguage,
    ) -> Result<Option<&'static Self>, Diagnostic> {
        match language {
            SourceLanguage::Kerml => Ok(None),
            SourceLanguage::Sysml => Self::load_sysml().map(Some),
        }
    }

    pub fn load_for_profile(profile_id: &str) -> Result<Option<&'static Self>, Diagnostic> {
        match profile_id {
            "sysml-2.0-pilot-0.57.0" => Self::load_sysml().map(Some),
            "kerml-bootstrap" => Ok(None),
            _ => Ok(None),
        }
    }

    fn load_sysml() -> Result<&'static Self, Diagnostic> {
        static SYSML_LOWERING_RULES: OnceLock<Result<LoweringRuleSeed, String>> = OnceLock::new();

        match SYSML_LOWERING_RULES.get_or_init(|| {
            serde_json::from_str(load_sysml_lowering_rules_seed())
                .map_err(|err| format!("failed to parse lowering rule seed: {err}"))
        }) {
            Ok(rules) => Ok(rules),
            Err(message) => Err(Diagnostic::new(message.clone(), None)),
        }
    }
}

fn load_sysml_lowering_rules_seed() -> &'static str {
    include_str!(
        "../../../../resources/language-profiles/sysml-2.0-pilot-0.57.0/mappings/lowering_rules.seed.json"
    )
}
