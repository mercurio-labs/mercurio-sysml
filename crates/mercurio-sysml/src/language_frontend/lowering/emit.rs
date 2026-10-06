pub use operand_construction::{DefinitionConstructionEvent, DefinitionConstructionPhase, DefinitionPendingReference};
mod textual_declarations;
mod annotation_checks;
mod memberships;
mod relationship_bodies;
mod multiplicity_ranges;
mod operand_construction;
mod type_relationship_scope_generated;
pub use operand_construction::{DefinitionQueryPrerequisite, DefinitionReferenceQueryError};
pub(crate) use operand_construction::{definition_direction_of_with_dependencies,definition_reference_target_batch_with_dependencies,definition_reference_targets_with_dependencies,inspect_definition_resources_traced,inspect_definition_resources_with_plan_traced,inspect_constructed_resources_with_plan_traced, definition_complete_literal_value_expressions, definition_complete_literal_value_bindings, definition_assess_feature_values, definition_assess_type_sets, definition_complete_fixed_transition_stored_tree,definition_complete_fixed_transition_owner,definition_materialize_transition_members_with_fixed_links,definition_complete_fresh_transition_link,definition_materialize_transition_members,definition_variation_contributions, definition_materialize_variation_typing, definition_materialize_transition_link_redefinition, definition_transition_source_feature, definition_materialize_transition_source, definition_materialize_transition_sources, definition_materialize_usage_multiplicities, definition_materialize_binary_cross_featuring, definition_materialize_owned_cross_specialization, definition_owned_cross_feature, definition_materialize_binary_crossing, definition_materialize_reference_binding_defaults, definition_materialize_reference_binding, definition_materialize_result_feature_defaults, definition_materialize_value_expression_defaults, definition_materialize_value_expression_featuring, definition_assess_valuation_specialization, definition_materialize_bound_value_subsetting, definition_materialize_selected_generals, definition_materialize_general_batch, definition_materialize_feature_defaults, definition_materialize_type_defaults, definition_type_default_contribution_owners, definition_feature_default_contribution_owners, definition_binding_context, definition_is_compatible, definition_all_featuring_types, definition_materialize_reference_result_subsetting, definition_materialize_expression_result, definition_materialize_owned_expression_results, definition_materialize_owning_type_featuring, definition_append_binding_connector, definition_materialize_binding_structure, definition_materialize_binding_defaults, definition_materialize_complete_binding, definition_materialize_resolved_binding, definition_complete_ordinary_features, definition_append_feature_chain, definition_may_time_vary, definition_specializes, definition_reference_targets, definition_attribute_value, definition_attribute_value_with_origins, DefinitionLibraryOriginContext, definition_library_namespace, materialize_definition_document, materialize_definition_resources_with_literal_bindings_traced, stored_standard_default_bindings, stored_plain_type_generalizations, stored_type_inherited_memberships, stored_namespace_imported_memberships, stored_namespace_membership_inputs, stored_package_import_members,generated_effective_names, stored_explicit_general_types, stored_membership_endpoint, StoredMembershipEndpoint};
mod flows;
mod ecore_ownership;
mod ecore_ownership_generated;
mod requirement_memberships;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::{Map, Value, json};

use mercurio_foundation::kir::{KIR_SCHEMA_VERSION, KirDocument, KirElement};
use mercurio_foundation::language_contracts::ast::{BinaryOp, SourceSpan, UnaryOp};
use mercurio_foundation::language_contracts::diagnostics::Diagnostic;
use mercurio_foundation::language_contracts::expression::{
    BinaryExpressionOp, ExpressionIr, ExpressionPathRoot, ExpressionPathSegment, UnaryExpressionOp,
};

