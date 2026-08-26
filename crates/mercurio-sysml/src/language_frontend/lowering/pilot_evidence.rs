use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PilotLoweringEvidence {
    pub source: PilotEvidenceSource,
    #[serde(default)]
    pub grammar_rules: Vec<PilotGrammarRuleEvidence>,
    #[serde(default)]
    pub ecore_classes: Vec<PilotEcoreClassEvidence>,
    #[serde(default)]
    pub transform_observations: Vec<PilotTransformObservation>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PilotEvidenceSource {
    pub pilot_source_id: Option<String>,
    pub exporter_version: Option<String>,
    pub captured_at_utc: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PilotGrammarRuleEvidence {
    pub grammar: String,
    pub rule: String,
    pub returns: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub source_file: Option<String>,
    pub source_line: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PilotEcoreClassEvidence {
    pub package: String,
    pub name: String,
    #[serde(default)]
    pub supertypes: Vec<String>,
    #[serde(default)]
    pub structural_features: Vec<PilotEcoreFeatureEvidence>,
    pub abstract_class: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PilotEcoreFeatureEvidence {
    pub name: String,
    pub kind: PilotEcoreFeatureKind,
    pub target: Option<String>,
    pub lower_bound: i32,
    pub upper_bound: i32,
    pub containment: bool,
    pub derived: bool,
    pub transient: bool,
    pub volatile: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotEcoreFeatureKind {
    Attribute,
    Reference,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PilotTransformObservation {
    pub construct: String,
    pub source_metaclass: String,
    pub produced_metaclass: String,
    #[serde(default)]
    pub produced_relationships: Vec<String>,
    pub note: Option<String>,
}
