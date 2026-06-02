use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SemanticDefaultsSeed {
    pub schema_version: u32,
    #[serde(default)]
    pub reference_usage_semantics: ReferenceUsageSemanticsSeed,
    #[serde(default)]
    pub definition_context: DefinitionContextDefaultSeed,
    #[serde(default)]
    pub usage_context: UsageContextDefaultSeed,
    #[serde(default)]
    pub usage_type_defaults: BTreeMap<String, UsageTypeDefaultSeed>,
    #[serde(default)]
    pub usage_subset_defaults: BTreeMap<String, UsageSubsetDefaultSeed>,
    #[serde(default)]
    pub usage_family_defaults: BTreeMap<String, UsageFamilyDefaultSeed>,
    #[serde(default)]
    pub usage_property_defaults: BTreeMap<String, Vec<UsagePropertyDefaultSeed>>,
    #[serde(default)]
    pub usage_actions: BTreeMap<String, Vec<UsageActionSeed>>,
    #[serde(default)]
    pub usage_specialization_policies: BTreeMap<String, UsageSpecializationPolicySeed>,
    #[serde(default)]
    pub usage_resolution_policies: BTreeMap<String, UsageResolutionPolicySeed>,
    #[serde(default)]
    pub usage_traversal_policies: BTreeMap<String, UsageTraversalPolicySeed>,
    #[serde(default)]
    pub usage_id_policies: BTreeMap<String, UsageIdPolicySeed>,
    #[serde(default)]
    pub definition_companion_policies: BTreeMap<String, DefinitionCompanionPolicySeed>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DefinitionContextDefaultSeed {
    #[serde(default)]
    pub abstract_constructs: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageContextDefaultSeed {
    #[serde(default)]
    pub non_variable_owner_constructs: Vec<String>,
    #[serde(default)]
    pub no_type_context_owner_constructs: Vec<String>,
    #[serde(default)]
    pub non_owned_member_constructs: Vec<String>,
    #[serde(default)]
    pub direction_modifiers: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsagePropertyDefaultSeed {
    pub owner_construct: Option<String>,
    pub kir_kind: Option<String>,
    #[serde(default)]
    pub present_modifiers: Vec<String>,
    #[serde(default)]
    pub absent_modifiers: Vec<String>,
    #[serde(default)]
    pub property_refs: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub property_values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageActionSeed {
    pub action: String,
    #[serde(default)]
    pub requires_metadata_properties: bool,
    #[serde(default)]
    pub requires_previous_state: bool,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageSpecializationPolicySeed {
    pub specialization_refs_policy: Option<String>,
    pub materialized_refs_policy: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageResolutionPolicySeed {
    pub reference_target_policy: Option<String>,
    pub connection_end_specialization_policy: Option<String>,
    pub connection_end_parent_construct: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageTraversalPolicySeed {
    #[serde(default)]
    pub records_previous_state: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageIdPolicySeed {
    #[serde(default)]
    pub append_source_location_if_missing_start_col: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DefinitionCompanionPolicySeed {
    pub generated_companion_construct: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReferenceUsageSemanticsSeed {
    #[serde(default)]
    pub constructs: Vec<String>,
    #[serde(default)]
    pub modifier_rules: Vec<ReferenceModifierSemanticsSeed>,
    #[serde(default)]
    pub typed_data_value: ReferenceTypedSemanticsSeed,
    #[serde(default)]
    pub typed_object: ReferenceTypedSemanticsSeed,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReferenceModifierSemanticsSeed {
    pub modifier: String,
    #[serde(default)]
    pub synthetic_declared_name: bool,
    pub default_type_ref: Option<String>,
    pub semantic_specializations: Option<Vec<String>>,
    #[serde(default)]
    pub subsetted_feature_refs: Vec<String>,
    #[serde(default)]
    pub specialized_feature_refs: Vec<String>,
    #[serde(default)]
    pub redefined_feature_refs: Vec<String>,
    pub direction: Option<String>,
    pub direction_from_modifier: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReferenceTypedSemanticsSeed {
    #[serde(default)]
    pub subsetted_feature_refs: Vec<String>,
    #[serde(default)]
    pub direction_from_modifiers: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageTypeDefaultSeed {
    pub type_ref: Option<String>,
    #[serde(default)]
    pub owner_type_refs: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageSubsetDefaultSeed {
    #[serde(default)]
    pub subsetted_feature_refs: Vec<String>,
    #[serde(default)]
    pub suppress_default_for_modifiers: Vec<String>,
    #[serde(default)]
    pub append_semantic_specializations_when_no_defaults: bool,
    #[serde(default)]
    pub owner_subsetted_feature_refs: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub modifier_owner_subsetted_feature_refs: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    pub specialized_feature_subset: Option<SpecializedFeatureSubsetDefaultSeed>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SpecializedFeatureSubsetDefaultSeed {
    #[serde(default)]
    pub require_feature_ref: bool,
    #[serde(default)]
    pub require_multiplicity: bool,
    #[serde(default)]
    pub include_specialized_features: bool,
    #[serde(default)]
    pub require_no_explicit_type_for_append_refs: bool,
    #[serde(default)]
    pub append_refs: Vec<String>,
    #[serde(default)]
    pub owner_append_refs: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageFamilyDefaultSeed {
    pub type_ref: String,
    #[serde(default)]
    pub subsetted_feature_refs: Vec<String>,
    #[serde(default)]
    pub owner_subsetted_feature_refs: BTreeMap<String, Vec<String>>,
    pub is_variable: bool,
}