use crate::SourceLanguage;
use crate::language_frontend::lowering::elaborate::{
    ReferenceUsageSemantics, UsageFamilyDefaults, dedupe_refs, usage_all_type_refs,
};
use crate::language_frontend::lowering::ir::{
    ResolvedDefinition, ResolvedExpr, ResolvedImport, ResolvedModule, ResolvedPackage,
    ResolvedPathSegment, ResolvedUsage,
};
use crate::language_frontend::lowering::rules::{LoweringRule, LoweringRuleSeed};
use crate::language_frontend::lowering::semantic_actions::{
    apply_usage_actions, usage_action_applies,
};
use crate::language_frontend::lowering::semantic_defaults::{
    ReferenceModifierSemanticsSeed, SemanticDefaultsSeed, UsageActionSeed, UsagePropertyDefaultSeed,
};
use crate::language_frontend::lowering::semantic_properties::{
    apply_usage_property_defaults, usage_property_default_applies,
};
#[derive(Debug, Clone, Deserialize)]
pub struct MetamodelConstructSeed {
    #[serde(default)]
    pub keyword_registry: KeywordRegistrySeed,
    #[serde(default)]
    pub default_specialization_anchors: DefaultSpecializationAnchorsSeed,
    #[serde(default)]
    pub semantic_specialization_defaults: SemanticSpecializationDefaultsSeed,
    #[serde(default)]
    pub usage_semantic_specialization_overrides: UsageSemanticSpecializationOverrideSeed,
    #[serde(default)]
    pub stdlib_aliases: StdlibAliasSeed,
    pub constructs: Vec<MetamodelConstructEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetamodelConstructEntry {
    pub construct: String,
    pub metaclass: String,
}

pub type PilotConstructSeed = MetamodelConstructSeed;
pub type PilotConstructEntry = MetamodelConstructEntry;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct KeywordRegistrySeed {
    #[serde(default)]
    pub definitions: BTreeMap<String, String>,
    #[serde(default)]
    pub usages: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DefaultSpecializationAnchorsSeed {
    #[serde(default)]
    pub packages: BTreeMap<String, String>,
    #[serde(default)]
    pub definitions: BTreeMap<String, String>,
    #[serde(default)]
    pub usages: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SemanticSpecializationDefaultsSeed {
    #[serde(default)]
    pub definitions: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub usages: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UsageSemanticSpecializationOverrideSeed {
    #[serde(default)]
    pub usages: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StdlibAliasSeed {
    #[serde(default)]
    pub ids: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KirEmissionSeed {
    pub metaclasses: BTreeMap<String, EmissionRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmissionRule {
    pub kir_kind: String,
    pub id_template: String,
    pub emit: EmissionSpec,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmissionSpec {
    pub properties: BTreeMap<String, String>,
}

pub struct MappingBundle {
    construct_to_metaclass: HashMap<String, String>,
    package_default_specializations: HashMap<String, String>,
    definition_keyword_constructs: HashMap<String, String>,
    definition_default_specializations: HashMap<String, String>,
    definition_semantic_specializations: HashMap<String, Vec<String>>,
    stdlib_aliases: HashMap<String, String>,
    usage_keyword_constructs: HashMap<String, String>,
    usage_default_specializations: HashMap<String, String>,
    usage_semantic_specializations: HashMap<String, Vec<String>>,
    usage_semantic_specialization_overrides: HashMap<String, HashMap<String, Vec<String>>>,
    kir_emission: KirEmissionSeed,
    lowering_rules: Option<LoweringRuleSeed>,
    semantic_defaults: SemanticDefaultsSeed,
}

impl MappingBundle {
    pub fn load() -> Result<&'static Self, Diagnostic> {
        Self::load_for_language(SourceLanguage::Sysml)
    }

    pub fn load_for_language(_language: SourceLanguage) -> Result<&'static Self, Diagnostic> {
        static KERML_MAPPINGS: OnceLock<Result<MappingBundle, String>> = OnceLock::new();
        static SYSML_MAPPINGS: OnceLock<Result<MappingBundle, String>> = OnceLock::new();

        let mappings = match _language {
            SourceLanguage::Kerml => KERML_MAPPINGS.get_or_init(|| {
                Self::load_uncached_for_language(SourceLanguage::Kerml)
                    .map_err(|err| err.message.clone())
            }),
            SourceLanguage::Sysml => SYSML_MAPPINGS.get_or_init(|| {
                Self::load_uncached_for_language(SourceLanguage::Sysml)
                    .map_err(|err| err.message.clone())
            }),
        };

        match mappings {
            Ok(bundle) => Ok(bundle),
            Err(message) => Err(Diagnostic::new(message.clone(), None)),
        }
    }

    pub fn load_for_profile(profile_id: &str) -> Result<&'static Self, Diagnostic> {
        match canonical_sysml_profile_id(profile_id) {
            Some(_) => Self::load_for_language(SourceLanguage::Sysml),
            None => Err(Diagnostic::new(
                format!("no mapping bundle registered for profile `{profile_id}`"),
                None,
            )),
        }
    }

    fn load_uncached_for_language(language: SourceLanguage) -> Result<Self, Diagnostic> {
        let overlay: KermlMappingOverlay = serde_json::from_str(include_str!(
            "../../../resources/kernel/kerml-mappings.overlay.json"
        ))
        .map_err(|error| {
            Diagnostic::new(format!("invalid KerML mapping overlay: {error}"), None)
        })?;
        let generated: MetamodelConstructSeed = serde_json::from_str(include_str!(
            "../../../resources/kernel/kerml-constructs.seed.json"
        ))
        .map_err(|e| Diagnostic::new(format!("invalid KerML release constructs: {e}"), None))?;
        let kerml_constructs = merge_construct_seeds(
            merge_construct_seeds(kerml_construct_seed(), overlay.constructs),
            generated,
        );
        let kerml_emission = merge_emission_seeds(kerml_emission_seed(), overlay.emission);
        let mut construct_seed = match language {
            SourceLanguage::Kerml => kerml_constructs,
            SourceLanguage::Sysml => {
                let sysml_seed: MetamodelConstructSeed =
                    serde_json::from_str(&load_metamodel_constructs_seed()?).map_err(|err| {
                        Diagnostic::new(format!("failed to parse mapping file: {err}"), None)
                    })?;
                let control_seed: MetamodelConstructSeed = serde_json::from_str(include_str!(
                    "../../../resources/metamodels/sysml-2.0-pilot-2026-08/mappings/control-constructs.seed.json"
                )).map_err(|err| Diagnostic::new(format!("invalid control constructs: {err}"), None))?;
                merge_construct_seeds(
                    merge_construct_seeds(kerml_constructs, sysml_seed),
                    merge_construct_seeds(
                        control_seed,
                        serde_json::from_str(include_str!(
                            "../../../resources/kernel/kerml-constructs.seed.json"
                        ))
                        .map_err(|err| {
                            Diagnostic::new(format!("invalid kernel constructs: {err}"), None)
                        })?,
                    ),
                )
            }
        };
        let kir_emission = match language {
            SourceLanguage::Kerml => kerml_emission,
            SourceLanguage::Sysml => {
                let sysml_emission: KirEmissionSeed =
                    serde_json::from_str(&load_kir_emission_seed()?).map_err(|err| {
                        Diagnostic::new(format!("failed to parse emission file: {err}"), None)
                    })?;
                let control_emission: KirEmissionSeed = serde_json::from_str(include_str!(
                    "../../../resources/metamodels/sysml-2.0-pilot-2026-08/mappings/control-emission.overlay.json"
                )).map_err(|err| Diagnostic::new(format!("invalid control emission: {err}"), None))?;
                let canonical_kinds: KirEmissionSeed = serde_json::from_str(include_str!(
                    "../../../resources/metamodels/sysml-2.0-pilot-2026-08/mappings/emission-kinds.extract.json"
                )).map_err(|err| Diagnostic::new(format!("invalid extracted emission kinds: {err}"), None))?;
                merge_emission_seeds(
                    merge_emission_seeds(
                        merge_emission_seeds(kerml_emission, sysml_emission),
                        canonical_kinds,
                    ),
                    control_emission,
                )
            }
        };
        let lowering_rules = LoweringRuleSeed::load_for_language(language)?.cloned();
        let semantic_defaults = match language {
            SourceLanguage::Kerml => SemanticDefaultsSeed::default(),
            SourceLanguage::Sysml => {
                serde_json::from_str(load_semantic_defaults_seed()).map_err(|err| {
                    Diagnostic::new(
                        format!("failed to parse semantic defaults file: {err}"),
                        None,
                    )
                })?
            }
        };

        let kernel_defaults: ObservedDefinitionDefaults = serde_json::from_str(include_str!(
            "../../../resources/kernel/kernel-defaults-2026-08.json"
        ))
        .map_err(|error| Diagnostic::new(format!("invalid kernel defaults: {error}"), None))?;
        construct_seed
            .semantic_specialization_defaults
            .definitions
            .extend(kernel_defaults.definitions);
        if language == SourceLanguage::Sysml {
            for source in [
                include_str!("../../../resources/kernel/definition-defaults-2026-08.json"),
                include_str!("../../../resources/kernel/view-definition-defaults-2026-08.json"),
                include_str!("../../../resources/kernel/connection-definition-defaults-2026-08.json"),
            ] {
                let observed: ObservedDefinitionDefaults =
                    serde_json::from_str(source).map_err(|error| {
                        Diagnostic::new(format!("invalid observed defaults: {error}"), None)
                    })?;
                for (construct, parents) in observed.definitions {
                    construct_seed
                        .semantic_specialization_defaults
                        .definitions
                        .entry(construct)
                        .or_insert(parents);
                }
            }
        }
        let mut semantic_defaults = semantic_defaults;
        let comment_defaults: SemanticDefaultsSeed = serde_json::from_str(include_str!(
            "../../../resources/kernel/comment-defaults.overlay.json"
        )).map_err(|e| Diagnostic::new(format!("invalid shared comment defaults: {e}"), None))?;
        semantic_defaults.usage_property_defaults.extend(comment_defaults.usage_property_defaults);
        semantic_defaults.usage_resolution_policies.extend(comment_defaults.usage_resolution_policies);
        // Expressions in both source languages inherit Evaluation's features.
        // Keep this default tied to the pinned Pilot observation.
        let expression_defaults: SemanticDefaultsSeed = serde_json::from_str(include_str!(
            "../../../resources/kernel/expression-defaults-2026-08.json"
        ))
        .map_err(|error| Diagnostic::new(format!("invalid expression defaults: {error}"), None))?;
        semantic_defaults
            .usage_type_defaults
            .extend(expression_defaults.usage_type_defaults);
        semantic_defaults
            .usage_subset_defaults
            .extend(expression_defaults.usage_subset_defaults);
        if language == SourceLanguage::Sysml {
            for source in [
                include_str!("../../../resources/kernel/flag-name-defaults.overlay.json"),
                include_str!("../../../resources/kernel/usage-defaults-2026-08.json"),
                include_str!("../../../resources/kernel/single-type-defaults-2026-08.json"),
                include_str!("../../../resources/kernel/include-defaults-2026-08.json"),
                include_str!("../../../resources/kernel/control-usage-defaults-2026-08.json"),
                include_str!("../../../resources/kernel/terminate-defaults-2026-08.json"),
                include_str!("../../../resources/kernel/succession-flow-defaults-2026-08.json"),
                include_str!("../../../resources/kernel/structured-usage-defaults-2026-08.json"),
                include_str!("../../../resources/kernel/occurrence-usage-defaults-2026-08.json"),
            ] {
                let observed: SemanticDefaultsSeed = serde_json::from_str(source).map_err(|e| {
                    Diagnostic::new(format!("invalid observed usage defaults: {e}"), None)
                })?;
                for (construct, default) in observed.usage_type_defaults {
                    semantic_defaults
                        .usage_type_defaults
                        .entry(construct)
                        .or_insert(default);
                }
            }
        }
        Self::from_seeds(
            construct_seed,
            kir_emission,
            lowering_rules,
            semantic_defaults,
        )
    }

    fn from_seeds(
        construct_seed: PilotConstructSeed,
        kir_emission: KirEmissionSeed,
        lowering_rules: Option<LoweringRuleSeed>,
        semantic_defaults: SemanticDefaultsSeed,
    ) -> Result<Self, Diagnostic> {
        Ok(Self {
            package_default_specializations: construct_seed
                .default_specialization_anchors
                .packages
                .clone()
                .into_iter()
                .collect(),
            definition_keyword_constructs: construct_seed
                .keyword_registry
                .definitions
                .clone()
                .into_iter()
                .collect(),
            definition_default_specializations: construct_seed
                .default_specialization_anchors
                .definitions
                .clone()
                .into_iter()
                .collect(),
            definition_semantic_specializations: construct_seed
                .semantic_specialization_defaults
                .definitions
                .clone()
                .into_iter()
                .collect(),
            stdlib_aliases: construct_seed
                .stdlib_aliases
                .ids
                .clone()
                .into_iter()
                .collect(),
            usage_keyword_constructs: construct_seed
                .keyword_registry
                .usages
                .clone()
                .into_iter()
                .collect(),
            usage_default_specializations: construct_seed
                .default_specialization_anchors
                .usages
                .clone()
                .into_iter()
                .collect(),
            usage_semantic_specializations: construct_seed
                .semantic_specialization_defaults
                .usages
                .clone()
                .into_iter()
                .collect(),
            usage_semantic_specialization_overrides: construct_seed
                .usage_semantic_specialization_overrides
                .usages
                .clone()
                .into_iter()
                .map(|(construct, overrides)| (construct, overrides.into_iter().collect()))
                .collect(),
            construct_to_metaclass: construct_seed
                .constructs
                .into_iter()
                .map(|entry| (entry.construct, entry.metaclass))
                .collect(),
            kir_emission,
            lowering_rules,
            semantic_defaults,
        })
    }

    pub fn metaclass_for(&self, construct: &str) -> Result<&str, Diagnostic> {
        if let Some(metaclass) = self.construct_to_metaclass.get(construct) {
            return Ok(metaclass);
        }
        if construct.ends_with("Usage") {
            return Ok("KerML::Feature");
        }
        if construct.ends_with("Definition") {
            return Ok("KerML::Classifier");
        }

        Err(Diagnostic::new(
            format!("missing construct mapping `{construct}`"),
            None,
        ))
    }

    pub fn emission_for(&self, metaclass: &str) -> Result<&EmissionRule, Diagnostic> {
        self.kir_emission
            .metaclasses
            .get(metaclass)
            .ok_or_else(|| Diagnostic::new(format!("missing emission mapping `{metaclass}`"), None))
    }

    pub fn lowering_rule_for_construct(&self, construct: &str) -> Option<&LoweringRule> {
        self.lowering_rules
            .as_ref()?
            .rules
            .iter()
            .find(|rule| rule.construct == construct)
    }

    pub fn lowering_rule_for_ast(&self, node: &str, keyword: &str) -> Option<&LoweringRule> {
        self.lowering_rules
            .as_ref()?
            .rules
            .iter()
            .find(|rule| rule.ast.node == node && rule.ast.keyword.as_deref() == Some(keyword))
    }

    pub fn definition_construct_for(&self, keyword: &str) -> String {
        if let Some(construct) = self.definition_keyword_constructs.get(keyword) {
            return construct.clone();
        }
        if let Some(rule) = self.lowering_rule_for_ast("GenericDefinitionDecl", keyword) {
            return rule.construct.clone();
        }
        self.definition_keyword_constructs
            .get(keyword)
            .cloned()
            .unwrap_or_else(|| format!("{}Definition", pascal_case(keyword)))
    }

    pub fn usage_construct_in_context(&self, keyword: &str, owner: &str) -> String {
        self.lowering_rules
            .as_ref()
            .and_then(|rules| {
                rules
                    .usage_context_overrides
                    .iter()
                    .find(|rule| rule.keyword == keyword && rule.owner == owner)
            })
            .map(|rule| rule.construct.clone())
            .unwrap_or_else(|| self.usage_construct_for(keyword))
    }

    pub fn usage_construct_for(&self, keyword: &str) -> String {
        self.usage_keyword_constructs
            .get(keyword)
            .cloned()
            .or_else(|| {
                self.lowering_rule_for_ast("GenericUsageDecl", keyword)
                    .map(|rule| rule.construct.clone())
            })
            .unwrap_or_else(|| format!("{}Usage", pascal_case(keyword)))
    }

    pub(crate) fn usage_family_default(
        &self,
        construct: &str,
        owner_construct: &str,
    ) -> Option<UsageFamilyDefaults> {
        let default = self
            .semantic_defaults
            .usage_family_defaults
            .get(construct)?;
        let subsetted_feature_refs = default
            .owner_subsetted_feature_refs
            .get(owner_construct)
            .unwrap_or(&default.subsetted_feature_refs)
            .clone();
        Some(UsageFamilyDefaults {
            type_ref: default.type_ref.clone(),
            subsetted_feature_refs,
            is_variable: default.is_variable,
        })
    }

    pub(crate) fn usage_type_for_end_count(&self, construct: &str, count: usize) -> Option<String> {
        self.semantic_defaults.usage_type_defaults.get(construct)?
            .end_count_type_refs.get(&count).cloned()
    }

    pub(crate) fn constant_usage_type_default(&self, construct: &str) -> Option<String> {
        self.semantic_defaults
            .usage_type_defaults
            .get(construct)
            .or_else(|| {
                self.construct_to_metaclass
                    .get(construct)
                    .and_then(|metaclass| {
                        self.semantic_defaults
                            .usage_type_defaults
                            .get(metaclass.rsplit("::").next().unwrap_or(metaclass))
                    })
            })?
            .type_ref
            .clone()
            .filter(|value| !value.contains('{'))
    }

    pub(crate) fn constant_usage_subset_defaults(&self, construct: &str) -> Vec<String> {
        let Some(default) = self.semantic_defaults.usage_subset_defaults.get(construct) else {
            return Vec::new();
        };
        // Context-sensitive defaults are selected later, once the owner is resolved.
        if !default.owner_subsetted_feature_refs.is_empty()
            || !default.modifier_owner_subsetted_feature_refs.is_empty()
            || !default.suppress_default_for_modifiers.is_empty()
            || default.specialized_feature_subset.is_some()
        {
            return Vec::new();
        }
        default.subsetted_feature_refs.clone()
    }

    pub(crate) fn usage_type_default(&self, usage: &ResolvedUsage) -> Option<String> {
        let default = self
            .semantic_defaults
            .usage_type_defaults
            .get(&usage.construct)
            .or_else(|| {
                self.construct_to_metaclass
                    .get(&usage.construct)
                    .and_then(|metaclass| {
                        self.semantic_defaults
                            .usage_type_defaults
                            .get(metaclass.rsplit("::").next().unwrap_or(metaclass))
                    })
            })?;
        let end_count = usage.members.iter().filter(|m| usage_is_end(m)).count();
        if let Some(type_ref) = default.end_count_type_refs.get(&end_count) {
            return Some(resolve_semantic_default_value(type_ref, usage));
        }
        for member in &usage.members {
            if let Some(type_ref) = default.member_type_refs.get(&member.declared_name) {
                return Some(resolve_semantic_default_value(type_ref, usage));
            }
        }
        if let Some(owner_type_ref) = default.owner_type_refs.get(&usage.owner_construct) {
            return Some(resolve_semantic_default_value(owner_type_ref, usage));
        }
        default
            .type_ref
            .as_ref()
            .map(|value| resolve_semantic_default_value(value, usage))
    }

    pub(crate) fn usage_subset_default(&self, usage: &ResolvedUsage) -> Vec<String> {
        let Some(default) = self
            .semantic_defaults
            .usage_subset_defaults
            .get(&usage.construct)
        else {
            return Vec::new();
        };

        if default
            .suppress_default_for_modifiers
            .iter()
            .any(|modifier| {
                usage
                    .modifiers
                    .iter()
                    .any(|usage_modifier| usage_modifier == modifier)
            })
        {
            return Vec::new();
        }

        for modifier in &usage.modifiers {
            if let Some(owner_defaults) =
                default.modifier_owner_subsetted_feature_refs.get(modifier)
                && let Some(refs) = owner_defaults.get(&usage.owner_construct)
            {
                return refs.clone();
            }
        }

        default
            .owner_subsetted_feature_refs
            .get(&usage.owner_construct)
            .cloned()
            .unwrap_or_else(|| default.subsetted_feature_refs.clone())
    }

    pub(crate) fn specialized_feature_subset_default(
        &self,
        usage: &ResolvedUsage,
    ) -> Option<Vec<String>> {
        let default = self
            .semantic_defaults
            .usage_subset_defaults
            .get(&usage.construct)?
            .specialized_feature_subset
            .as_ref()?;

        if usage.specialized_features.is_empty() {
            return None;
        }
        if default.require_feature_ref
            && !usage
                .specialized_features
                .iter()
                .any(|feature| feature.starts_with("feature."))
        {
            return None;
        }
        if default.require_multiplicity && usage.multiplicity.is_none() {
            return None;
        }

        let mut refs = Vec::new();
        if default.include_specialized_features {
            refs.extend(usage.specialized_features.clone());
        }
        if default.require_no_explicit_type_for_append_refs && usage.has_explicit_type {
            return Some(refs);
        }
        refs.extend(
            default
                .owner_append_refs
                .get(&usage.owner_construct)
                .unwrap_or(&default.append_refs)
                .clone(),
        );
        Some(refs)
    }

    pub(crate) fn usage_appends_semantic_specializations_to_subset_defaults(
        &self,
        usage: &ResolvedUsage,
    ) -> bool {
        self.semantic_defaults
            .usage_subset_defaults
            .get(&usage.construct)
            .is_some_and(|default| default.append_semantic_specializations_when_no_defaults)
    }

    pub(crate) fn reference_usage_semantics(
        &self,
        usage: &ResolvedUsage,
    ) -> Option<ReferenceUsageSemantics> {
        if !self
            .semantic_defaults
            .reference_usage_semantics
            .constructs
            .iter()
            .any(|construct| construct == &usage.construct)
        {
            return None;
        }

        let type_refs = usage_all_type_refs(usage);
        let mut semantics = ReferenceUsageSemantics {
            type_refs: type_refs.clone(),
            semantic_specializations: type_refs,
            ..ReferenceUsageSemantics::default()
        };
        let seed = &self.semantic_defaults.reference_usage_semantics;
        if seed.modifier_rules.is_empty()
            && seed.typed_data_value.subsetted_feature_refs.is_empty()
            && seed.typed_object.subsetted_feature_refs.is_empty()
        {
            return None;
        }

        for rule in &seed.modifier_rules {
            if !usage
                .modifiers
                .iter()
                .any(|modifier| modifier == &rule.modifier)
            {
                continue;
            }
            apply_reference_modifier_semantics(rule, usage, &mut semantics);
            return Some(semantics);
        }

        if !semantics.type_refs.is_empty() && all_data_value_like_refs(&semantics.type_refs) {
            semantics
                .subsetted_feature_refs
                .extend(seed.typed_data_value.subsetted_feature_refs.clone());
            return Some(semantics);
        }

        if !semantics.type_refs.is_empty() {
            semantics
                .subsetted_feature_refs
                .extend(seed.typed_object.subsetted_feature_refs.clone());
            semantics.direction = reference_direction_from_modifiers(
                usage,
                &seed.typed_object.direction_from_modifiers,
            );
            return Some(semantics);
        }

        None
    }

    pub(crate) fn reference_usage_has_synthetic_declared_name(
        &self,
        usage: &ResolvedUsage,
    ) -> bool {
        // A payload role does not erase a name explicitly declared in source.
        if !usage.is_implicit_name && usage.modifiers.iter().any(|m| m == "payload") {
            return false;
        }
        self.semantic_defaults
            .reference_usage_semantics
            .constructs
            .iter()
            .any(|construct| construct == &usage.construct)
            && self
                .semantic_defaults
                .reference_usage_semantics
                .modifier_rules
                .iter()
                .any(|rule| {
                    rule.synthetic_declared_name
                        && usage
                            .modifiers
                            .iter()
                            .any(|modifier| modifier == &rule.modifier)
                })
    }

    pub(crate) fn usage_is_variable(&self, usage: &ResolvedUsage) -> bool {
        !self
            .semantic_defaults
            .usage_context
            .non_variable_owner_constructs
            .iter()
            .any(|owner| owner == &usage.owner_construct)
    }

    pub(crate) fn usage_counts_as_owned_member(&self, usage: &ResolvedUsage) -> bool {
        !self
            .semantic_defaults
            .usage_context
            .non_owned_member_constructs
            .iter()
            .any(|construct| construct == &usage.construct)
    }

    pub(crate) fn usage_has_type_context(&self, usage: &ResolvedUsage) -> bool {
        !self
            .semantic_defaults
            .usage_context
            .no_type_context_owner_constructs
            .iter()
            .any(|owner| owner == &usage.owner_construct)
    }

    pub(crate) fn definition_is_abstract(&self, definition: &ResolvedDefinition) -> bool {
        definition.is_abstract
            || self
                .semantic_defaults
                .definition_context
                .abstract_constructs
                .iter()
                .any(|construct| construct == &definition.construct)
    }

    pub(crate) fn usage_direction_from_modifiers<'a>(
        &'a self,
        usage: &ResolvedUsage,
    ) -> Option<&'a str> {
        // Honor profile precedence, then fall back to the shared KerML directions.
        self.semantic_defaults.usage_context.direction_modifiers.iter()
            .find(|direction| usage.modifiers.contains(direction))
            .map(String::as_str)
            .or_else(|| ["inout", "out", "in"].into_iter()
                .find(|direction| usage.modifiers.iter().any(|m| m == direction))
                .and_then(|direction| crate::enum_grammar::shared_literal("FeatureDirection", direction)))
    }

    pub(crate) fn usage_property_defaults(
        &self,
        usage: &ResolvedUsage,
    ) -> Vec<UsagePropertyDefaultSeed> {
        self.semantic_defaults
            .usage_property_defaults
            .get(&usage.construct)
            .into_iter()
            .flatten()
            .filter(|default| usage_property_default_applies(default, usage))
            .cloned()
            .collect()
    }

    pub(crate) fn usage_actions(&self, usage: &ResolvedUsage) -> Vec<UsageActionSeed> {
        self.semantic_defaults
            .usage_actions
            .get(&usage.construct)
            .into_iter()
            .flatten()
            .filter(|action| usage_action_applies(action, usage))
            .cloned()
            .collect()
    }

    pub(crate) fn usage_materialized_specialization_policy(
        &self,
        usage: &ResolvedUsage,
    ) -> Option<&str> {
        self.semantic_defaults
            .usage_specialization_policies
            .get(&usage.construct)
            .and_then(|policy| policy.materialized_refs_policy.as_deref())
    }

    pub(crate) fn usage_specialization_refs_policy(&self, usage: &ResolvedUsage) -> Option<&str> {
        self.semantic_defaults
            .usage_specialization_policies
            .get(&usage.construct)
            .and_then(|policy| policy.specialization_refs_policy.as_deref())
    }

    pub(crate) fn usage_reference_target_resolution_policy(&self, construct: &str) -> Option<&str> {
        self.semantic_defaults
            .usage_resolution_policies
            .get(construct)
            .and_then(|policy| policy.reference_target_policy.as_deref())
    }

    pub(crate) fn usage_connection_end_specialization_policy(
        &self,
        construct: &str,
    ) -> Option<(&str, Option<&str>)> {
        let policy = self
            .semantic_defaults
            .usage_resolution_policies
            .get(construct)?;
        Some((
            policy.connection_end_specialization_policy.as_deref()?,
            policy.connection_end_parent_construct.as_deref(),
        ))
    }

    pub(crate) fn usage_records_previous_state(&self, usage: &ResolvedUsage) -> bool {
        self.semantic_defaults
            .usage_traversal_policies
            .get(&usage.construct)
            .is_some_and(|policy| policy.records_previous_state)
    }

    pub(crate) fn generated_companion_construct_for_definition(
        &self,
        construct: &str,
    ) -> Option<&str> {
        self.semantic_defaults
            .definition_companion_policies
            .get(construct)
            .and_then(|policy| policy.generated_companion_construct.as_deref())
    }

    pub fn default_specialization_for_definition(&self, construct: &str) -> Option<&str> {
        self.definition_default_specializations
            .get(construct)
            .map(String::as_str)
    }

    pub fn default_specialization_for_package(&self, construct: &str) -> Option<&str> {
        self.package_default_specializations
            .get(construct)
            .map(String::as_str)
    }

    pub fn default_specialization_for_usage(&self, construct: &str) -> Option<&str> {
        self.usage_default_specializations
            .get(construct)
            .map(String::as_str)
    }

    pub fn semantic_specializations_for_definition(&self, construct: &str) -> Vec<String> {
        self.definition_semantic_specializations
            .get(construct)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn connection_definition_defaults(
        &self,
        construct: &str,
        end_count: usize,
    ) -> Result<Option<Vec<String>>, Diagnostic> {
        static DEFAULTS: OnceLock<Result<ConnectionDefinitionDefaults, String>> = OnceLock::new();
        let defaults = DEFAULTS.get_or_init(|| serde_json::from_str(include_str!(
            "../../../resources/kernel/connection-defaults.extract.json"
        )).map_err(|error| error.to_string())).as_ref()
            .map_err(|error| Diagnostic::new(format!("invalid connection defaults: {error}"), None))?;
        Ok(defaults.definitions.get(construct).map(|selectors| {
            vec![if end_count == 2 { &selectors.binary } else { &selectors.base }.clone()]
        }))
    }

    pub fn semantic_specializations_for_usage(
        &self,
        construct: &str,
        modifiers: &[String],
    ) -> Vec<String> {
        if let Some(overrides) = self.usage_semantic_specialization_overrides.get(construct) {
            for modifier in modifiers {
                if let Some(targets) = overrides.get(modifier) {
                    return targets.clone();
                }
            }
        }

        self.usage_semantic_specializations
            .get(construct)
            .cloned()
            .unwrap_or_default()
    }

    pub fn stdlib_aliases(&self) -> &HashMap<String, String> {
        &self.stdlib_aliases
    }

    pub fn compatibility_library_aliases(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("Items::Item", "sysml.Item"),
            ("Base::DataValue", "sysml.DataValue"),
            ("Parts::Part", "sysml.Part"),
            ("Ports::Port", "sysml.Port"),
            ("Interfaces::Interface", "sysml.Interface"),
            ("ISQSpaceTime::breadth", "ISQSpaceTime::width"),
            ("breadth", "ISQSpaceTime::width"),
        ]
    }
}

fn kerml_construct_seed() -> PilotConstructSeed {
    let mut definitions = BTreeMap::new();
    for keyword in [
        "assoc",
        "association",
        "behavior",
        "class",
        "classifier",
        "datatype",
        "function",
        "interaction",
        "metaclass",
        "predicate",
        "struct",
    ] {
        definitions.insert(keyword.to_string(), "Classifier".to_string());
    }
    definitions.insert("feature".to_string(), "FeatureDefinition".to_string());

    let mut usages = BTreeMap::new();
    for keyword in ["comment", "doc", "feature", "locale"] {
        usages.insert(keyword.to_string(), "FeatureUsage".to_string());
    }

    let mut packages = BTreeMap::new();
    packages.insert("Package".to_string(), "KerML::Kernel::Package".to_string());

    PilotConstructSeed {
        keyword_registry: KeywordRegistrySeed {
            definitions,
            usages,
        },
        default_specialization_anchors: DefaultSpecializationAnchorsSeed {
            packages,
            ..Default::default()
        },
        semantic_specialization_defaults: SemanticSpecializationDefaultsSeed::default(),
        usage_semantic_specialization_overrides: UsageSemanticSpecializationOverrideSeed::default(),
        stdlib_aliases: StdlibAliasSeed::default(),
        constructs: vec![
            PilotConstructEntry {
                construct: "Package".to_string(),
                metaclass: "KerML::Package".to_string(),
            },
            PilotConstructEntry {
                construct: "Import".to_string(),
                metaclass: "KerML::Import".to_string(),
            },
            PilotConstructEntry {
                construct: "Classifier".to_string(),
                metaclass: "KerML::Classifier".to_string(),
            },
            PilotConstructEntry {
                construct: "FeatureDefinition".to_string(),
                metaclass: "KerML::Classifier".to_string(),
            },
            PilotConstructEntry {
                construct: "FeatureUsage".to_string(),
                metaclass: "KerML::Feature".to_string(),
            },
        ],
    }
}

fn kerml_emission_seed() -> KirEmissionSeed {
    KirEmissionSeed {
        metaclasses: BTreeMap::from([
            (
                "KerML::Package".to_string(),
                emission_rule(
                    "KerML::Package",
                    "pkg.{qualified_name}",
                    &[
                        ("declared_name", "{declared_name}"),
                        ("name", "{name}"),
                        ("owner", "{owner_id}"),
                        ("members", "{member_ids}"),
                        ("metatype", "{metatype_ref}"),
                    ],
                ),
            ),
            (
                "KerML::Import".to_string(),
                emission_rule(
                    "KerML::Import",
                    "import.{owner_id}.{ordinal}",
                    &[
                        ("owner", "{owner_id}"),
                        ("imports", "{target_ref}"),
                        ("metatype", "{metatype_ref}"),
                    ],
                ),
            ),
            (
                "KerML::Classifier".to_string(),
                emission_rule(
                    "KerML::Core::Type",
                    "type.{qualified_name}",
                    &[
                        ("declared_name", "{declared_name}"),
                        ("name", "{name}"),
                        ("owner", "{owner_id}"),
                        ("is_abstract", "{is_abstract}"),
                        ("specializes", "{specializes_refs}"),
                        ("members", "{member_ids}"),
                        ("features", "{owned_feature_ids}"),
                        ("metatype", "{metatype_ref}"),
                    ],
                ),
            ),
            (
                "KerML::Feature".to_string(),
                emission_rule(
                    "KerML::Core::Feature",
                    "feature.{owner_path}.{declared_name}",
                    &[
                        ("owner", "{owner_id}"),
                        ("type", "{type_ref}"),
                        ("declared_name", "{declared_name}"),
                        ("specialized_features", "{specialized_feature_refs}"),
                        ("subsetted_features", "{subsetted_feature_refs}"),
                        ("redefined_features", "{redefined_feature_refs}"),
                        ("members", "{member_ids}"),
                        ("features", "{owned_feature_ids}"),
                        ("metatype", "{metatype_ref}"),
                    ],
                ),
            ),
        ]),
    }
}

fn emission_rule(kir_kind: &str, id_template: &str, properties: &[(&str, &str)]) -> EmissionRule {
    EmissionRule {
        kir_kind: kir_kind.to_string(),
        id_template: id_template.to_string(),
        emit: EmissionSpec {
            properties: properties
                .iter()
                .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
                .collect(),
        },
    }
}

fn merge_construct_seeds(
    mut base: PilotConstructSeed,
    overlay: PilotConstructSeed,
) -> PilotConstructSeed {
    base.keyword_registry
        .definitions
        .extend(overlay.keyword_registry.definitions);
    base.keyword_registry
        .usages
        .extend(overlay.keyword_registry.usages);
    base.default_specialization_anchors
        .packages
        .extend(overlay.default_specialization_anchors.packages);
    base.default_specialization_anchors
        .definitions
        .extend(overlay.default_specialization_anchors.definitions);
    base.default_specialization_anchors
        .usages
        .extend(overlay.default_specialization_anchors.usages);
    base.semantic_specialization_defaults
        .definitions
        .extend(overlay.semantic_specialization_defaults.definitions);
    base.semantic_specialization_defaults
        .usages
        .extend(overlay.semantic_specialization_defaults.usages);
    base.usage_semantic_specialization_overrides
        .usages
        .extend(overlay.usage_semantic_specialization_overrides.usages);
    base.stdlib_aliases.ids.extend(overlay.stdlib_aliases.ids);

    let mut constructs = base
        .constructs
        .into_iter()
        .map(|entry| (entry.construct.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    constructs.extend(
        overlay
            .constructs
            .into_iter()
            .map(|entry| (entry.construct.clone(), entry)),
    );
    base.constructs = constructs.into_values().collect();
    base
}

fn merge_emission_seeds(mut base: KirEmissionSeed, overlay: KirEmissionSeed) -> KirEmissionSeed {
    base.metaclasses.extend(overlay.metaclasses);
    base
}

fn load_metamodel_constructs_seed() -> Result<String, Diagnostic> {
    Ok(crate::resources::SYSML_057_METAMODEL_CONSTRUCTS.to_string())
}

fn canonical_sysml_profile_id(profile_id: &str) -> Option<&'static str> {
    match profile_id {
        "sysml-2.0-metamodel-0.57.0"
        | "sysml-2.0-pilot-0.57.0"
        | "pilot-0.57.0"
        | "0.57.0"
        | "sysml-2.0-pilot-2026-04"
        | "pilot-2026-04"
        | "2026-04" => Some("sysml-2.0-metamodel-0.57.0"),
        _ => None,
    }
}

fn load_kir_emission_seed() -> Result<String, Diagnostic> {
    Ok(crate::resources::SYSML_057_KIR_EMISSION.to_string())
}

fn load_semantic_defaults_seed() -> &'static str {
    crate::resources::SYSML_057_SEMANTIC_DEFAULTS
}

fn resolve_semantic_default_value(value: &str, usage: &ResolvedUsage) -> String {
    match value {
        "$declared_name" => usage.declared_name.clone(),
        "$owner_qualified_name" => usage.owner_qualified_name.clone(),
        _ => value.to_string(),
    }
}

fn apply_reference_modifier_semantics(
    rule: &ReferenceModifierSemanticsSeed,
    usage: &ResolvedUsage,
    semantics: &mut ReferenceUsageSemantics,
) {
    if semantics.type_refs.is_empty()
        && let Some(default_type_ref) = &rule.default_type_ref
    {
        semantics
            .type_refs
            .push(resolve_semantic_default_value(default_type_ref, usage));
    }
    if let Some(semantic_specializations) = &rule.semantic_specializations {
        semantics.semantic_specializations = semantic_specializations
            .iter()
            .map(|value| resolve_semantic_default_value(value, usage))
            .collect();
    }
    semantics.subsetted_feature_refs.extend(
        rule.subsetted_feature_refs
            .iter()
            .map(|value| resolve_semantic_default_value(value, usage)),
    );
    semantics.specialized_feature_refs.extend(
        rule.specialized_feature_refs
            .iter()
            .map(|value| resolve_semantic_default_value(value, usage)),
    );
    semantics.redefined_feature_refs.extend(
        rule.redefined_feature_refs
            .iter()
            .map(|value| resolve_semantic_default_value(value, usage)),
    );
    if let Some(direction) = &rule.direction {
        semantics.direction = Some(direction.clone());
    } else if let Some(direction_modifier) = &rule.direction_from_modifier
        && usage
            .modifiers
            .iter()
            .any(|modifier| modifier == direction_modifier)
    {
        semantics.direction = Some(direction_modifier.clone());
    }
}

fn reference_direction_from_modifiers(
    usage: &ResolvedUsage,
    direction_modifiers: &[String],
) -> Option<String> {
    direction_modifiers
        .iter()
        .find(|direction| {
            usage
                .modifiers
                .iter()
                .any(|modifier| modifier == *direction)
        })
        .cloned()
}

fn all_data_value_like_refs(type_refs: &[String]) -> bool {
    !type_refs.is_empty()
        && type_refs
            .iter()
            .all(|type_ref| is_data_value_like_ref(type_ref))
}

fn is_data_value_like_ref(type_ref: &str) -> bool {
    let tail = type_ref
        .rsplit("::")
        .next()
        .unwrap_or(type_ref)
        .rsplit('.')
        .next()
        .unwrap_or(type_ref);
    matches!(
        tail,
        "Boolean" | "Integer" | "Natural" | "Real" | "Rational" | "String" | "UnlimitedNatural"
    ) || tail.ends_with("Value")
}

fn pascal_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => {
            let mut out = first.to_ascii_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

pub fn transpile_module(
    module: &ResolvedModule,
    source_file: &str,
    mappings: &MappingBundle,
) -> Result<KirDocument, Diagnostic> {
    transpile_module_with_source(module, source_file, "sysml", mappings)
}

pub fn transpile_module_with_source(
    module: &ResolvedModule,
    source_file: &str,
    source_language: &str,
    mappings: &MappingBundle,
) -> Result<KirDocument, Diagnostic> {
    let mut elements = Vec::new();
    let definition_ids = module
        .definitions
        .iter()
        .map(|definition| {
            render_definition_id(definition, mappings)
                .map(|id| (definition.qualified_name.clone(), id))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let conjugated_port_ids = module
        .definitions
        .iter()
        .filter(|definition| {
            mappings
                .generated_companion_construct_for_definition(&definition.construct)
                .is_some()
        })
        .map(|definition| {
            render_conjugated_port_definition_id(definition, mappings)
                .map(|id| (definition.qualified_name.clone(), id))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let package_ids = module
        .packages
        .iter()
        .map(|package| {
            render_package_id(package, mappings).map(|id| (package.qualified_name.clone(), id))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let top_level_usage_ids = module
        .usages
        .iter()
        .map(|usage| {
            let owner_id = package_owner_id(usage, &package_ids);
            render_usage_id(usage, &owner_id, mappings).map(|id| (usage.qualified_name.clone(), id))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let mut namespace_ids = package_ids.clone();
    namespace_ids.extend(definition_ids.clone());
    namespace_ids.extend(top_level_usage_ids.clone());
    for definition in &module.definitions {
        if let Some(owner_id) = definition_ids.get(&definition.qualified_name) {
            index_owned_usage_ids(
                &definition.members,
                owner_id,
                mappings,
                source_language == "kerml",
                &mut namespace_ids,
            )?;
        }
    }
    for usage in &module.usages {
        if let Some(owner_id) = top_level_usage_ids.get(&usage.qualified_name) {
            index_owned_usage_ids(
                &usage.members,
                owner_id,
                mappings,
                source_language == "kerml",
                &mut namespace_ids,
            )?;
        }
    }
    let package_member_ids = build_package_member_ids(
        module,
        &package_ids,
        &definition_ids,
        &top_level_usage_ids,
        mappings,
    );

    for package in &module.packages {
        let package_id = package_ids
            .get(&package.qualified_name)
            .ok_or_else(|| Diagnostic::new("missing package id", None))?;
        let member_ids = package_member_ids
            .get(&package.qualified_name)
            .cloned()
            .unwrap_or_default();
        elements.push(transpile_package(
            package,
            package_id,
            &member_ids,
            &package_ids,
            &definition_ids,
            source_file,
            source_language,
            mappings,
        )?);
        append_documentation_elements(
            &mut elements,
            package_id,
            &package.docs,
            &package.span,
            source_file,
            source_language,
        )?;
    }

    let mut relationship_scopes = BTreeMap::new();
    for alias in &module.aliases {
        let owner_id = namespace_ids.get(&alias.owner_qualified_name).cloned().unwrap_or_else(|| "pkg.root".into());
        let id = format!("membership.alias.{}.{}.{}", alias.qualified_name, alias.span.start_line, alias.span.start_col);
        let target = &alias.target;
        let mut properties = BTreeMap::from([
            ("membership_owning_namespace".into(), json!(owner_id)),
            ("owning_related_element".into(), json!(owner_id)),
            ("member_element".into(), json!(target)),
            ("source".into(), json!([owner_id])),
            ("target".into(), json!([alias.target])),
            ("related_element".into(), json!(dedupe_refs(vec![owner_id.clone(), target.clone()]))),
            ("owned_related_element".into(), json!([])),
            ("visibility".into(), json!(alias.visibility)),
            ("is_implied".into(), json!(false)),
            ("metadata".into(), json!({"source_file":source_file, "source_language":source_language,
                "source_span":alias.span, "lowering":{"construct":"AliasMember", "metaclass":"SysML::Membership"}})),
        ]);
        if !alias.declared_name.is_empty() { properties.insert("member_name".into(), json!(alias.declared_name)); }
        if let Some(short) = &alias.declared_short_name { properties.insert("member_short_name".into(), json!(short)); }
        elements.push(KirElement { id: id.clone(), kind: "SysML::Membership".into(), layer: 2, properties });
        relationship_scopes.insert(alias.qualified_name.clone(), id.clone());
        index_owned_usage_ids(&alias.members, &id, mappings, source_language == "kerml", &mut namespace_ids)?;
        append_documentation_elements(&mut elements, &id, &alias.docs, &alias.span, source_file, source_language)?;
        transpile_usage_tree(&alias.members, &id, source_file, source_language, mappings, &mut elements)?;
    }

    let mut import_objects = Vec::new();
    for import in &module.imports {
        // A namespace query is owned by whatever namespace declares it, which
        // is not always a package: `view v { expose x::**; }` is owned by the
        // view *usage*. Consulting only `package_ids` silently reparented every
        // such query onto `pkg.root`, which made a view's exposes
        // indistinguishable from the next view's â€” fatal for
        // `exposed_elements`, which has to ask what *this* view exposes
        // (save-as-view SV-2).
        let owner_id = import
            .owner_qualified_name
            .as_ref()
            .and_then(|qualified_name| namespace_ids.get(qualified_name))
            .cloned()
            .unwrap_or_else(|| "pkg.root".to_string());
        elements.push(transpile_import(
            import,
            &owner_id,
            source_file,
            source_language,
            mappings,
        )?);
        let import_id = elements
            .last()
            .map(|element| element.id.clone())
            .ok_or_else(|| Diagnostic::new("missing import id", None))?;
        import_objects.push(import_id.clone());
        relationship_scopes.insert(format!("{}.__import_{}_{}", import.owner_qualified_name.as_deref().unwrap_or("root"), import.span.start_line, import.span.start_col), import_id.clone());
        index_owned_usage_ids(&import.members, &import_id, mappings, source_language == "kerml", &mut namespace_ids)?;
        append_documentation_elements(
            &mut elements,
            &import_id,
            &import.docs,
            &import.span,
            source_file,
            source_language,
        )?;
        transpile_usage_tree(&import.members, &import_id, source_file, source_language, mappings, &mut elements)?;

    }

    for definition in &module.definitions {
        let definition_id = definition_ids
            .get(&definition.qualified_name)
            .cloned()
            .ok_or_else(|| Diagnostic::new("missing definition id", None))?;
        let feature_ids = render_owned_usage_tree_ids(
            &definition.members,
            &definition_id,
            mappings,
            source_language == "kerml",
        )?;
        let mut member_ids = feature_ids.clone();
        if let Some(conjugated_id) = conjugated_port_ids.get(&definition.qualified_name) {
            member_ids.push(conjugated_id.clone());
        }
        elements.push(transpile_definition(
            definition,
            &definition_id,
            &feature_ids,
            &member_ids,
            source_file,
            source_language,
            mappings,
        )?);
        append_documentation_elements(
            &mut elements,
            &definition_id,
            &definition.docs,
            &definition.span,
            source_file,
            source_language,
        )?;
        transpile_usage_tree(
            &definition.members,
            &definition_id,
            source_file,
            source_language,
            mappings,
            &mut elements,
        )?;
        if let Some(conjugated_id) = conjugated_port_ids.get(&definition.qualified_name) {
            elements.push(transpile_conjugated_port_definition(
                definition,
                conjugated_id,
                &definition_id,
                source_file,
                source_language,
                mappings,
            )?);
        }
    }

    for usage in &module.usages {
        let owner_id = package_owner_id(usage, &package_ids);
        transpile_usage_tree(
            std::slice::from_ref(usage),
            &owner_id,
            source_file,
            source_language,
            mappings,
            &mut elements,
        )?;
    }

    // Alias objects precede body declarations, but their namespace owners may
    // be definitions or usages emitted later. Link after every owner exists.
    for alias in &module.aliases {
        let owner_id = namespace_ids.get(&alias.owner_qualified_name).cloned().unwrap_or_else(|| "pkg.root".into());
        let id = &relationship_scopes[&alias.qualified_name];
        if let Some(owner) = elements.iter_mut().find(|e| e.id == owner_id) {
            for key in ["owned_relationship", "owned_membership", "membership"] {
                append_unique_property_ref_list(&mut owner.properties, key, id);
            }
        }
    }

    // Every concrete Relationship may own a RelationshipBody. Resolve the
    // indexed source scopes through generated Ecore ancestry after emission,
    // including nested relationship declarations (not only aliases/imports).
    if source_language == "kerml" {
        let relationship_ids = elements.iter()
            .filter_map(|element| match super::relationship_declarations::has_direct_relationship_body(&element.kind) {
                Ok(true) => Some(Ok(element.id.as_str())),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<std::collections::BTreeSet<_>, Diagnostic>>()?;
        for (name, id) in &namespace_ids {
            if relationship_ids.contains(id.as_str()) {
                relationship_scopes.insert(name.clone(), id.clone());
            }
        }
    }
    relationship_bodies::attach(&relationship_scopes, &namespace_ids, source_language == "kerml", &mut elements)?;
    memberships::materialize(module, &namespace_ids, source_file, source_language, &mut elements)?;
    for (import, id) in module.imports.iter().zip(import_objects) {
        let namespace = import.target_id.ends_with("::*") || import.target_id.ends_with("::*::**");
        let target_id = import.referenced_element_id.clone().unwrap_or_else(|| import.target_id.clone());
        if namespace {
            if let Some(target) = elements.iter().find(|e| e.id == target_id) {
                if !super::relationship_declarations::metaclass_is(&target.kind, "Namespace")? {
                    return Err(Diagnostic::semantic("namespace import/expose target must be a Namespace", Some(import.span.clone())));
                }
            }
        }
        let target = if namespace { Some(target_id.clone()) } else {
            import.alias_membership_id.clone().or_else(|| elements.iter().find(|e| e.id == target_id)
                .and_then(|e| e.properties.get("owning_membership")).and_then(Value::as_str).map(str::to_string))
        };
        let owner = import.owner_qualified_name.as_ref().and_then(|q| namespace_ids.get(q)).cloned().unwrap_or_else(|| "pkg.root".into());
        if let Some(parent) = elements.iter_mut().find(|e| e.id == owner) {
            append_unique_property_ref_list(&mut parent.properties, "owned_relationship", &id);
            append_unique_property_ref_list(&mut parent.properties, "owned_import", &id);
        }
        let element = elements.iter_mut().find(|e| e.id == id).unwrap();
        element.properties.insert("source".into(), json!([owner]));
        element.properties.entry("owned_related_element".into()).or_insert(json!([]));
        if let Some(target) = target {
            element.properties.insert(if namespace { "imported_namespace" } else { "imported_membership" }.into(), json!(target));
            element.properties.insert("target".into(), json!([target]));
            element.properties.insert("related_element".into(), json!([owner, target]));
        }
    }

    link_nested_namespaces(module, &namespace_ids, &mut elements);
    if source_language == "kerml" {
        disambiguate_duplicate_element_ids(&mut elements);
    }
    disambiguate_duplicate_source_position_usage_ids(&mut elements);
    if source_language == "sysml" {
        materialize_referenced_state_done(&mut elements);
    }
    memberships::order_source_relationships(&mut elements);
    memberships::qualify_resource_namespace(&mut elements, source_file);
    super::ecore_defaults::apply(&mut elements);
    validate_unique_ids(&elements)?;
    annotation_checks::validate(&elements)?;

    Ok(KirDocument {
        metadata: [
            (
                "kir_schema_version".to_string(),
                Value::String(KIR_SCHEMA_VERSION.to_string()),
            ),
            (
                "source".to_string(),
                Value::String(source_language.to_string()),
            ),
            (
                "parsed_from".to_string(),
                Value::String(source_file.to_string()),
            ),
        ]
        .into_iter()
        .collect(),
        elements,
    })
}

// States::StateAction provides an inherited done endpoint. Materialize a scoped
// usage only for an explicit transition reference and preserve local shadowing.
// This makes the endpoint available to every graph consumer, including views.
fn materialize_referenced_state_done(elements: &mut Vec<KirElement>) {
    let mut generated = BTreeMap::new();
    for transition in elements.iter() {
        let Some(target) = transition.properties.get("target").and_then(Value::as_str) else {
            continue;
        };
        let Some(parent_id) = target.strip_suffix(".done") else {
            continue;
        };
        if transition.properties.get("owner").and_then(Value::as_str) != Some(parent_id)
            || !transition.properties.contains_key("source")
            || elements.iter().any(|element| element.id == target)
        {
            continue;
        }
        let Some(parent) = elements.iter().find(|element| {
            element.id == parent_id
                && element
                    .properties
                    .get("metatype")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| kind.contains("StateUsage"))
        }) else {
            continue;
        };
        generated.entry(target.to_string()).or_insert_with(|| KirElement {
            id: target.into(), kind: parent.kind.clone(), layer: parent.layer,
            properties: BTreeMap::from([
                ("owner".into(), json!(parent_id)),
                ("parent_state".into(), json!(parent_id)),
                ("declared_name".into(), json!("done")),
                ("metatype".into(), json!("SysML::StateUsage")),
                ("is_final".into(), json!(true)),
                ("metadata".into(), json!({"generated":true,"semantic_origin":"States::StateAction::done","source_transition":transition.id})),
            ]),
        });
    }
    elements.extend(generated.into_values());
}

fn package_owner_id(usage: &ResolvedUsage, package_ids: &BTreeMap<String, String>) -> String {
    if usage.owner_qualified_name == "root" {
        "pkg.root".to_string()
    } else if let Some(package_id) = package_ids.get(&usage.owner_qualified_name) {
        package_id.clone()
    } else {
        format!("pkg.{}", usage.owner_qualified_name)
    }
}

fn transpile_package(
    package: &ResolvedPackage,
    package_id: &str,
    member_ids: &[String],
    package_ids: &BTreeMap<String, String>,
    definition_ids: &BTreeMap<String, String>,
    source_file: &str,
    source_language: &str,
    mappings: &MappingBundle,
) -> Result<KirElement, Diagnostic> {
    let construct = if package
        .modifiers
        .iter()
        .any(|modifier| modifier == "library")
    {
        "LibraryPackage"
    } else {
        "Package"
    };
    let metaclass = mappings.metaclass_for(construct)?;
    let emission = mappings.emission_for(metaclass)?;
    let lowering_rule = mappings.lowering_rule_for_construct(construct);
    if let Some(rule) = lowering_rule {
        validate_rule_emission_compatibility(rule, metaclass, emission)?;
    }
    let metatype_ref = mappings
        .default_specialization_for_package(construct)
        .or(Some(metaclass))
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null);
    let context = BTreeMap::from([
        (
            "qualified_name".to_string(),
            Value::String(package.qualified_name.clone()),
        ),
        (
            "declared_name".to_string(),
            Value::String(package.declared_name.clone()),
        ),
        (
            "name".to_string(),
            Value::String(package.declared_name.clone()),
        ),
        (
            "member_ids".to_string(),
            Value::Array(member_ids.iter().cloned().map(Value::String).collect()),
        ),
        (
            "owner_id".to_string(),
            package
                .owner_package_qualified_name
                .as_ref()
                .and_then(|qualified_name| {
                    package_ids
                        .get(qualified_name)
                        .or_else(|| definition_ids.get(qualified_name))
                })
                .cloned()
                .map(Value::String)
                .unwrap_or(Value::Null),
        ),
        ("metatype_ref".to_string(), metatype_ref),
        (
            "is_library".to_string(),
            Value::Bool(construct == "LibraryPackage"),
        ),
    ]);

    let mut element = build_element(
        package_id,
        &package.span,
        source_file,
        source_language,
        emission,
        lowering_rule,
        context,
    )?;
    if construct == "LibraryPackage" {
        element.properties.insert(
            "is_standard".to_string(),
            Value::Bool(
                package
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "standard"),
            ),
        );
    }
    Ok(element)
}

fn validate_rule_emission_compatibility(
    rule: &LoweringRule,
    metaclass: &str,
    emission: &EmissionRule,
) -> Result<(), Diagnostic> {
    if rule.metaclass != metaclass {
        return Err(Diagnostic::new(
            format!(
                "lowering rule `{}` targets `{}` but active mapping uses `{metaclass}`",
                rule.construct, rule.metaclass
            ),
            None,
        ));
    }
    if rule.emit.id_template != emission.id_template {
        return Err(Diagnostic::new(
            format!(
                "lowering rule `{}` id template `{}` does not match emission template `{}`",
                rule.construct, rule.emit.id_template, emission.id_template
            ),
            None,
        ));
    }
    for property in rule.emit.properties.keys() {
        if !emission.emit.properties.contains_key(property) {
            return Err(Diagnostic::new(
                format!(
                    "lowering rule `{}` emits property `{}` missing from active emission mapping `{metaclass}`",
                    rule.construct, property
                ),
                None,
            ));
        }
    }
    Ok(())
}

fn id_template_for_construct<'a>(
    construct: &str,
    metaclass: &str,
    emission: &'a EmissionRule,
    mappings: &'a MappingBundle,
) -> Result<&'a str, Diagnostic> {
    if let Some(rule) = mappings.lowering_rule_for_construct(construct) {
        validate_rule_emission_compatibility(rule, metaclass, emission)?;
        Ok(rule.emit.id_template.as_str())
    } else {
        Ok(emission.id_template.as_str())
    }
}

fn transpile_import(
    import: &ResolvedImport,
    owner_id: &str,
    source_file: &str,
    source_language: &str,
    mappings: &MappingBundle,
) -> Result<KirElement, Diagnostic> {
    // `expose` and `import` share this transpiler because they are
    // parallel metamodel branches, but they must not share a KIR kind:
    // a saved view's scope is an Expose, not an Import (save-as-view SV-1).
    let construct = if import.is_expose { "Expose" } else { "Import" };
    let metaclass = mappings.metaclass_for(construct)?;
    let emission = mappings.emission_for(metaclass)?;
    let lowering_rule = mappings.lowering_rule_for_construct(construct);
    if let Some(rule) = lowering_rule {
        validate_rule_emission_compatibility(rule, metaclass, emission)?;
    }
    let metatype_ref = Value::String(metaclass.to_string());
    let id_template = id_template_for_construct(construct, metaclass, emission, mappings)?;
    let id = render_string(
        id_template,
        &BTreeMap::from([
            ("owner_id".to_string(), Value::String(owner_id.to_string())),
            ("ordinal".to_string(), json!(import.ordinal)),
        ]),
    )?;

    let target_ref = Value::Array(vec![Value::String(import.target_id.clone())]);
    let context = BTreeMap::from([
        ("owner_id".to_string(), Value::String(owner_id.to_string())),
        ("target_ref".to_string(), target_ref.clone()),
        // The emission rule reads `{target_ref}`, but a lowering rule -- which
        // takes precedence when one exists, and one does for `Import` -- reads
        // `$imports`, the name of the property it fills. Binding only
        // `target_ref` left every SysML import emitting an Import element with
        // no target at all: `render_rule_value` found no `imports` key, and
        // `insert_rendered_property` drops nulls silently.
        ("imports".to_string(), target_ref),
        ("ordinal".to_string(), json!(import.ordinal)),
        (
            // Null when absent; `insert_rendered_property` drops it, so an
            // unfiltered query emits no `filter` property at all.
            "filter".to_string(),
            import
                .filter
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        ),
        ("metatype_ref".to_string(), metatype_ref),
    ]);

    let mut element = build_element(
        &id,
        &import.span,
        source_file,
        source_language,
        emission,
        lowering_rule,
        context,
    )?;
    let namespace = import.target_id.ends_with("::*") || import.target_id.ends_with("::*::**");
    let concrete = crate::namespace_grammar::query_kind(source_language == "kerml", import.is_expose, namespace)
        .ok_or_else(|| Diagnostic::new("expose is not a KerML grammar construct", Some(import.span.clone())))?;
    element.kind = concrete.to_string();
    element.properties.insert("metatype".into(), json!(element.kind));
    if let Some(metadata) = element.properties.get_mut("metadata") { metadata["lowering"] = json!({"construct":construct,"metaclass":element.kind}); }
    element.properties.insert("owning_related_element".into(), json!(owner_id));
    element.properties.insert("import_owning_namespace".into(), json!(owner_id));
    element.properties.insert("visibility".into(), json!(import.visibility));
    element.properties.insert("is_implied".into(), json!(false));
    element.properties.insert("is_import_all".into(), json!(import.is_import_all));
    element.properties.insert("is_recursive".into(), json!(import.target_id.ends_with("::**")));
    Ok(element)
}

fn transpile_definition(
    definition: &ResolvedDefinition,
    definition_id: &str,
    feature_ids: &[String],
    member_ids: &[String],
    source_file: &str,
    source_language: &str,
    mappings: &MappingBundle,
) -> Result<KirElement, Diagnostic> {
    let metaclass = mappings.metaclass_for(&definition.construct)?;
    let emission = mappings.emission_for(metaclass)?;
    let lowering_rule = mappings.lowering_rule_for_construct(&definition.construct);
    if let Some(rule) = lowering_rule {
        validate_rule_emission_compatibility(rule, metaclass, emission)?;
    }
    let specializes = semantic_specializations_for_definition(definition, mappings);
    let owner_id = definition_owner_id(definition, mappings)?;
    let metatype_ref = mappings
        .default_specialization_for_definition(&definition.construct)
        .or(Some(metaclass))
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null);
    let context = BTreeMap::from([
        (
            "is_individual".to_string(),
            Value::Bool(definition.construct.starts_with("Individual")),
        ),
        (
            "qualified_name".to_string(),
            Value::String(definition.qualified_name.clone()),
        ),
        (
            "declared_name".to_string(),
            Value::String(definition.declared_name.clone()),
        ),
        (
            "name".to_string(),
            Value::String(definition.declared_name.clone()),
        ),
        (
            "owner_id".to_string(),
            owner_id.clone().map(Value::String).unwrap_or(Value::Null),
        ),
        (
            "specializes_refs".to_string(),
            Value::Array(specializes.iter().cloned().map(Value::String).collect()),
        ),
        (
            "owned_feature_ids".to_string(),
            Value::Array(feature_ids.iter().cloned().map(Value::String).collect()),
        ),
        (
            "member_ids".to_string(),
            Value::Array(member_ids.iter().cloned().map(Value::String).collect()),
        ),
        (
            "is_abstract".to_string(),
            Value::Bool(mappings.definition_is_abstract(definition)),
        ),
        ("metatype_ref".to_string(), metatype_ref),
    ]);

    let mut element = build_element(
        definition_id,
        &definition.span,
        source_file,
        source_language,
        emission,
        lowering_rule,
        context,
    )?;
    if definition.construct == "Namespace" && definition.is_anonymous {
        element.properties.remove("declared_name");
        element.properties.remove("name");
    }
    element.properties.insert("is_abstract".into(), Value::Bool(mappings.definition_is_abstract(definition)));
    if let Some(short) = &definition.declared_short_name {
        element.properties.insert("declared_short_name".into(), json!(short));
    }
    if definition.is_variation {
        element.properties.insert("is_variation".into(), Value::Bool(true));
    }
    Ok(element)
}

fn transpile_conjugated_port_definition(
    definition: &ResolvedDefinition,
    definition_id: &str,
    owner_id: &str,
    source_file: &str,
    source_language: &str,
    mappings: &MappingBundle,
) -> Result<KirElement, Diagnostic> {
    let companion_construct = mappings
        .generated_companion_construct_for_definition(&definition.construct)
        .ok_or_else(|| {
            Diagnostic::new(
                format!(
                    "definition `{}` has no generated companion construct",
                    definition.construct
                ),
                Some(definition.span.clone()),
            )
        })?;
    let metaclass = mappings.metaclass_for(companion_construct)?;
    let emission = mappings.emission_for(metaclass)?;
    let lowering_rule = mappings.lowering_rule_for_construct(companion_construct);
    if let Some(rule) = lowering_rule {
        validate_rule_emission_compatibility(rule, metaclass, emission)?;
    }
    let metatype_ref = mappings
        .default_specialization_for_definition(companion_construct)
        .or(Some(metaclass))
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null);
    let conjugated_name = format!("~{}", definition.declared_name);
    let span = SourceSpan {
        start_line: definition.span.end_line,
        start_col: definition.span.end_col,
        end_line: definition.span.end_line,
        end_col: definition.span.end_col,
    };
    let context = BTreeMap::from([
        (
            "qualified_name".to_string(),
            Value::String(definition.qualified_name.clone()),
        ),
        (
            "declared_name".to_string(),
            Value::String(conjugated_name.clone()),
        ),
        ("name".to_string(), Value::String(conjugated_name)),
        ("owner_id".to_string(), Value::String(owner_id.to_string())),
        ("is_abstract".to_string(), Value::Bool(false)),
        ("metatype_ref".to_string(), metatype_ref),
    ]);

    build_element(
        definition_id,
        &span,
        source_file,
        source_language,
        emission,
        lowering_rule,
        context,
    )
}

fn transpile_usage(
    usage: &ResolvedUsage,
    usage_id: &str,
    owner_id: &str,
    source_file: &str,
    source_language: &str,
    mappings: &MappingBundle,
) -> Result<KirElement, Diagnostic> {
    let metaclass = mappings.metaclass_for(&usage.construct)?;
    let emission = mappings.emission_for(metaclass)?;
    let lowering_rule = mappings.lowering_rule_for_construct(&usage.construct);
    if let Some(rule) = lowering_rule {
        validate_rule_emission_compatibility(rule, metaclass, emission)?;
    }
    let reference_semantics = mappings.reference_usage_semantics(usage);
    let specializes =
        semantic_specializations_for_usage(usage, mappings, reference_semantics.as_ref());
    let subsetted_feature_refs =
        usage_subsetted_feature_refs(usage, mappings, reference_semantics.as_ref());
    let specialized_feature_refs =
        usage_specialized_feature_refs(usage, reference_semantics.as_ref());
    let redefined_feature_refs = usage_redefined_feature_refs(usage, reference_semantics.as_ref());
    let specialization_refs = usage_specialization_refs(
        usage,
        mappings,
        specializes,
        &specialized_feature_refs,
        &subsetted_feature_refs,
        &redefined_feature_refs,
    );
    let materialized_specialization_refs = materialized_usage_specialization_refs(
        usage,
        mappings,
        &specialization_refs,
        &specialized_feature_refs,
    );
    let declared_name_is_synthetic = mappings.reference_usage_has_synthetic_declared_name(usage);
    let usage_name = usage_display_name(usage, mappings);
    let metatype_ref = mappings
        .default_specialization_for_usage(&usage.construct)
        .or(Some(metaclass))
        .map(|value| Value::String(value.to_string()))
        .unwrap_or(Value::Null);
    let context = BTreeMap::from([
        (
            "is_individual".to_string(),
            Value::Bool(usage.construct.starts_with("Individual")),
        ),
        ("owner_id".to_string(), Value::String(owner_id.to_string())),
        (
            "owner_path".to_string(),
            Value::String(usage.owner_qualified_name.clone()),
        ),
        (
            "qualified_name".to_string(),
            Value::String(usage.qualified_name.clone()),
        ),
        (
            "declared_name".to_string(),
            if usage.is_implicit_name || declared_name_is_synthetic {
                Value::Null
            } else {
                Value::String(usage.declared_name.clone())
            },
        ),
        (
            "name".to_string(),
            usage_name.map(Value::String).unwrap_or(Value::Null),
        ),
        (
            "type_ref".to_string(),
            usage_type_ref(usage, mappings, reference_semantics.as_ref())
                .map(Value::String)
                .unwrap_or(Value::Null),
        ),
        (
            "featuring_type_ref".to_string(),
            usage_featuring_type_ref(usage, owner_id, mappings)
                .map(Value::String)
                .unwrap_or(Value::Null),
        ),
        (
            "specializes_refs".to_string(),
            Value::Array(
                materialized_specialization_refs
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        ),
        (
            "specialized_feature_refs".to_string(),
            Value::Array(
                specialized_feature_refs
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        ),
        (
            "subsetted_feature_refs".to_string(),
            Value::Array(
                subsetted_feature_refs
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        ),
        (
            "redefined_feature_refs".to_string(),
            Value::Array(
                redefined_feature_refs
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        ),
        (
            "owned_feature_ids".to_string(),
            Value::Array(
                render_owned_usage_tree_ids(
                    &usage.members,
                    usage_id,
                    mappings,
                    source_language == "kerml",
                )?
                .into_iter()
                .map(Value::String)
                .collect(),
            ),
        ),
        (
            "member_ids".to_string(),
            Value::Array(
                render_owned_usage_tree_ids(
                    &usage.members,
                    usage_id,
                    mappings,
                    source_language == "kerml",
                )?
                .into_iter()
                .map(Value::String)
                .collect(),
            ),
        ),
        (
            "direction".to_string(),
            usage_direction(usage, mappings, reference_semantics.as_ref())
                .map(|value| Value::String(value.to_string()))
                .unwrap_or(Value::Null),
        ),
        (
            "is_abstract".to_string(),
            Value::Bool(
                usage
                    .modifiers
                    .iter()
                    .any(|modifier| modifier == "abstract"),
            ),
        ),
        ("is_derived".to_string(), Value::Bool(usage.is_derived)),
        ("is_end".to_string(), Value::Bool(usage_is_end(usage))),
        ("is_ordered".to_string(), Value::Bool(false)),
        ("is_unique".to_string(), Value::Bool(true)),
        (
            "multiplicity".to_string(),
            usage
                .multiplicity
                .as_ref()
                .map(|multiplicity| Value::String(multiplicity.raw.clone()))
                .unwrap_or(Value::Null),
        ),
        (
            "multiplicity_lower".to_string(),
            usage
                .multiplicity
                .as_ref()
                .map(|multiplicity| Value::String(multiplicity.lower.clone()))
                .unwrap_or(Value::Null),
        ),
        (
            "multiplicity_upper".to_string(),
            usage
                .multiplicity
                .as_ref()
                .map(|multiplicity| Value::String(multiplicity.upper.clone()))
                .unwrap_or(Value::Null),
        ),
        (
            "is_variable".to_string(),
            Value::Bool(mappings.usage_is_variable(usage)),
        ),
        ("metatype_ref".to_string(), metatype_ref),
        ("start_line".to_string(), json!(usage.span.start_line)),
        ("start_col".to_string(), json!(usage.span.start_col)),
    ]);

    let mut element = build_element(
        usage_id,
        &usage_source_span(usage),
        source_file,
        source_language,
        emission,
        lowering_rule,
        context,
    )?;
    if let Some(direction) = usage_direction(usage, mappings, reference_semantics.as_ref()) {
        element.properties.insert(
            "direction".to_string(),
            Value::String(direction.to_string()),
        );
    }
    if crate::language_frontend::lowering::relationship_declarations::metaclass_conforms(
        &element.kind, "OccurrenceUsage",
    ) {
        if let Some(portion) = usage.modifiers.iter().find_map(|modifier|
            crate::enum_grammar::lookup(false, "PortionKind", modifier)) {
            element.properties.insert("portion_kind".into(), json!(portion.literal));
        }
    }
    if matches!(element.kind.rsplit("::").next(), Some("AssertConstraintUsage" | "SatisfyRequirementUsage")) {
        element.properties.insert("is_negated".into(), Value::Bool(usage.modifiers.iter().any(|m| m == "is_negated")));
    }
    if usage_is_end(usage) {
        element.properties.insert("is_end".into(), Value::Bool(true));
    }
    if usage.modifiers.iter().any(|modifier| modifier == "owned_crossing_feature") {
        element.properties.insert("is_end".into(), Value::Bool(false));
    }
    if let Some(index) = usage.members.iter().position(|member| member.modifiers.iter().any(|modifier| modifier == "owned_crossing_feature")) {
        let ids = render_sibling_usage_ids(&usage.members, usage_id, mappings)?;
        if let Some(id) = ids.get(index) {
            element.properties.insert("cross_feature".into(), Value::String(id.clone()));
        }
    }

    if usage.modifiers.iter().any(|m| m == "individual") {
        element
            .properties
            .insert("is_individual".to_string(), Value::Bool(true));
    }
    if let Some(short_name) = usage
        .modifiers
        .iter()
        .find_map(|modifier| modifier.strip_prefix("short_name="))
    {
        element.properties.insert(
            "declared_short_name".to_string(),
            Value::String(short_name.to_string()),
        );
    }
    if let Some(expression) = &usage.expression {
        element.properties.insert(
            "expression_ir".to_string(),
            render_expression_ir(expression)?,
        );
        element.properties.insert(
            "expression_is_initial".to_string(),
            Value::Bool(
                usage
                    .modifiers
                    .iter()
                    .any(|m| m == "feature_value_is_initial"),
            ),
        );
        element.properties.insert(
            "expression_is_default".to_string(),
            Value::Bool(
                usage
                    .modifiers
                    .iter()
                    .any(|m| m == "feature_value_is_default"),
            ),
        );
    }
    if let Some(multiplicity) = &usage.multiplicity {
        element.properties.insert(
            "multiplicity".to_string(),
            Value::String(multiplicity.raw.clone()),
        );
        element.properties.insert(
            "multiplicity_lower".to_string(),
            Value::String(multiplicity.lower.clone()),
        );
        element.properties.insert(
            "multiplicity_upper".to_string(),
            Value::String(multiplicity.upper.clone()),
        );
    }
    if let Some(do_behavior) = state_do_behavior(usage) {
        element
            .properties
            .insert("do_behavior".to_string(), do_behavior);
    }
    if let Some(reference_semantics) = &reference_semantics {
        set_property_refs(
            &mut element.properties,
            "type",
            &reference_semantics.type_refs,
        );
        set_property_refs(
            &mut element.properties,
            "definition",
            &reference_semantics.type_refs,
        );
        if let Some(direction) = &reference_semantics.direction {
            element
                .properties
                .insert("direction".to_string(), Value::String(direction.clone()));
        }
    }
    for specialization_ref in &materialized_specialization_refs {
        append_unique_property_ref_list(&mut element.properties, "specializes", specialization_ref);
    }
    for subsetted_feature_ref in &subsetted_feature_refs {
        append_unique_property_ref_list(
            &mut element.properties,
            "subsetted_features",
            subsetted_feature_ref,
        );
    }
    enrich_usage_semantics(&mut element, usage, owner_id, mappings);
    if usage.modifiers.iter().any(|modifier| modifier == "ordered") {
        element
            .properties
            .insert("is_ordered".into(), Value::Bool(true));
    }
    if usage
        .modifiers
        .iter()
        .any(|modifier| modifier == "nonunique")
    {
        element
            .properties
            .insert("is_unique".into(), Value::Bool(false));
    }
    for (key, value) in &usage.derived_properties {
        element.properties.insert(key.clone(), Value::Bool(*value));
    }
    if usage.construct == "TransitionUsage" {
        let modifier = |prefix: &str| {
            usage
                .modifiers
                .iter()
                .find_map(|value| value.strip_prefix(prefix))
        };
        let guard = modifier("guard=").or_else(|| {
            (modifier("trigger_kind=") == Some("when"))
                .then(|| {
                    modifier("trigger=").map(|text| text.strip_prefix("when ").unwrap_or(text))
                })
                .flatten()
        });
        if let Some(guard) = guard {
            let expression =
                crate::parser::behavior_expression::expression(guard).map_err(|error| {
                    Diagnostic::new(
                        format!("{}: {error}", usage.qualified_name),
                        Some(usage.span.clone()),
                    )
                })?;
            element
                .properties
                .insert("expression_ir".into(), expression);
        }
        if let Some(effect) =
            modifier("effect=").filter(|effect| effect.trim_start().starts_with("assign "))
        {
            let (feature, expression) = crate::parser::behavior_expression::assignment(effect)
                .map_err(|error| {
                    Diagnostic::new(
                        format!("{}: {error}", usage.qualified_name),
                        Some(usage.span.clone()),
                    )
                })?;
            element.properties.insert(
                "effects".into(),
                json!([{
                    "kind":"assign_expression", "feature":feature, "expression":expression,
                    "source":usage.qualified_name,
                }]),
            );
        }
    }
    Ok(element)
}

fn state_do_behavior(usage: &ResolvedUsage) -> Option<Value> {
    if usage.construct != "StateUsage" {
        return None;
    }

    let rates = usage
        .members
        .iter()
        .filter(|member| {
            // A `do action` lowers to PerformActionUsage; keep matching plain
            // ActionUsage for robustness.
            matches!(
                member.construct.as_str(),
                "ActionUsage" | "PerformActionUsage"
            ) && member.modifiers.iter().any(|modifier| modifier == "do")
        })
        .flat_map(do_action_expressions)
        .filter_map(rate_integration_from_assertion)
        .collect::<Vec<_>>();

    (!rates.is_empty()).then(|| {
        json!({
            "kind": "rate_integration",
            "rates": rates
        })
    })
}

fn do_action_expressions(action: &ResolvedUsage) -> Vec<&ResolvedExpr> {
    let mut expressions = Vec::new();
    collect_usage_expressions(action, &mut expressions);
    expressions
}

fn collect_usage_expressions<'a>(
    usage: &'a ResolvedUsage,
    expressions: &mut Vec<&'a ResolvedExpr>,
) {
    if let Some(expression) = &usage.expression {
        expressions.push(expression);
    }
    for member in &usage.members {
        collect_usage_expressions(member, expressions);
    }
}

fn rate_integration_from_assertion(expression: &ResolvedExpr) -> Option<Value> {
    let ResolvedExpr::Binary { left, op, right } = expression else {
        return None;
    };
    if *op != BinaryOp::Equal {
        return None;
    }
    let feature = simple_feature_name(left)?;
    let (base, delta) = match right.as_ref() {
        ResolvedExpr::Binary {
            left,
            op: BinaryOp::Add,
            right,
        } => (left.as_ref(), right.as_ref()),
        _ => return None,
    };
    if simple_feature_name(base)? != feature {
        return None;
    }

    let rate = rate_term_from_duration_product(delta)?;
    Some(match rate {
        RateTerm::Feature(rate_feature) => {
            json!({ "feature": feature, "rate_feature": rate_feature })
        }
        RateTerm::Constant(rate_per_second) => {
            json!({ "feature": feature, "rate_per_second": rate_per_second })
        }
        RateTerm::Expression(expression) => json!({"feature":feature,"rate_expr":expression}),
    })
}

#[derive(Debug, Clone, PartialEq)]
enum RateTerm {
    Feature(String),
    Constant(f64),
    Expression(Value),
}

fn rate_term_from_duration_product(expression: &ResolvedExpr) -> Option<RateTerm> {
    let ResolvedExpr::Binary {
        left,
        op: BinaryOp::Multiply,
        right,
    } = expression
    else {
        return None;
    };

    if simple_feature_name(left.as_ref()).as_deref() == Some("duration") {
        rate_term(right.as_ref())
    } else if simple_feature_name(right.as_ref()).as_deref() == Some("duration") {
        rate_term(left.as_ref())
    } else {
        None
    }
}

fn rate_term(expression: &ResolvedExpr) -> Option<RateTerm> {
    if let Some(feature) = simple_feature_name(expression) {
        return Some(RateTerm::Feature(feature));
    }
    match expression {
        ResolvedExpr::Literal(Value::Number(number)) => number.as_f64().map(RateTerm::Constant),
        _ => render_expression_ir(expression)
            .ok()
            .map(RateTerm::Expression),
    }
}

fn simple_feature_name(expression: &ResolvedExpr) -> Option<String> {
    match expression {
        ResolvedExpr::FeaturePath { segments } if segments.len() == 1 => {
            Some(segments[0].name.clone())
        }
        _ => None,
    }
}

fn build_element(
    id: &str,
    span: &SourceSpan,
    source_file: &str,
    source_language: &str,
    emission: &EmissionRule,
    lowering_rule: Option<&LoweringRule>,
    context: BTreeMap<String, Value>,
) -> Result<KirElement, Diagnostic> {
    let mut properties = BTreeMap::new();
    if let Some(rule) = lowering_rule {
        for (key, expression) in &rule.emit.properties {
            insert_rendered_property(
                &mut properties,
                key,
                render_rule_value(expression, &context),
            );
        }
    } else {
        for (key, template) in &emission.emit.properties {
            insert_rendered_property(&mut properties, key, render_value(template, &context)?);
        }
    }

    if !properties.contains_key("metatype") {
        if let Some(Value::String(metatype)) = context.get("metatype_ref") {
            if !metatype.is_empty() {
                properties.insert("metatype".to_string(), Value::String(metatype.clone()));
            }
        }
    }

    let mut metadata = Map::new();
    metadata.insert(
        "source_file".to_string(),
        Value::String(source_file.to_string()),
    );
    metadata.insert(
        "source_language".to_string(),
        Value::String(source_language.to_string()),
    );
    metadata.insert("generated".to_string(), Value::Bool(false));
    metadata.insert(
        "source_span".to_string(),
        json!({
            "start_line": span.start_line,
            "start_col": span.start_col,
            "end_line": span.end_line,
            "end_col": span.end_col
        }),
    );
    if let Some(rule) = lowering_rule {
        metadata.insert(
            "lowering".to_string(),
            json!({
                "construct": rule.construct,
                "metaclass": rule.metaclass,
                "ast_node": rule.ast.node,
                "ast_keyword": rule.ast.keyword,
                "elaboration_rules": rule
                    .elaborate
                    .iter()
                    .map(|step| step.id.clone())
                    .collect::<Vec<_>>()
            }),
        );
    }
    if !metadata.is_empty() {
        properties.insert("metadata".to_string(), Value::Object(metadata));
    }

    Ok(KirElement {
        id: id.to_string(),
        kind: emission.kir_kind.clone(),
        layer: 2,
        properties,
    })
}

fn insert_rendered_property(properties: &mut BTreeMap<String, Value>, key: &str, value: Value) {
    if is_materialized_derived_property(key) {
        return;
    }
    match &value {
        Value::Null => {}
        Value::Array(values) if values.is_empty() => {}
        Value::String(text) if text.is_empty() => {}
        _ => {
            properties.insert(key.to_string(), value);
        }
    }
}

fn render_rule_value(expression: &str, context: &BTreeMap<String, Value>) -> Value {
    if let Some(key) = expression.strip_prefix('$') {
        return context.get(key).cloned().unwrap_or(Value::Null);
    }
    Value::String(expression.to_string())
}

fn is_materialized_derived_property(key: &str) -> bool {
    matches!(key, "name" | "short_name" | "shortName" | "qualifiedName")
}

fn append_documentation_elements(
    elements: &mut Vec<KirElement>,
    owner_id: &str,
    docs: &[String],
    span: &SourceSpan,
    source_file: &str,
    source_language: &str,
) -> Result<(), Diagnostic> {
    for (index, body) in docs.iter().enumerate() {
        let mut properties = BTreeMap::new();
        properties.insert("owner".to_string(), Value::String(owner_id.to_string()));
        properties.insert("body".to_string(), Value::String(body.clone()));
        properties.insert(
            "source_language".to_string(),
            Value::String(source_language.to_string()),
        );

        let mut metadata = Map::new();
        metadata.insert(
            "source_file".to_string(),
            Value::String(source_file.to_string()),
        );
        metadata.insert(
            "source_language".to_string(),
            Value::String(source_language.to_string()),
        );
        metadata.insert("generated".to_string(), Value::Bool(true));
        metadata.insert(
            "source_span".to_string(),
            json!({
                "start_line": span.start_line,
                "start_col": span.start_col,
                "end_line": span.end_line,
                "end_col": span.end_col
            }),
        );
        properties.insert("metadata".to_string(), Value::Object(metadata));

        let mut element = KirElement {
            id: format!("doc.{owner_id}.{}", index + 1),
            kind: "SysML::Documentation".to_string(),
            layer: 2,
            properties,
        };
        textual_declarations::attach_legacy_documentation(owner_id, &mut element, elements)?;
        elements.push(element);
    }
    Ok(())
}

fn render_value(template: &str, context: &BTreeMap<String, Value>) -> Result<Value, Diagnostic> {
    if let Some(key) = exact_placeholder(template) {
        return Ok(context.get(key).cloned().unwrap_or(Value::Null));
    }

    Ok(Value::String(render_string(template, context)?))
}

fn render_string(template: &str, context: &BTreeMap<String, Value>) -> Result<String, Diagnostic> {
    let mut rendered = template.to_string();
    for (key, value) in context {
        let placeholder = format!("{{{key}}}");
        let replacement = match value {
            Value::String(text) => text.clone(),
            Value::Number(number) => number.to_string(),
            Value::Bool(boolean) => boolean.to_string(),
            Value::Null => String::new(),
            _ => {
                return Err(Diagnostic::new(
                    format!("non-scalar template value for `{key}`"),
                    None,
                ));
            }
        };
        rendered = rendered.replace(&placeholder, &replacement);
    }
    Ok(rendered)
}

fn exact_placeholder(template: &str) -> Option<&str> {
    template
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
}

fn render_package_id(
    package: &ResolvedPackage,
    mappings: &MappingBundle,
) -> Result<String, Diagnostic> {
    let metaclass = mappings.metaclass_for("Package")?;
    let emission = mappings.emission_for(metaclass)?;
    let id_template = id_template_for_construct("Package", metaclass, emission, mappings)?;
    render_string(
        id_template,
        &BTreeMap::from([(
            "qualified_name".to_string(),
            Value::String(package.qualified_name.clone()),
        )]),
    )
}

fn render_owned_usage_tree_ids(
    usages: &[ResolvedUsage],
    owner_id: &str,
    mappings: &MappingBundle,
    disambiguate_siblings: bool,
) -> Result<Vec<String>, Diagnostic> {
    let rendered_ids = if disambiguate_siblings {
        render_sibling_usage_ids(usages, owner_id, mappings)?
    } else {
        usages
            .iter()
            .map(|usage| render_usage_id(usage, owner_id, mappings))
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut ids = Vec::new();
    for (usage, usage_id) in usages.iter().zip(rendered_ids) {
        if mappings.usage_counts_as_owned_member(usage) {
            ids.push(usage_id);
        }
    }
    Ok(ids)
}

fn enrich_usage_semantics(
    element: &mut KirElement,
    usage: &ResolvedUsage,
    owner_id: &str,
    mappings: &MappingBundle,
) {
    if usage.is_implicit_name || mappings.reference_usage_has_synthetic_declared_name(usage) {
        element.properties.remove("declared_name");
        // A redefining feature with no declared name still has an effective
        // name inherited from the redefined feature (SysML v2). The pilot
        // surfaces it as a derived `name`; without it the element has neither
        // name nor declared name and cannot be identified. Emit the derived
        // name only when the KIR carries no `name` already.
        if !element.properties.contains_key("name")
            && !usage.redefined_features.is_empty()
            && let Some(name) = redefined_feature_effective_name(usage)
        {
            element
                .properties
                .insert("name".to_string(), Value::String(name));
        }
    }

    if let Some(defaults) = mappings.usage_family_default(&usage.construct, &usage.owner_construct)
    {
        insert_property_ref_if_missing(&mut element.properties, "type", &defaults.type_ref);
        for family_ref in &defaults.subsetted_feature_refs {
            append_unique_property_ref_list(
                &mut element.properties,
                "subsetted_features",
                family_ref,
            );
            append_unique_property_ref_list(&mut element.properties, "specializes", family_ref);
        }
        element
            .properties
            .insert("is_unique".to_string(), Value::Bool(true));
        element
            .properties
            .insert("is_variable".to_string(), Value::Bool(defaults.is_variable));
    }
    apply_usage_property_defaults(element, usage, owner_id, mappings);
    if usage.has_explicit_type {
        if let Some(type_ref) = &usage.type_ref {
            prepend_unique_property_ref(&mut element.properties, "definition", type_ref);
        }
    }
    for type_ref in &usage.additional_type_refs {
        append_unique_property_ref_list(&mut element.properties, "type", type_ref);
        if usage.has_explicit_type {
            prepend_unique_property_ref(&mut element.properties, "definition", type_ref);
        }
    }
    if !element.properties.contains_key("definition")
        && let Some(type_ref) = element.properties.get("type").cloned()
    {
        element
            .properties
            .insert("definition".to_string(), type_ref);
    }

    if mappings.usage_has_type_context(usage) {
        if !element.properties.contains_key("featuring_type") {
            element.properties.insert(
                "featuring_type".to_string(),
                Value::String(owner_id.to_string()),
            );
        }
        element.properties.insert(
            "owning_type".to_string(),
            Value::String(owner_id.to_string()),
        );
        element.properties.insert(
            "owning_namespace".to_string(),
            Value::String(owner_id.to_string()),
        );
        if usage.owner_construct.ends_with("Definition") {
            element.properties.insert(
                "owning_definition".to_string(),
                Value::String(owner_id.to_string()),
            );
        }
    }
}

fn usage_display_name(usage: &ResolvedUsage, mappings: &MappingBundle) -> Option<String> {
    if usage.is_implicit_name {
        return mappings
            .usage_family_default(&usage.construct, &usage.owner_construct)
            .and_then(|defaults| defaults.subsetted_feature_refs.last().cloned())
            .map(|value| display_name_for_ref(&value))
            .or_else(|| (!usage.declared_name.is_empty()).then(|| usage.declared_name.clone()));
    }

    (!usage.declared_name.is_empty()).then(|| usage.declared_name.clone())
}

// An unnamed redefining feature takes the redefined feature's name as its
// effective name (SysML v2 redefinition semantics); the pilot derives the
// same `name` while leaving the declared name unset.
fn redefined_feature_effective_name(usage: &ResolvedUsage) -> Option<String> {
    if !usage.declared_name.is_empty() {
        return Some(usage.declared_name.clone());
    }
    usage
        .redefined_features
        .first()
        .map(|value| display_name_for_ref(value))
}

pub(crate) fn modifier_value<'a>(modifiers: &'a [String], key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    modifiers
        .iter()
        .find_map(|modifier| modifier.strip_prefix(&prefix))
}

pub(crate) fn sibling_state_id(owner_qualified_name: &str, target: &str) -> Option<String> {
    let target_qualified = if target.contains('.') {
        target.to_string()
    } else {
        format!("{owner_qualified_name}.{target}")
    };
    Some(format!("state.{target_qualified}"))
}

fn display_name_for_ref(value: &str) -> String {
    value
        .rsplit("::")
        .next()
        .unwrap_or(value)
        .rsplit('.')
        .next()
        .unwrap_or(value)
        .to_string()
}

pub(crate) fn append_unique_property_ref(
    properties: &mut BTreeMap<String, Value>,
    key: &str,
    value: &str,
) {
    let updated = match properties.get(key) {
        Some(Value::String(existing)) if existing == value => return,
        Some(Value::String(existing)) => Value::Array(vec![
            Value::String(existing.clone()),
            Value::String(value.to_string()),
        ]),
        Some(Value::Array(values)) => {
            if values.iter().any(|item| item.as_str() == Some(value)) {
                return;
            }
            let mut next = values.clone();
            next.push(Value::String(value.to_string()));
            Value::Array(next)
        }
        Some(Value::Null) | None => Value::String(value.to_string()),
        Some(other) => Value::Array(vec![other.clone(), Value::String(value.to_string())]),
    };

    properties.insert(key.to_string(), updated);
}

fn prepend_unique_property_ref(properties: &mut BTreeMap<String, Value>, key: &str, value: &str) {
    let updated = match properties.get(key) {
        Some(Value::String(existing)) if existing == value => return,
        Some(Value::String(existing)) => Value::Array(vec![
            Value::String(value.to_string()),
            Value::String(existing.clone()),
        ]),
        Some(Value::Array(values)) => {
            if values.iter().any(|item| item.as_str() == Some(value)) {
                return;
            }
            let mut next = Vec::with_capacity(values.len() + 1);
            next.push(Value::String(value.to_string()));
            next.extend(values.clone());
            Value::Array(next)
        }
        Some(Value::Null) | None => Value::String(value.to_string()),
        Some(other) => Value::Array(vec![Value::String(value.to_string()), other.clone()]),
    };

    properties.insert(key.to_string(), updated);
}

fn insert_property_ref_if_missing(
    properties: &mut BTreeMap<String, Value>,
    key: &str,
    value: &str,
) {
    if !matches!(properties.get(key), Some(existing) if !existing.is_null()) {
        properties.insert(key.to_string(), Value::String(value.to_string()));
    }
}

pub(crate) fn append_unique_property_ref_list(
    properties: &mut BTreeMap<String, Value>,
    key: &str,
    value: &str,
) {
    let updated = match properties.get(key) {
        Some(Value::Array(values)) => {
            if values.iter().any(|item| item.as_str() == Some(value)) {
                return;
            }
            let mut next = values.clone();
            next.push(Value::String(value.to_string()));
            Value::Array(next)
        }
        Some(Value::String(existing)) if existing == value => {
            Value::Array(vec![Value::String(existing.clone())])
        }
        Some(Value::String(existing)) => Value::Array(vec![
            Value::String(existing.clone()),
            Value::String(value.to_string()),
        ]),
        Some(Value::Null) | None => Value::Array(vec![Value::String(value.to_string())]),
        Some(other) => Value::Array(vec![other.clone(), Value::String(value.to_string())]),
    };

    properties.insert(key.to_string(), updated);
}

fn transpile_usage_tree(
    usages: &[ResolvedUsage],
    owner_id: &str,
    source_file: &str,
    source_language: &str,
    mappings: &MappingBundle,
    elements: &mut Vec<KirElement>,
) -> Result<(), Diagnostic> {
    let rendered_ids = if source_language == "kerml" {
        render_sibling_usage_ids(usages, owner_id, mappings)?
    } else {
        usages
            .iter()
            .map(|usage| render_usage_id(usage, owner_id, mappings))
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut previous_state_id = None::<String>;
    for (usage, usage_id) in usages.iter().zip(rendered_ids) {
        let mut element = transpile_usage(
            usage,
            &usage_id,
            owner_id,
            source_file,
            source_language,
            mappings,
        )?;
        apply_usage_actions(
            elements,
            &mut element,
            usage,
            owner_id,
            previous_state_id.as_deref(),
            mappings,
        );
        if let Some((source, targets)) = usage.related_features.split_first() {
            set_property_refs(&mut element.properties, "source", std::slice::from_ref(source));
            element.properties.insert("source_feature".into(), json!(source));
            set_property_refs(&mut element.properties, "target", targets);
            element.properties.insert("target_feature".into(), json!(targets));
            element.properties.insert("related_feature".into(), json!(usage.related_features));
            element.properties.insert("related_element".into(), json!(dedupe_refs(usage.related_features.clone())));
        }
        textual_declarations::emit(usage, owner_id, &mut element, elements)?;
        super::relationship_declarations::emit(usage, &mut element, elements)?;
        super::relationship_declarations::emit_multiplicity_subsettings(usage, &mut element, elements)?;
        multiplicity_ranges::emit(usage, &mut element, elements)?;
        flows::emit(usage, &mut element, elements)?;
        requirement_memberships::emit(usage, owner_id, &mut element, elements)?;
        if (matches!(element.kind.rsplit("::").next(),
                Some("IncludeUseCaseUsage" | "PerformActionUsage" | "ExhibitStateUsage"
                    | "AssertConstraintUsage" | "SatisfyRequirementUsage" | "ConstraintUsage"))
            || element.kind.rsplit("::").next() == Some("Feature")
            || (matches!(usage.construct.as_str(), "ReferenceUsage" | "EndUsage" | "PortUsage")
                && usage.modifiers.iter().any(|m| m == "end" || m.starts_with("end-"))))
            && let Some(target) = &usage.reference_target
        {
            let relationship_id = format!("{usage_id}.reference-subsetting");
            element
                .properties
                .insert("owned_reference_subsetting".into(), json!(relationship_id));
            append_unique_property_ref_list(
                &mut element.properties,
                "owned_relationship",
                &relationship_id,
            );
            append_unique_property_ref_list(&mut element.properties, "subsetted_features", target);
            append_unique_property_ref_list(&mut element.properties, "specializes", target);
            let mut properties = BTreeMap::from([
                ("owning_related_element".into(), json!(usage_id)),
                ("subsetting_feature".into(), json!(usage_id)),
                ("subsetted_feature".into(), json!(target)),
                ("referenced_feature".into(), json!(target)),
                ("source".into(), json!(usage_id)),
                ("target".into(), json!(target)),
                ("related_element".into(), json!([usage_id, target])),
                ("is_implied".into(), json!(false)),
            ]);
            if let Some(metadata) = element.properties.get("metadata") {
                properties.insert("metadata".into(), metadata.clone());
            }
            elements.push(KirElement {
                id: relationship_id,
                kind: "SysML::ReferenceSubsetting".into(),
                layer: 2,
                properties,
            });
        }
        elements.push(element);
        append_documentation_elements(
            elements,
            &usage_id,
            &usage.docs,
            &usage.span,
            source_file,
            source_language,
        )?;
        transpile_usage_tree(
            &usage.members,
            &usage_id,
            source_file,
            source_language,
            mappings,
            elements,
        )?;
        if mappings.usage_records_previous_state(usage) {
            previous_state_id = Some(usage_id);
        }
    }
    Ok(())
}

fn index_owned_usage_ids(
    usages: &[ResolvedUsage],
    owner_id: &str,
    mappings: &MappingBundle,
    disambiguate: bool,
    ids: &mut BTreeMap<String, String>,
) -> Result<(), Diagnostic> {
    let rendered = if disambiguate {
        render_sibling_usage_ids(usages, owner_id, mappings)?
    } else {
        usages
            .iter()
            .map(|usage| render_usage_id(usage, owner_id, mappings))
            .collect::<Result<Vec<_>, _>>()?
    };
    for (usage, id) in usages.iter().zip(rendered) {
        index_owned_usage_ids(&usage.members, &id, mappings, disambiguate, ids)?;
        ids.insert(usage.qualified_name.clone(), id);
    }
    Ok(())
}

fn link_nested_namespaces(
    module: &ResolvedModule,
    ids: &BTreeMap<String, String>,
    elements: &mut [KirElement],
) {
    let positions = elements
        .iter()
        .enumerate()
        .map(|(index, e)| (e.id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let namespaces = module
        .packages
        .iter()
        .map(|p| p.qualified_name.as_str())
        .chain(module.definitions.iter().map(|d| d.qualified_name.as_str()));
    for name in namespaces {
        let Some((owner, _)) = name.rsplit_once('.') else {
            continue;
        };
        let (Some(child_id), Some(owner_id)) = (ids.get(name), ids.get(owner)) else {
            continue;
        };
        let (Some(&child_index), Some(&owner_index)) =
            (positions.get(child_id), positions.get(owner_id))
        else {
            continue;
        };
        elements[child_index]
            .properties
            .insert("owner".to_string(), json!(owner_id));
        append_unique_property_ref_list(&mut elements[owner_index].properties, "members", child_id);
    }
}

fn render_sibling_usage_ids(
    usages: &[ResolvedUsage],
    owner_id: &str,
    mappings: &MappingBundle,
) -> Result<Vec<String>, Diagnostic> {
    let base_ids = usages
        .iter()
        .map(|usage| render_usage_id(usage, owner_id, mappings))
        .collect::<Result<Vec<_>, _>>()?;
    let mut counts = BTreeMap::<String, usize>::new();
    for id in &base_ids {
        *counts.entry(id.clone()).or_default() += 1;
    }

    Ok(usages
        .iter()
        .zip(base_ids)
        .map(|(usage, id)| {
            if counts.get(&id).copied().unwrap_or_default() <= 1 {
                id
            } else {
                format!("{}.{}_{}", id, usage.span.start_line, usage.span.start_col)
            }
        })
        .collect())
}

fn render_definition_id(
    definition: &ResolvedDefinition,
    mappings: &MappingBundle,
) -> Result<String, Diagnostic> {
    let metaclass = mappings.metaclass_for(&definition.construct)?;
    let emission = mappings.emission_for(metaclass)?;
    let id_template =
        id_template_for_construct(&definition.construct, metaclass, emission, mappings)?;
    render_string(
        id_template,
        &BTreeMap::from([(
            "qualified_name".to_string(),
            Value::String(definition.qualified_name.clone()),
        )]),
    )
}

pub(crate) fn render_usage_id(
    usage: &ResolvedUsage,
    owner_id: &str,
    mappings: &MappingBundle,
) -> Result<String, Diagnostic> {
    render_usage_identity(
        &usage.construct,
        &usage.owner_qualified_name,
        &usage.declared_name,
        usage.is_implicit_name,
        !usage.redefined_features.is_empty(),
        &usage.span,
        owner_id,
        mappings,
    )
}

pub(crate) fn render_collected_usage_id(
    usage: &super::collect::CollectedUsage,
    owner_id: &str,
    mappings: &MappingBundle,
) -> Result<String, Diagnostic> {
    render_usage_identity(
        &usage.construct,
        &usage.owner_qualified_name,
        &usage.declared_name,
        usage.is_implicit_name,
        !usage.redefines.is_empty(),
        &usage.span,
        owner_id,
        mappings,
    )
}

fn render_usage_identity(
    construct: &str,
    owner_path: &str,
    declared_name: &str,
    is_implicit_name: bool,
    has_redefinitions: bool,
    span: &SourceSpan,
    owner_id: &str,
    mappings: &MappingBundle,
) -> Result<String, Diagnostic> {
    let metaclass = mappings.metaclass_for(construct)?;
    let emission = mappings.emission_for(metaclass)?;
    let id_template = id_template_for_construct(construct, metaclass, emission, mappings)?;
    let mut id = render_string(
        id_template,
        &BTreeMap::from([
            ("owner_id".to_string(), Value::String(owner_id.to_string())),
            (
                "owner_path".to_string(),
                Value::String(owner_path.to_string()),
            ),
            (
                "declared_name".to_string(),
                Value::String(declared_name.to_string()),
            ),
            ("start_line".to_string(), json!(span.start_line)),
            ("start_col".to_string(), json!(span.start_col)),
        ]),
    )?;
    // The named overlay preserves role IDs used by existing consumers while
    // the element itself uses the actual ConstraintUsage metaclass/template.
    if let Some(prefix) = requirement_memberships::compatibility_id_prefix(construct)? {
        if let Some((_, tail)) = id.split_once('.') {
            id = format!("{prefix}.{tail}");
        }
    }
    let append_location = mappings
        .semantic_defaults
        .usage_id_policies
        .get(construct)
        .is_some_and(|policy| policy.append_source_location_if_missing_start_col);
    if ((is_implicit_name && !has_redefinitions) || append_location)
        && !id.ends_with(&format!(".{}_{}", span.start_line, span.start_col))
        && !id_template.contains("{start_line}")
    {
        id = format!("{}.{}_{}", id, span.start_line, span.start_col);
    }
    Ok(id)
}

fn render_conjugated_port_definition_id(
    definition: &ResolvedDefinition,
    mappings: &MappingBundle,
) -> Result<String, Diagnostic> {
    let companion_construct = mappings
        .generated_companion_construct_for_definition(&definition.construct)
        .ok_or_else(|| {
            Diagnostic::new(
                format!(
                    "definition `{}` has no generated companion construct",
                    definition.construct
                ),
                Some(definition.span.clone()),
            )
        })?;
    let metaclass = mappings.metaclass_for(companion_construct)?;
    let emission = mappings.emission_for(metaclass)?;
    let id_template =
        id_template_for_construct(companion_construct, metaclass, emission, mappings)?;
    render_string(
        id_template,
        &BTreeMap::from([
            (
                "qualified_name".to_string(),
                Value::String(definition.qualified_name.clone()),
            ),
            (
                "declared_name".to_string(),
                Value::String(definition.declared_name.clone()),
            ),
        ]),
    )
}

fn build_package_member_ids(
    module: &ResolvedModule,
    package_ids: &BTreeMap<String, String>,
    definition_ids: &BTreeMap<String, String>,
    usage_ids: &BTreeMap<String, String>,
    mappings: &MappingBundle,
) -> BTreeMap<String, Vec<String>> {
    module
        .packages
        .iter()
        .map(|package| {
            let child_packages = module
                .packages
                .iter()
                .filter(|candidate| {
                    is_direct_child(&candidate.qualified_name, &package.qualified_name)
                })
                .filter_map(|candidate| package_ids.get(&candidate.qualified_name).cloned());
            let child_definitions = module
                .definitions
                .iter()
                .filter(|candidate| {
                    is_direct_child(&candidate.qualified_name, &package.qualified_name)
                })
                .filter_map(|candidate| definition_ids.get(&candidate.qualified_name).cloned());
            let child_usages = module
                .usages
                .iter()
                .filter(|candidate| mappings.usage_counts_as_owned_member(candidate))
                .filter(|candidate| {
                    is_direct_child(&candidate.qualified_name, &package.qualified_name)
                })
                .filter_map(|candidate| usage_ids.get(&candidate.qualified_name).cloned());
            (
                package.qualified_name.clone(),
                child_packages
                    .chain(child_definitions)
                    .chain(child_usages)
                    .collect(),
            )
        })
        .collect()
}

fn is_direct_child(candidate: &str, parent: &str) -> bool {
    let Some(remainder) = candidate.strip_prefix(parent) else {
        return false;
    };
    let Some(remainder) = remainder.strip_prefix('.') else {
        return false;
    };
    !remainder.contains('.')
}

fn definition_owner_id(
    definition: &ResolvedDefinition,
    mappings: &MappingBundle,
) -> Result<Option<String>, Diagnostic> {
    let Some((owner, _)) = definition.qualified_name.rsplit_once('.') else {
        return Ok(None);
    };

    let package = ResolvedPackage {
        owner_package_qualified_name: None,
        qualified_name: owner.to_string(),
        declared_name: owner.rsplit('.').next().unwrap_or(owner).to_string(),
        modifiers: Vec::new(),
        docs: Vec::new(),
        span: definition.span.clone(),
    };
    render_package_id(&package, mappings).map(Some)
}

fn semantic_specializations_for_definition(
    definition: &ResolvedDefinition,
    mappings: &MappingBundle,
) -> Vec<String> {
    if definition.specializes.is_empty() {
        mappings.semantic_specializations_for_definition(&definition.construct)
    } else {
        definition.specializes.clone()
    }
}

fn semantic_specializations_for_usage(
    usage: &ResolvedUsage,
    mappings: &MappingBundle,
    reference_semantics: Option<&ReferenceUsageSemantics>,
) -> Vec<String> {
    if let Some(reference_semantics) = reference_semantics {
        return reference_semantics.semantic_specializations.clone();
    }

    if !usage.specializes.is_empty() {
        return usage.specializes.clone();
    }

    if usage.is_implicit_name
        && !usage.redefined_features.is_empty()
        && usage.specialized_features.is_empty()
        && usage.subsetted_features.is_empty()
    {
        return Vec::new();
    }

    if !usage.has_explicit_type
        && (!usage.specialized_features.is_empty()
            || !usage.subsetted_features.is_empty()
            || !usage.redefined_features.is_empty())
    {
        return Vec::new();
    }

    let mut specializes = Vec::new();
    if mappings.usage_specialization_refs_policy(usage)
        != Some("merge_feature_refs_into_semantic_specializations")
    {
        if let Some(type_ref) = usage.type_ref.clone() {
            specializes.push(type_ref);
        } else if usage.construct == "EnumerationUsage"
            && usage.owner_construct == "EnumerationDefinition"
        {
            specializes.push(usage.owner_qualified_name.clone());
        }
    }
    specializes
}

fn usage_specialization_refs(
    usage: &ResolvedUsage,
    mappings: &MappingBundle,
    mut semantic_specializations: Vec<String>,
    specialized_feature_refs: &[String],
    subsetted_feature_refs: &[String],
    redefined_feature_refs: &[String],
) -> Vec<String> {
    if mappings.usage_specialization_refs_policy(usage)
        == Some("merge_feature_refs_into_semantic_specializations")
    {
        semantic_specializations.extend(specialized_feature_refs.iter().cloned());
        semantic_specializations.extend(subsetted_feature_refs.iter().cloned());
        semantic_specializations.extend(redefined_feature_refs.iter().cloned());
        return dedupe_refs(semantic_specializations);
    }

    if mappings.usage_specialization_refs_policy(usage)
        == Some(
            "suppress_feature_refs_for_explicit_type_specialized_features_without_redefinitions",
        )
        && usage.has_explicit_type
        && !usage.specialized_features.is_empty()
        && redefined_feature_refs.is_empty()
    {
        semantic_specializations.extend(redefined_feature_refs.iter().cloned());
        return dedupe_refs(semantic_specializations);
    }

    semantic_specializations.extend(specialized_feature_refs.iter().cloned());
    semantic_specializations.extend(subsetted_feature_refs.iter().cloned());
    semantic_specializations.extend(redefined_feature_refs.iter().cloned());
    dedupe_refs(semantic_specializations)
}

fn materialized_usage_specialization_refs(
    usage: &ResolvedUsage,
    mappings: &MappingBundle,
    specialization_refs: &[String],
    specialized_feature_refs: &[String],
) -> Vec<String> {
    if mappings.usage_materialized_specialization_policy(usage)
        == Some("prepend_feature_for_specialized_actions_without_multiplicity")
        && usage.multiplicity.is_none()
    {
        let specialized = specialized_feature_refs
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut refs = specialization_refs
            .iter()
            .filter(|value| !specialized.contains(*value))
            .cloned()
            .collect::<Vec<_>>();
        if !specialized.is_empty() {
            refs.insert(0, "kerml.Feature".to_string());
        }
        return dedupe_refs(refs);
    }

    specialization_refs.to_vec()
}

fn usage_is_end(usage: &ResolvedUsage) -> bool {
    usage
        .modifiers
        .iter()
        .any(|modifier| modifier == "end" || modifier.starts_with("end-"))
}

fn usage_featuring_type_ref(
    usage: &ResolvedUsage,
    owner_id: &str,
    mappings: &MappingBundle,
) -> Option<String> {
    mappings
        .usage_has_type_context(usage)
        .then(|| owner_id.to_string())
}

fn usage_direction<'a>(
    usage: &'a ResolvedUsage,
    mappings: &'a MappingBundle,
    reference_semantics: Option<&'a ReferenceUsageSemantics>,
) -> Option<&'a str> {
    if let Some(reference_semantics) = reference_semantics
        && let Some(direction) = reference_semantics.direction.as_deref()
    {
        return Some(direction);
    }

    if usage.modifiers.iter().any(|modifier| modifier == "return") {
        return Some("out");
    }
    mappings.usage_direction_from_modifiers(usage)
}

fn usage_source_span(usage: &ResolvedUsage) -> SourceSpan {
    let mut span = usage.span.clone();
    if !usage.members.is_empty() && span.end_line > span.start_line {
        span.end_line -= 1;
    }
    span
}

fn render_expression_ir(expr: &ResolvedExpr) -> Result<Value, Diagnostic> {
    build_expression_ir(expr)?
        .to_value()
        .map_err(|err| Diagnostic::new(format!("failed to serialize expression_ir: {err}"), None))
}

fn build_expression_ir(expr: &ResolvedExpr) -> Result<ExpressionIr, Diagnostic> {
    match expr {
        ResolvedExpr::Operation { operator, operands } => Ok(ExpressionIr::Operation {
            operator: operator.clone(),
            operands: operands
                .iter()
                .map(build_expression_ir)
                .collect::<Result<Vec<_>, _>>()?,
        }),
        ResolvedExpr::TypeReference { target } => Ok(ExpressionIr::TypeReference {
            target: target.clone(),
        }),
        ResolvedExpr::Variable { name } => Ok(ExpressionIr::Variable { name: name.clone() }),
        ResolvedExpr::NamedArgument { parameter, value } => Ok(ExpressionIr::NamedArgument {
            parameter: parameter.clone(),
            value: Box::new(build_expression_ir(value)?),
        }),
        ResolvedExpr::Lambda { parameters, body } => Ok(ExpressionIr::Lambda {
            parameters: parameters
                .iter()
                .map(|p| {
                    Ok(mercurio_foundation::kir::ExpressionParameter {
                        name: p.name.clone(),
                        type_ref: p.type_ref.clone(),
                        properties: p.properties.clone(),
                        default: p
                            .default
                            .as_deref()
                            .map(build_expression_ir)
                            .transpose()?
                            .map(Box::new),
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
            body: Box::new(build_expression_ir(body)?),
        }),
        ResolvedExpr::Literal(value) => Ok(ExpressionIr::Literal {
            value: value.clone(),
        }),
        ResolvedExpr::Tuple { items } => Ok(ExpressionIr::Tuple {
            items: items
                .iter()
                .map(build_expression_ir)
                .collect::<Result<Vec<_>, _>>()?,
        }),
        ResolvedExpr::SelfRef => Ok(ExpressionIr::SelfRef),
        ResolvedExpr::Unary { op, expr } => Ok(ExpressionIr::Unary {
            op: unary_expression_op(op),
            expr: Box::new(build_expression_ir(expr)?),
        }),
        ResolvedExpr::Binary { left, op, right } => Ok(ExpressionIr::Binary {
            left: Box::new(build_expression_ir(left)?),
            op: binary_expression_op(op),
            right: Box::new(build_expression_ir(right)?),
        }),
        ResolvedExpr::FeaturePath { segments } => Ok(ExpressionIr::Path {
            root: ExpressionPathRoot::SelfRef,
            segments: segments.iter().map(expression_path_segment).collect(),
        }),
        ResolvedExpr::Select { root, segments } => Ok(ExpressionIr::Select {
            root: Box::new(build_expression_ir(root)?),
            segments: segments.iter().map(expression_path_segment).collect(),
        }),
        ResolvedExpr::Call { function, args } => Ok(ExpressionIr::Call {
            function: function.clone(),
            args: args
                .iter()
                .map(build_expression_ir)
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn expression_path_segment(segment: &ResolvedPathSegment) -> ExpressionPathSegment {
    ExpressionPathSegment::Resolved {
        name: segment.name.clone(),
        feature: Some(segment.feature_id.clone()),
    }
}

fn unary_expression_op(op: &UnaryOp) -> UnaryExpressionOp {
    match op {
        UnaryOp::Negate => UnaryExpressionOp::Negate,
        UnaryOp::Not => UnaryExpressionOp::Not,
    }
}

fn binary_expression_op(op: &BinaryOp) -> BinaryExpressionOp {
    match op {
        BinaryOp::Add => BinaryExpressionOp::Add,
        BinaryOp::Subtract => BinaryExpressionOp::Subtract,
        BinaryOp::Multiply => BinaryExpressionOp::Multiply,
        BinaryOp::Divide => BinaryExpressionOp::Divide,
        BinaryOp::Power => BinaryExpressionOp::Power,
        BinaryOp::Equal => BinaryExpressionOp::Equal,
        BinaryOp::NotEqual => BinaryExpressionOp::NotEqual,
        BinaryOp::Less => BinaryExpressionOp::Less,
        BinaryOp::LessEqual => BinaryExpressionOp::LessEqual,
        BinaryOp::Greater => BinaryExpressionOp::Greater,
        BinaryOp::GreaterEqual => BinaryExpressionOp::GreaterEqual,
        BinaryOp::And => BinaryExpressionOp::And,
        BinaryOp::Or => BinaryExpressionOp::Or,
    }
}

fn usage_type_ref(
    usage: &ResolvedUsage,
    mappings: &MappingBundle,
    reference_semantics: Option<&ReferenceUsageSemantics>,
) -> Option<String> {
    if let Some(reference_semantics) = reference_semantics {
        return reference_semantics.type_refs.first().cloned();
    }

    usage
        .type_ref
        .clone()
        .or_else(|| mappings.usage_type_default(usage))
}

fn usage_subsetted_feature_refs(
    usage: &ResolvedUsage,
    mappings: &MappingBundle,
    reference_semantics: Option<&ReferenceUsageSemantics>,
) -> Vec<String> {
    if let Some(reference_semantics) = reference_semantics {
        return dedupe_refs(reference_semantics.subsetted_feature_refs.clone());
    }

    let mut subsetted_feature_refs = usage.subsetted_features.clone();
    if !subsetted_feature_refs.is_empty() {
        return dedupe_refs(subsetted_feature_refs);
    }

    if !usage.redefined_features.is_empty() {
        return Vec::new();
    }

    if let Some(default_refs) = mappings.specialized_feature_subset_default(usage) {
        return dedupe_refs(default_refs);
    }

    if mappings.usage_appends_semantic_specializations_to_subset_defaults(usage) {
        subsetted_feature_refs.extend(
            mappings.semantic_specializations_for_usage(&usage.construct, &usage.modifiers),
        );
        return dedupe_refs(subsetted_feature_refs);
    }

    let default_subset_refs = mappings.usage_subset_default(usage);
    if !default_subset_refs.is_empty() {
        subsetted_feature_refs.extend(default_subset_refs);
        return dedupe_refs(subsetted_feature_refs);
    }

    subsetted_feature_refs
        .extend(mappings.semantic_specializations_for_usage(&usage.construct, &usage.modifiers));
    dedupe_refs(subsetted_feature_refs)
}

fn usage_specialized_feature_refs(
    usage: &ResolvedUsage,
    reference_semantics: Option<&ReferenceUsageSemantics>,
) -> Vec<String> {
    let mut specialized_feature_refs = usage.specialized_features.clone();
    if let Some(reference_semantics) = reference_semantics {
        specialized_feature_refs.extend(reference_semantics.specialized_feature_refs.clone());
    }
    dedupe_refs(specialized_feature_refs)
}

fn usage_redefined_feature_refs(
    usage: &ResolvedUsage,
    reference_semantics: Option<&ReferenceUsageSemantics>,
) -> Vec<String> {
    let mut redefined_feature_refs = usage.redefined_features.clone();
    if let Some(reference_semantics) = reference_semantics {
        redefined_feature_refs.extend(reference_semantics.redefined_feature_refs.clone());
    }
    dedupe_refs(redefined_feature_refs)
}

fn set_property_refs(properties: &mut BTreeMap<String, Value>, key: &str, refs: &[String]) {
    match refs {
        [] => {
            properties.remove(key);
        }
        [only] => {
            properties.insert(key.to_string(), Value::String(only.clone()));
        }
        _ => {
            properties.insert(
                key.to_string(),
                Value::Array(refs.iter().cloned().map(Value::String).collect()),
            );
        }
    }
}

fn validate_unique_ids(elements: &[KirElement]) -> Result<(), Diagnostic> {
    let mut seen = BTreeSet::new();
    for element in elements {
        if !seen.insert(element.id.clone()) {
            return Err(Diagnostic::new(
                format!("duplicate emitted KIR id `{}`", element.id),
                None,
            ));
        }
    }
    Ok(())
}

fn disambiguate_duplicate_element_ids(elements: &mut [KirElement]) {
    let mut seen = BTreeSet::new();
    for element in elements {
        if seen.insert(element.id.clone()) {
            continue;
        }

        let base = element.id.clone();
        let suffix = element
            .properties
            .get("source_span")
            .and_then(Value::as_object)
            .and_then(|span| {
                Some(format!(
                    "{}_{}",
                    span.get("start_line")?.as_u64()?,
                    span.get("start_col")?.as_u64()?
                ))
            })
            .unwrap_or_else(|| "duplicate".to_string());
        let mut candidate = format!("{base}.{suffix}");
        let mut ordinal = 2;
        while !seen.insert(candidate.clone()) {
            candidate = format!("{base}.{suffix}_{ordinal}");
            ordinal += 1;
        }
        element.id = candidate;
    }
}

fn disambiguate_duplicate_source_position_usage_ids(elements: &mut [KirElement]) {
    let mut seen = BTreeSet::new();
    for element in elements {
        if seen.insert(element.id.clone()) {
            continue;
        }
        let disambiguate_by_source_position = element.id.ends_with(".end")
            || element.kind == "AcceptActionUsage"
            || element.id.ends_with(".AcceptActionUsage")
            || element.id.starts_with("assert.")
            || element.id.starts_with("assume.")
            || element.id.starts_with("require.")
            || element.id.starts_with("reference.")
            || element.id.starts_with("transition.")
            || (element.id.starts_with("feature.") && element.id.ends_with(".annotatedElement"));
        if !disambiguate_by_source_position {
            continue;
        }

        let base = element.id.clone();
        let suffix = element
            .properties
            .get("source_span")
            .and_then(Value::as_object)
            .and_then(|span| {
                Some(format!(
                    "{}_{}",
                    span.get("start_line")?.as_u64()?,
                    span.get("start_col")?.as_u64()?
                ))
            })
            .unwrap_or_else(|| "duplicate".to_string());
        let mut candidate = format!("{base}.{suffix}");
        let mut ordinal = 2;
        while !seen.insert(candidate.clone()) {
            candidate = format!("{base}.{suffix}_{ordinal}");
            ordinal += 1;
        }
        element.id = candidate;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kerml_mapping_uses_base_kerml_metaclasses() {
        let mappings = MappingBundle::load_for_language(SourceLanguage::Kerml).unwrap();

        assert_eq!(mappings.metaclass_for("Package").unwrap(), "KerML::Package");
        assert_eq!(mappings.metaclass_for("Import").unwrap(), "KerML::Import");
        assert_eq!(
            mappings.emission_for("KerML::Package").unwrap().kir_kind,
            "KerML::Package"
        );
        assert_eq!(
            mappings.emission_for("KerML::Classifier").unwrap().kir_kind,
            "KerML::Core::Type"
        );
    }

    #[test]
    fn sysml_mapping_overlays_kerml_base() {
        let mappings = MappingBundle::load_for_language(SourceLanguage::Sysml).unwrap();

        assert_eq!(mappings.metaclass_for("Package").unwrap(), "SysML::Package");
        assert_eq!(
            mappings.metaclass_for("Classifier").unwrap(),
            "KerML::Classifier"
        );
        assert_eq!(
            mappings.emission_for("KerML::Package").unwrap().kir_kind,
            "KerML::Package"
        );
    }
}

#[cfg(test)]
mod lowering_golden_tests {
    use super::*;

    fn span(line: usize) -> SourceSpan {
        SourceSpan {
            start_line: line,
            start_col: 1,
            end_line: line,
            end_col: 10,
        }
    }

    fn element<'a>(document: &'a KirDocument, id: &str) -> &'a KirElement {
        document
            .elements
            .iter()
            .find(|element| element.id == id)
            .unwrap_or_else(|| panic!("missing element `{id}`"))
    }

    fn lowering_metadata(element: &KirElement) -> &Map<String, Value> {
        element.properties["metadata"]["lowering"]
            .as_object()
            .expect("lowering metadata")
    }

    fn reference_usage(declared_name: &str) -> ResolvedUsage {
        ResolvedUsage {
        annotation_targets: Vec::new(),
            derived_properties: BTreeMap::new(),
            construct: "ReferenceUsage".to_string(),
            owner_construct: "Package".to_string(),
            owner_qualified_name: "root".to_string(),
            qualified_name: format!("root.{declared_name}"),
            declared_name: declared_name.to_string(),
            is_implicit_name: false,
            has_explicit_type: false,
            has_explicit_specialization: false,
            related_features: Vec::new(),
            type_ref: None,
            additional_type_refs: Vec::new(),
            reference_target: None,
            allocation_source: None,
            allocation_target: None,
            metadata_properties: BTreeMap::new(),
            multiplicity: None,
            expression: None,
            is_derived: false,
            specializes: Vec::new(),
            specialized_features: Vec::new(),
            subsetted_features: Vec::new(),
            redefined_features: Vec::new(),
            members: Vec::new(),
            modifiers: Vec::new(),
            docs: Vec::new(),
            span: span(1),
        }
    }

    fn rate_assertion_expr(feature: &str, rate_feature: &str) -> ResolvedExpr {
        ResolvedExpr::Binary {
            left: Box::new(feature_expr(feature)),
            op: BinaryOp::Equal,
            right: Box::new(ResolvedExpr::Binary {
                left: Box::new(feature_expr(feature)),
                op: BinaryOp::Add,
                right: Box::new(ResolvedExpr::Binary {
                    left: Box::new(feature_expr(rate_feature)),
                    op: BinaryOp::Multiply,
                    right: Box::new(feature_expr("duration")),
                }),
            }),
        }
    }

    fn feature_expr(name: &str) -> ResolvedExpr {
        ResolvedExpr::FeaturePath {
            segments: vec![ResolvedPathSegment {
                name: name.to_string(),
                feature_id: format!("feature.root.{name}"),
            }],
        }
    }

    #[test]
    fn package_lowering_trace_is_stable() {
        let mappings = MappingBundle::load().unwrap();
        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: vec![ResolvedPackage {
                owner_package_qualified_name: None,
                qualified_name: "Demo".to_string(),
                declared_name: "Demo".to_string(),
                modifiers: Vec::new(),
                docs: Vec::new(),
                span: span(1),
            }],
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: Vec::new(),
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let package = element(&document, "pkg.Demo");
        let lowering = lowering_metadata(package);

        assert_eq!(package.kind, "SysML::Package");
        assert_eq!(lowering["construct"], "Package");
        assert_eq!(lowering["metaclass"], "SysML::Package");
        assert_eq!(lowering["ast_node"], "PackageDecl");
    }

    #[test]
    fn connection_definition_lowering_trace_records_elaboration_rule() {
        let mappings = MappingBundle::load().unwrap();
        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: vec![ResolvedDefinition {
                visibility: "public".into(),
                declared_short_name: None,
                is_anonymous: false,
                construct: "ConnectionDefinition".to_string(),
                qualified_name: "Link".to_string(),
                declared_name: "Link".to_string(),
                is_abstract: false,
                is_variation: false,
                specializes: Vec::new(),
                members: Vec::new(),
                docs: Vec::new(),
                span: span(1),
            }],
            usages: Vec::new(),
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let definition = element(&document, "type.Link");
        let lowering = lowering_metadata(definition);

        assert_eq!(definition.kind, "SysML::Systems::ConnectionDefinition");
        assert_eq!(lowering["construct"], "ConnectionDefinition");
        assert_eq!(
            lowering["elaboration_rules"],
            json!(["connection-end-direction"])
        );
    }

    #[test]
    fn usage_family_defaults_are_profile_backed_in_kir() {
        let mappings = MappingBundle::load().unwrap();
        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![ResolvedUsage {
        annotation_targets: Vec::new(),
                derived_properties: BTreeMap::new(),
                construct: "ActionUsage".to_string(),
                owner_construct: "Package".to_string(),
                owner_qualified_name: "root".to_string(),
                qualified_name: "root.act".to_string(),
                declared_name: "act".to_string(),
                is_implicit_name: false,
                has_explicit_type: false,
                has_explicit_specialization: false,
                related_features: Vec::new(),
                type_ref: None,
                additional_type_refs: Vec::new(),
                reference_target: None,
                allocation_source: None,
                allocation_target: None,
                metadata_properties: BTreeMap::new(),
                multiplicity: None,
                expression: None,
                is_derived: false,
                specializes: Vec::new(),
                specialized_features: Vec::new(),
                subsetted_features: Vec::new(),
                redefined_features: Vec::new(),
                members: Vec::new(),
                modifiers: Vec::new(),
                docs: Vec::new(),
                span: span(1),
            }],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let usage = element(&document, "action.root.act");

        assert_eq!(usage.properties["type"], "Actions::Action");
        assert_eq!(
            usage.properties["subsetted_features"],
            json!(["Actions::actions"])
        );
        assert_eq!(usage.properties["specializes"], json!(["Actions::actions"]));
        assert_eq!(lowering_metadata(usage)["construct"], "ActionUsage");
    }

    #[test]
    fn reference_modifier_semantics_are_profile_backed() {
        let mappings = MappingBundle::load().unwrap();
        let mut usage = reference_usage("source");
        usage.modifiers.push("source-output".to_string());

        let semantics = mappings.reference_usage_semantics(&usage).unwrap();

        assert_eq!(semantics.type_refs, vec!["Ports::Port"]);
        assert_eq!(
            semantics.redefined_feature_refs,
            vec!["source", "Transfers::sourceOutput"]
        );
        assert!(semantics.semantic_specializations.is_empty());
        assert!(mappings.reference_usage_has_synthetic_declared_name(&usage));
    }

    #[test]
    fn reference_typed_semantics_are_profile_backed() {
        let mappings = MappingBundle::load().unwrap();
        let mut data_value = reference_usage("flag");
        data_value.type_ref = Some("ScalarValues::Boolean".to_string());
        let data_semantics = mappings.reference_usage_semantics(&data_value).unwrap();
        assert_eq!(
            data_semantics.subsetted_feature_refs,
            vec!["Base::dataValues"]
        );

        let mut object = reference_usage("part");
        object.type_ref = Some("Parts::Part".to_string());
        object.modifiers.push("out".to_string());
        let object_semantics = mappings.reference_usage_semantics(&object).unwrap();
        assert_eq!(
            object_semantics.subsetted_feature_refs,
            vec!["Objects::objects"]
        );
        assert_eq!(object_semantics.direction.as_deref(), Some("out"));
    }

    #[test]
    fn usage_property_defaults_are_profile_backed_in_kir() {
        let mappings = MappingBundle::load().unwrap();
        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![ResolvedUsage {
        annotation_targets: Vec::new(),
                derived_properties: BTreeMap::new(),
                construct: "PartUsage".to_string(),
                owner_construct: "ItemDefinition".to_string(),
                owner_qualified_name: "Items::Item".to_string(),
                qualified_name: "Items::Item.child".to_string(),
                declared_name: "child".to_string(),
                is_implicit_name: false,
                has_explicit_type: false,
                has_explicit_specialization: false,
                related_features: Vec::new(),
                type_ref: None,
                additional_type_refs: Vec::new(),
                reference_target: None,
                allocation_source: None,
                allocation_target: None,
                metadata_properties: BTreeMap::new(),
                multiplicity: None,
                expression: None,
                is_derived: false,
                specializes: Vec::new(),
                specialized_features: Vec::new(),
                subsetted_features: Vec::new(),
                redefined_features: Vec::new(),
                members: Vec::new(),
                modifiers: Vec::new(),
                docs: Vec::new(),
                span: span(1),
            }],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let usage = element(&document, "feature.Items::Item.child");

        assert_eq!(usage.properties["type"], "Parts::Part");
        assert_eq!(usage.properties["definition"], "Parts::Part");
    }

    #[test]
    fn explicit_part_usage_definition_preserves_family_default() {
        let mappings = MappingBundle::load().unwrap();
        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![ResolvedUsage {
        annotation_targets: Vec::new(),
                derived_properties: BTreeMap::new(),
                construct: "PartUsage".to_string(),
                owner_construct: "ItemDefinition".to_string(),
                owner_qualified_name: "ItemTest.B".to_string(),
                qualified_name: "ItemTest.B.a".to_string(),
                declared_name: "a".to_string(),
                is_implicit_name: false,
                has_explicit_type: true,
                has_explicit_specialization: true,
                related_features: Vec::new(),
                type_ref: Some("type.ItemTest.A".to_string()),
                additional_type_refs: Vec::new(),
                reference_target: None,
                allocation_source: None,
                allocation_target: None,
                metadata_properties: BTreeMap::new(),
                multiplicity: None,
                expression: None,
                is_derived: false,
                specializes: Vec::new(),
                specialized_features: Vec::new(),
                subsetted_features: Vec::new(),
                redefined_features: Vec::new(),
                members: Vec::new(),
                modifiers: Vec::new(),
                docs: Vec::new(),
                span: span(11),
            }],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let usage = element(&document, "feature.ItemTest.B.a");

        assert_eq!(
            usage.properties["definition"],
            json!(["type.ItemTest.A", "Parts::Part"])
        );
    }

    #[test]
    fn usage_property_values_are_profile_backed_in_kir() {
        let mappings = MappingBundle::load().unwrap();
        let child_state = ResolvedUsage {
        annotation_targets: Vec::new(),
            derived_properties: BTreeMap::new(),
            construct: "StateUsage".to_string(),
            owner_construct: "StateUsage".to_string(),
            owner_qualified_name: "root.parent".to_string(),
            qualified_name: "root.parent.child".to_string(),
            declared_name: "child".to_string(),
            is_implicit_name: false,
            has_explicit_type: false,
            has_explicit_specialization: false,
            related_features: Vec::new(),
            type_ref: None,
            additional_type_refs: Vec::new(),
            reference_target: None,
            allocation_source: None,
            allocation_target: None,
            metadata_properties: BTreeMap::new(),
            multiplicity: None,
            expression: None,
            is_derived: false,
            specializes: Vec::new(),
            specialized_features: Vec::new(),
            subsetted_features: Vec::new(),
            redefined_features: Vec::new(),
            members: Vec::new(),
            modifiers: Vec::new(),
            docs: Vec::new(),
            span: span(2),
        };
        let parent_state = ResolvedUsage {
        annotation_targets: Vec::new(),
            derived_properties: BTreeMap::new(),
            construct: "StateUsage".to_string(),
            owner_construct: "Package".to_string(),
            owner_qualified_name: "root".to_string(),
            qualified_name: "root.parent".to_string(),
            declared_name: "parent".to_string(),
            is_implicit_name: false,
            has_explicit_type: false,
            has_explicit_specialization: false,
            related_features: Vec::new(),
            type_ref: None,
            additional_type_refs: Vec::new(),
            reference_target: None,
            allocation_source: None,
            allocation_target: None,
            metadata_properties: BTreeMap::new(),
            multiplicity: None,
            expression: None,
            is_derived: false,
            specializes: Vec::new(),
            specialized_features: Vec::new(),
            subsetted_features: Vec::new(),
            redefined_features: Vec::new(),
            members: vec![child_state],
            modifiers: Vec::new(),
            docs: Vec::new(),
            span: span(1),
        };
        let succession = ResolvedUsage {
        annotation_targets: Vec::new(),
            derived_properties: BTreeMap::new(),
            construct: "SuccessionUsage".to_string(),
            owner_construct: "Package".to_string(),
            owner_qualified_name: "root".to_string(),
            qualified_name: "root.next".to_string(),
            declared_name: "next".to_string(),
            is_implicit_name: false,
            has_explicit_type: false,
            has_explicit_specialization: false,
            related_features: Vec::new(),
            type_ref: None,
            additional_type_refs: Vec::new(),
            reference_target: None,
            allocation_source: None,
            allocation_target: None,
            metadata_properties: BTreeMap::new(),
            multiplicity: None,
            expression: None,
            is_derived: false,
            specializes: Vec::new(),
            specialized_features: Vec::new(),
            subsetted_features: Vec::new(),
            redefined_features: Vec::new(),
            members: Vec::new(),
            modifiers: vec!["then".to_string()],
            docs: Vec::new(),
            span: span(3),
        };
        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![parent_state, succession],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let child = element(&document, "state.root.parent.child");
        let succession = element(&document, "succession.root.next.3_1");

        assert_eq!(child.properties["parent_state"], "state.root.parent");
        assert_eq!(succession.properties["target"], "state.root.next");
        assert_eq!(succession.properties["trigger_kind"], "completion");
    }

    #[test]
    fn state_do_action_rate_assertion_emits_do_behavior() {
        let mappings = MappingBundle::load().unwrap();
        let mut assertion = reference_usage("thermalEquation");
        assertion.construct = "AssertUsage".to_string();
        assertion.qualified_name = "root.Heating.integrate.thermalEquation".to_string();
        assertion.expression = Some(rate_assertion_expr("temperature", "heatRate"));

        let mut action = reference_usage("integrate");
        action.construct = "ActionUsage".to_string();
        action.owner_construct = "StateUsage".to_string();
        action.owner_qualified_name = "root.Heating".to_string();
        action.qualified_name = "root.Heating.integrate".to_string();
        action.modifiers = vec!["do".to_string()];
        action.members = vec![assertion];

        let mut state = reference_usage("Heating");
        state.construct = "StateUsage".to_string();
        state.qualified_name = "root.Heating".to_string();
        state.members = vec![action];

        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![state],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let state = element(&document, "state.root.Heating");

        assert_eq!(
            state.properties["do_behavior"],
            json!({
                "kind": "rate_integration",
                "rates": [
                    { "feature": "temperature", "rate_feature": "heatRate" }
                ]
            })
        );
    }

    #[test]
    fn accept_and_allocation_properties_are_profile_backed_in_kir() {
        let mappings = MappingBundle::load().unwrap();
        let mut accept = reference_usage("acceptA");
        accept.construct = "AcceptActionUsage".to_string();
        accept.qualified_name = "root.acceptA".to_string();
        accept.modifiers = vec![
            "trigger=go".to_string(),
            "trigger_kind=signal".to_string(),
            "transition_target=done".to_string(),
        ];

        let mut allocation = reference_usage("allocA");
        allocation.construct = "AllocationUsage".to_string();
        allocation.qualified_name = "root.allocA".to_string();
        allocation.allocation_source = Some("feature.source".to_string());
        allocation.allocation_target = Some("feature.target".to_string());

        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![accept, allocation],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let accept = element(&document, "accept.root.acceptA.1_1");
        let allocation = element(&document, "allocation.root.allocA");

        assert_eq!(accept.properties["source"], document.elements.iter().find(|e| e.kind == "SysML::Namespace").unwrap().id);
        assert_eq!(accept.properties["trigger"], "go");
        assert_eq!(accept.properties["trigger_kind"], "signal");
        assert_eq!(accept.properties["target"], "state.root.done");
        assert_eq!(allocation.properties["allocated"], "feature.source");
        assert_eq!(allocation.properties["source"], "feature.source");
        assert_eq!(allocation.properties["allocated_to"], "feature.target");
        assert_eq!(allocation.properties["target"], "feature.target");
    }

    #[test]
    fn trace_relationship_properties_are_profile_backed_in_kir() {
        let mappings = MappingBundle::load().unwrap();
        let mut satisfy = reference_usage("satA");
        satisfy.construct = "SatisfyUsage".to_string();
        satisfy.qualified_name = "root.satA".to_string();
        satisfy.reference_target = Some("requirement.reqA".to_string());

        let mut verify = reference_usage("verA");
        verify.construct = "VerifyUsage".to_string();
        verify.qualified_name = "root.verA".to_string();
        verify.reference_target = Some("requirement.reqB".to_string());

        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![satisfy, verify],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let satisfy = element(&document, "satisfy.root.satA");
        let verify = element(&document, "verify.root.verA");

        assert_eq!(satisfy.kind, "SysML::Requirements::SatisfyRequirementUsage");
        assert_eq!(satisfy.properties["source"], document.elements.iter().find(|e| e.kind == "SysML::Namespace").unwrap().id);
        assert_eq!(satisfy.properties["target"], "requirement.reqA");
        assert_eq!(verify.kind, "SysML::Requirements::VerifyRequirementUsage");
        assert_eq!(verify.properties["source"], document.elements.iter().find(|e| e.kind == "SysML::Namespace").unwrap().id);
        assert_eq!(verify.properties["target"], "requirement.reqB");
    }

    #[test]
    fn comment_properties_are_profile_backed_in_kir() {
        let mappings = MappingBundle::load().unwrap();
        let mut comment = reference_usage("note");
        comment.construct = "CommentUsage".to_string();
        comment.qualified_name = "root.note".to_string();
        comment.reference_target = Some("part.root.target".to_string());
        comment
            .metadata_properties
            .insert("body".to_string(), "review this".to_string());
        comment
            .metadata_properties
            .insert("locale".to_string(), "en-US".to_string());

        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![comment],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let comment = element(&document, "comment.root.note.1.1");

        assert_eq!(comment.properties["body"], "review this");
        assert_eq!(comment.properties["locale"], "en-US");
        assert_eq!(comment.properties["annotatedElement"], "part.root.target");
    }

    #[test]
    fn perform_action_materialized_specialization_policy_is_profile_backed() {
        let mappings = MappingBundle::load().unwrap();
        let mut perform = reference_usage("doIt");
        perform.construct = "PerformActionUsage".to_string();
        perform.qualified_name = "root.doIt".to_string();
        perform.specialized_features = vec!["feature.root.action".to_string()];

        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![perform],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let perform = element(&document, "perform.root.doIt");
        let specializes = perform.properties["specializes"]
            .as_array()
            .expect("specializes refs")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();

        assert_eq!(specializes.first().copied(), Some("kerml.Feature"));
        assert!(!specializes.contains(&"feature.root.action"));
    }

    #[test]
    fn semantic_actions_attach_metadata_in_kir() {
        let mappings = MappingBundle::load().unwrap();
        let package = ResolvedPackage {
            owner_package_qualified_name: None,
            qualified_name: "root".to_string(),
            declared_name: "root".to_string(),
            modifiers: Vec::new(),
            docs: Vec::new(),
            span: span(1),
        };
        let mut state = reference_usage("targetState");
        state.construct = "StateUsage".to_string();
        state.qualified_name = "root.targetState".to_string();

        let mut explicit_metadata = reference_usage("review");
        explicit_metadata.construct = "MetadataUsage".to_string();
        explicit_metadata.qualified_name = "root.review".to_string();
        explicit_metadata.reference_target = Some("state.root.targetState".to_string());
        explicit_metadata
            .metadata_properties
            .insert("status".to_string(), "approved".to_string());

        let mut owner_metadata = reference_usage("ownerNote");
        owner_metadata.construct = "MetadataUsage".to_string();
        owner_metadata.qualified_name = "root.ownerNote".to_string();
        owner_metadata
            .metadata_properties
            .insert("level".to_string(), "package".to_string());

        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: vec![package],
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![state, explicit_metadata, owner_metadata],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let state = element(&document, "state.root.targetState");
        let package = element(&document, "pkg.root");

        assert_eq!(
            state.properties["metadata"]["review"]["properties"]["status"],
            "approved"
        );
        assert_eq!(
            package.properties["metadata"]["ownerNote"]["properties"]["level"],
            "package"
        );
    }

    #[test]
    fn semantic_actions_source_accept_from_previous_state_in_kir() {
        let mappings = MappingBundle::load().unwrap();
        let mut state = reference_usage("ready");
        state.construct = "StateUsage".to_string();
        state.owner_construct = "ActionUsage".to_string();
        state.owner_qualified_name = "root.act".to_string();
        state.qualified_name = "root.act.ready".to_string();

        let mut accept = reference_usage("acceptAfterReady");
        accept.construct = "AcceptActionUsage".to_string();
        accept.owner_construct = "ActionUsage".to_string();
        accept.owner_qualified_name = "root.act".to_string();
        accept.qualified_name = "root.act.acceptAfterReady".to_string();

        let mut action = reference_usage("act");
        action.construct = "ActionUsage".to_string();
        action.qualified_name = "root.act".to_string();
        action.members = vec![state, accept];

        let module = ResolvedModule {
            aliases: Vec::new(),
            packages: Vec::new(),
            imports: Vec::new(),
            definitions: Vec::new(),
            usages: vec![action],
        };

        let document = transpile_module(&module, "golden.sysml", mappings).unwrap();
        let accept = element(&document, "accept.root.act.acceptAfterReady.1_1");

        assert_eq!(accept.properties["source"], "state.root.act.ready");
    }
}

#[derive(Deserialize)]
struct KermlMappingOverlay {
    constructs: MetamodelConstructSeed,
    emission: KirEmissionSeed,
}

#[derive(Deserialize)]
struct ObservedDefinitionDefaults {
    definitions: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct ConnectionDefinitionDefaults {
    definitions: BTreeMap<String, ConnectionDefinitionSelectors>,
}

#[derive(Deserialize)]
struct ConnectionDefinitionSelectors {
    base: String,
    binary: String,
}
