use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use mercurio_core::frontend::lowering::pilot_evidence::PilotLoweringEvidence;
use mercurio_core::frontend::lowering::rules::{
    LoweringAstPattern, LoweringCollectRule, LoweringEmitRule, LoweringPilotSources, LoweringRule,
    LoweringRuleSeed, has_runtime_collect_expression, has_runtime_elaboration_hook,
};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let constructs = read_json(&args.constructs)?;
    let emission = read_json(&args.emission)?;
    let semantic_defaults = args
        .semantic_defaults
        .as_deref()
        .map(read_json)
        .transpose()?;
    let lowering_rules = args.rules.as_deref().map(read_lowering_rules).transpose()?;

    let construct_names = construct_names(&constructs);
    let construct_metaclasses = construct_metaclasses(&constructs);
    let emission_metaclasses = emission_metaclasses(&emission);
    let emission_properties = emission_properties(&emission);
    let missing_emission = construct_metaclasses
        .difference(&emission_metaclasses)
        .cloned()
        .collect::<Vec<_>>();
    let unused_emission = emission_metaclasses
        .difference(&construct_metaclasses)
        .cloned()
        .collect::<Vec<_>>();

    println!("Lowering audit");
    println!("  constructs: {}", construct_metaclasses.len());
    println!("  emission rules: {}", emission_metaclasses.len());
    println!("  missing emission rules: {}", missing_emission.len());
    println!(
        "  emission rules without construct evidence: {}",
        unused_emission.len()
    );

    if !missing_emission.is_empty() {
        println!();
        println!("Missing emission rules:");
        for metaclass in &missing_emission {
            println!("  {metaclass}");
        }
    }

    if !unused_emission.is_empty() {
        println!();
        println!("Emission rules without construct evidence:");
        for metaclass in &unused_emission {
            println!("  {metaclass}");
        }
    }

    if let Some(lowering_rules) = &lowering_rules {
        let rule_constructs = lowering_rule_constructs(lowering_rules);
        let rule_metaclasses = lowering_rule_metaclasses(lowering_rules);
        let rule_status_counts = lowering_rule_status_counts(lowering_rules);
        let reviewed_rule_count = rule_status_counts.get("reviewed").copied().unwrap_or(0);
        let constructs_missing_rules = construct_metaclasses
            .difference(&rule_metaclasses)
            .cloned()
            .collect::<Vec<_>>();
        let rules_missing_construct = rule_metaclasses
            .difference(&construct_metaclasses)
            .cloned()
            .collect::<Vec<_>>();
        let rules_missing_emission = rule_metaclasses
            .difference(&emission_metaclasses)
            .cloned()
            .collect::<Vec<_>>();
        let rule_id_template_gaps = lowering_rule_id_template_gaps(lowering_rules, &emission);
        let rule_property_gaps = lowering_rule_property_gaps(lowering_rules, &emission_properties);
        let emission_property_gaps =
            emission_property_gaps(lowering_rules, &emission_properties, &construct_metaclasses);
        let collect_expression_count = lowering_collect_expression_count(lowering_rules);
        let unsupported_collect_expressions = unsupported_collect_expressions(lowering_rules);
        let elaboration_rule_count = lowering_elaboration_rule_count(lowering_rules);
        let unimplemented_elaboration_rules = unimplemented_elaboration_rules(lowering_rules);

        println!();
        println!("Declarative lowering rules");
        println!("  rules: {}", lowering_rules.rules.len());
        println!("  constructs covered: {}", rule_constructs.len());
        for (status, count) in &rule_status_counts {
            println!("  {status} rules: {count}");
        }
        println!(
            "  construct mappings without declarative rules: {}",
            constructs_missing_rules.len()
        );
        println!(
            "  rule metaclasses missing construct mappings: {}",
            rules_missing_construct.len()
        );
        println!(
            "  rule metaclasses missing emission rules: {}",
            rules_missing_emission.len()
        );
        println!(
            "  rule id templates different from emission templates: {}",
            rule_id_template_gaps.len()
        );
        println!(
            "  rule properties missing emission properties: {}",
            rule_property_gaps.len()
        );
        println!(
            "  emission properties missing declarative rule properties: {}",
            emission_property_gaps.len()
        );
        println!("  collect expressions: {collect_expression_count}");
        println!(
            "  collect expressions without runtime support: {}",
            unsupported_collect_expressions.len()
        );
        println!("  elaboration rules: {elaboration_rule_count}");
        println!(
            "  elaboration rules without runtime hook: {}",
            unimplemented_elaboration_rules.len()
        );

        if !rules_missing_construct.is_empty() {
            println!();
            println!("Rule metaclasses missing construct mappings:");
            for metaclass in &rules_missing_construct {
                println!("  {metaclass}");
            }
        }

        if !rules_missing_emission.is_empty() {
            println!();
            println!("Rule metaclasses missing emission rules:");
            for metaclass in &rules_missing_emission {
                println!("  {metaclass}");
            }
        }

        if !rule_id_template_gaps.is_empty() {
            println!();
            println!("Rule id templates different from emission templates:");
            for gap in &rule_id_template_gaps {
                println!(
                    "  {}: rule=`{}` emission=`{}`",
                    gap.metaclass, gap.rule_template, gap.emission_template
                );
            }
        }

        if !rule_property_gaps.is_empty() {
            println!();
            println!("Rule properties missing emission properties:");
            for gap in &rule_property_gaps {
                println!("  {}.{}", gap.metaclass, gap.property);
            }
        }

        if !emission_property_gaps.is_empty() && args.verbose_rules {
            println!();
            println!("Emission properties missing declarative rule properties:");
            for gap in &emission_property_gaps {
                println!("  {}.{}", gap.metaclass, gap.property);
            }
        }

        if !unsupported_collect_expressions.is_empty() {
            println!();
            println!("Collect expressions without runtime support:");
            for gap in &unsupported_collect_expressions {
                println!("  {}.{}: {}", gap.construct, gap.slot, gap.expression);
            }
        }

        if !unimplemented_elaboration_rules.is_empty() {
            println!();
            println!("Elaboration rules without runtime hook:");
            for gap in &unimplemented_elaboration_rules {
                println!("  {}: {}", gap.construct, gap.rule_id);
            }
        }

        if !constructs_missing_rules.is_empty() && args.verbose_rules {
            println!();
            println!("Construct mappings without declarative rules:");
            for metaclass in &constructs_missing_rules {
                println!("  {metaclass}");
            }
        }

        if reviewed_rule_count < args.min_reviewed_rules {
            return Err(format!(
                "reviewed lowering rule count {reviewed_rule_count} is below required minimum {}",
                args.min_reviewed_rules
            )
            .into());
        }
    }

    if let Some(semantic_defaults) = &semantic_defaults {
        let reference_usage_modifier_rules =
            semantic_default_reference_usage_modifier_rules(semantic_defaults);
        let definition_context_construct_refs =
            semantic_default_definition_context_construct_refs(semantic_defaults);
        let usage_context_construct_refs =
            semantic_default_usage_context_construct_refs(semantic_defaults);
        let usage_type_defaults = semantic_default_usage_type_defaults(semantic_defaults);
        let usage_property_defaults = semantic_default_usage_property_defaults(semantic_defaults);
        let usage_property_default_rule_count =
            semantic_default_usage_property_default_rule_count(semantic_defaults);
        let usage_action_defaults = semantic_default_usage_actions(semantic_defaults);
        let usage_action_rule_count = semantic_default_usage_action_rule_count(semantic_defaults);
        let unsupported_usage_actions =
            unsupported_semantic_default_usage_actions(semantic_defaults);
        let usage_specialization_policies =
            semantic_default_usage_specialization_policies(semantic_defaults);
        let unsupported_usage_specialization_policies =
            unsupported_semantic_default_usage_specialization_policies(semantic_defaults);
        let usage_resolution_policies =
            semantic_default_usage_resolution_policies(semantic_defaults);
        let unsupported_usage_resolution_policies =
            unsupported_semantic_default_usage_resolution_policies(semantic_defaults);
        let usage_traversal_policies = semantic_default_usage_traversal_policies(semantic_defaults);
        let usage_id_policies = semantic_default_usage_id_policies(semantic_defaults);
        let definition_companion_policies =
            semantic_default_definition_companion_policies(semantic_defaults);
        let usage_subset_defaults = semantic_default_usage_subset_defaults(semantic_defaults);
        let usage_family_defaults = semantic_default_usage_family_defaults(semantic_defaults);
        let unknown_usage_type_defaults = usage_type_defaults
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_usage_property_defaults = usage_property_defaults
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_usage_action_defaults = usage_action_defaults
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_usage_specialization_policies = usage_specialization_policies
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_usage_resolution_policies = usage_resolution_policies
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_usage_traversal_policies = usage_traversal_policies
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_usage_id_policies = usage_id_policies
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_definition_companion_policies = definition_companion_policies
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_usage_subset_defaults = usage_subset_defaults
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_usage_defaults = usage_family_defaults
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_owner_overrides =
            semantic_default_owner_override_gaps(semantic_defaults, &construct_names);
        let unsupported_placeholders = unsupported_semantic_default_placeholders(semantic_defaults);
        let unknown_usage_context_refs = usage_context_construct_refs
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let unknown_definition_context_refs = definition_context_construct_refs
            .difference(&construct_names)
            .cloned()
            .collect::<Vec<_>>();
        let semantic_defaults_without_lowering_rules = lowering_rules
            .as_ref()
            .map(|rules| {
                let rule_constructs = lowering_rule_constructs(rules);
                semantic_default_construct_refs(semantic_defaults)
                    .difference(&rule_constructs)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        println!();
        println!("Semantic defaults");
        println!(
            "  reference usage modifier rules: {}",
            reference_usage_modifier_rules
        );
        println!(
            "  definition context construct refs: {}",
            definition_context_construct_refs.len()
        );
        println!(
            "  definition context refs without construct mappings: {}",
            unknown_definition_context_refs.len()
        );
        println!(
            "  usage context construct refs: {}",
            usage_context_construct_refs.len()
        );
        println!(
            "  usage context refs without construct mappings: {}",
            unknown_usage_context_refs.len()
        );
        println!("  usage type defaults: {}", usage_type_defaults.len());
        println!(
            "  usage type defaults without construct mappings: {}",
            unknown_usage_type_defaults.len()
        );
        println!(
            "  usage property defaults: {}",
            usage_property_defaults.len()
        );
        println!("  usage property default rules: {usage_property_default_rule_count}");
        println!(
            "  usage property defaults without construct mappings: {}",
            unknown_usage_property_defaults.len()
        );
        println!("  usage actions: {}", usage_action_defaults.len());
        println!("  usage action rules: {usage_action_rule_count}");
        println!(
            "  usage actions without construct mappings: {}",
            unknown_usage_action_defaults.len()
        );
        println!(
            "  unsupported usage actions: {}",
            unsupported_usage_actions.len()
        );
        println!(
            "  usage specialization policies: {}",
            usage_specialization_policies.len()
        );
        println!(
            "  usage specialization policies without construct mappings: {}",
            unknown_usage_specialization_policies.len()
        );
        println!(
            "  unsupported usage specialization policies: {}",
            unsupported_usage_specialization_policies.len()
        );
        println!(
            "  usage resolution policies: {}",
            usage_resolution_policies.len()
        );
        println!(
            "  usage resolution policies without construct mappings: {}",
            unknown_usage_resolution_policies.len()
        );
        println!(
            "  unsupported usage resolution policies: {}",
            unsupported_usage_resolution_policies.len()
        );
        println!(
            "  usage traversal policies: {}",
            usage_traversal_policies.len()
        );
        println!(
            "  usage traversal policies without construct mappings: {}",
            unknown_usage_traversal_policies.len()
        );
        println!("  usage id policies: {}", usage_id_policies.len());
        println!(
            "  usage id policies without construct mappings: {}",
            unknown_usage_id_policies.len()
        );
        println!(
            "  definition companion policies: {}",
            definition_companion_policies.len()
        );
        println!(
            "  definition companion policies without construct mappings: {}",
            unknown_definition_companion_policies.len()
        );
        println!("  usage subset defaults: {}", usage_subset_defaults.len());
        println!(
            "  usage subset defaults without construct mappings: {}",
            unknown_usage_subset_defaults.len()
        );
        println!("  usage family defaults: {}", usage_family_defaults.len());
        println!(
            "  usage family defaults without construct mappings: {}",
            unknown_usage_defaults.len()
        );
        println!(
            "  owner overrides without construct mappings: {}",
            unknown_owner_overrides.len()
        );
        println!(
            "  unsupported placeholder tokens: {}",
            unsupported_placeholders.len()
        );
        if lowering_rules.is_some() {
            println!(
                "  semantic default constructs without declarative lowering rules: {}",
                semantic_defaults_without_lowering_rules.len()
            );
        }

        if !unknown_usage_defaults.is_empty() {
            println!();
            println!("Usage family defaults without construct mappings:");
            for construct in &unknown_usage_defaults {
                println!("  {construct}");
            }
        }

        if !unknown_usage_type_defaults.is_empty() {
            println!();
            println!("Usage type defaults without construct mappings:");
            for construct in &unknown_usage_type_defaults {
                println!("  {construct}");
            }
        }

        if !unknown_usage_property_defaults.is_empty() {
            println!();
            println!("Usage property defaults without construct mappings:");
            for construct in &unknown_usage_property_defaults {
                println!("  {construct}");
            }
        }

        if !unknown_usage_action_defaults.is_empty() {
            println!();
            println!("Usage actions without construct mappings:");
            for construct in &unknown_usage_action_defaults {
                println!("  {construct}");
            }
        }

        if !unsupported_usage_actions.is_empty() {
            println!();
            println!("Unsupported usage actions:");
            for gap in &unsupported_usage_actions {
                println!("  {}: {}", gap.construct, gap.action);
            }
        }

        if !unknown_usage_specialization_policies.is_empty() {
            println!();
            println!("Usage specialization policies without construct mappings:");
            for construct in &unknown_usage_specialization_policies {
                println!("  {construct}");
            }
        }

        if !unsupported_usage_specialization_policies.is_empty() {
            println!();
            println!("Unsupported usage specialization policies:");
            for gap in &unsupported_usage_specialization_policies {
                println!("  {}: {}", gap.construct, gap.policy);
            }
        }

        if !unknown_usage_resolution_policies.is_empty() {
            println!();
            println!("Usage resolution policies without construct mappings:");
            for construct in &unknown_usage_resolution_policies {
                println!("  {construct}");
            }
        }

        if !unsupported_usage_resolution_policies.is_empty() {
            println!();
            println!("Unsupported usage resolution policies:");
            for gap in &unsupported_usage_resolution_policies {
                println!("  {}: {}", gap.construct, gap.policy);
            }
        }

        if !unknown_usage_traversal_policies.is_empty() {
            println!();
            println!("Usage traversal policies without construct mappings:");
            for construct in &unknown_usage_traversal_policies {
                println!("  {construct}");
            }
        }

        if !unknown_usage_id_policies.is_empty() {
            println!();
            println!("Usage id policies without construct mappings:");
            for construct in &unknown_usage_id_policies {
                println!("  {construct}");
            }
        }

        if !unknown_definition_companion_policies.is_empty() {
            println!();
            println!("Definition companion policies without construct mappings:");
            for construct in &unknown_definition_companion_policies {
                println!("  {construct}");
            }
        }

        if !unknown_usage_subset_defaults.is_empty() {
            println!();
            println!("Usage subset defaults without construct mappings:");
            for construct in &unknown_usage_subset_defaults {
                println!("  {construct}");
            }
        }

        if !unknown_usage_context_refs.is_empty() {
            println!();
            println!("Usage context refs without construct mappings:");
            for construct in &unknown_usage_context_refs {
                println!("  {construct}");
            }
        }

        if !unknown_definition_context_refs.is_empty() {
            println!();
            println!("Definition context refs without construct mappings:");
            for construct in &unknown_definition_context_refs {
                println!("  {construct}");
            }
        }

        if !unknown_owner_overrides.is_empty() {
            println!();
            println!("Owner overrides without construct mappings:");
            for gap in &unknown_owner_overrides {
                println!("  {} -> {}", gap.construct, gap.owner_construct);
            }
        }

        if !unsupported_placeholders.is_empty() {
            println!();
            println!("Unsupported semantic default placeholder tokens:");
            for gap in &unsupported_placeholders {
                println!("  {}: {}", gap.path, gap.placeholder);
            }
        }

        if !semantic_defaults_without_lowering_rules.is_empty() && args.verbose_rules {
            println!();
            println!("Semantic default constructs without declarative lowering rules:");
            for construct in &semantic_defaults_without_lowering_rules {
                println!("  {construct}");
            }
        }
    }

    let hardcoded_lowering_policies = hardcoded_lowering_policy_burndown();
    let hardcoded_lowering_policy_counts =
        hardcoded_lowering_policy_counts(&hardcoded_lowering_policies);
    println!();
    println!("Hard-coded lowering policy burndown");
    println!(
        "  remaining policies: {}",
        hardcoded_lowering_policies.len()
    );
    for (category, count) in &hardcoded_lowering_policy_counts {
        println!("  {category}: {count}");
    }
    if args.verbose_rules {
        println!();
        println!("Hard-coded lowering policies:");
        for policy in &hardcoded_lowering_policies {
            println!(
                "  {} [{}] {} -> {}",
                policy.construct, policy.category, policy.location, policy.extraction
            );
        }
    }

    if let Some(evidence_path) = args.evidence.as_deref() {
        let evidence = read_pilot_evidence(evidence_path)?;
        let grammar_returns = grammar_returns(&evidence);
        let ecore_classes = ecore_classes(&evidence);
        let grammar_metaclasses = grammar_returns.values().cloned().collect::<BTreeSet<_>>();
        let grammar_missing_emission = grammar_metaclasses
            .difference(&emission_metaclasses)
            .cloned()
            .collect::<Vec<_>>();
        let evidence_missing_construct = grammar_metaclasses
            .difference(&construct_metaclasses)
            .cloned()
            .collect::<Vec<_>>();
        let ecore_missing_emission = ecore_classes
            .difference(&emission_metaclasses)
            .cloned()
            .collect::<Vec<_>>();

        let ecore_feature_gaps = ecore_feature_gaps(&evidence, &emission_properties);
        let rule_evidence_gaps = lowering_rules
            .as_ref()
            .map(|rules| lowering_rule_evidence_gaps(rules, &evidence))
            .unwrap_or_default();
        let grammar_missing_lowering_rules = lowering_rules
            .as_ref()
            .map(|rules| {
                let rule_metaclasses = lowering_rule_metaclasses(rules);
                grammar_metaclasses
                    .difference(&rule_metaclasses)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let actual_transform_observation_count = evidence.transform_observations.len();
        println!();
        println!("Pilot evidence");
        println!("  grammar rules: {}", grammar_returns.len());
        println!("  ecore classes: {}", ecore_classes.len());
        println!("  transform observations: {actual_transform_observation_count}");
        println!(
            "  grammar returns missing emission rules: {}",
            grammar_missing_emission.len()
        );
        println!(
            "  grammar returns missing construct mappings: {}",
            evidence_missing_construct.len()
        );
        println!(
            "  ecore classes missing emission rules: {}",
            ecore_missing_emission.len()
        );
        println!(
            "  ecore exact-name features missing emission properties: {}",
            ecore_feature_gaps.len()
        );
        if lowering_rules.is_some() {
            println!(
                "  grammar returns missing declarative lowering rules: {}",
                grammar_missing_lowering_rules.len()
            );
            println!(
                "  lowering rule pilot sources missing evidence: {}",
                rule_evidence_gaps.len()
            );
        }

        if !grammar_missing_emission.is_empty() {
            println!();
            println!("Grammar returns missing emission rules:");
            for metaclass in &grammar_missing_emission {
                println!("  {metaclass}");
            }
        }

        if !evidence_missing_construct.is_empty() {
            println!();
            println!("Grammar returns missing construct mappings:");
            for metaclass in &evidence_missing_construct {
                println!("  {metaclass}");
            }
        }

        if !ecore_missing_emission.is_empty() {
            println!();
            println!("Ecore classes missing emission rules:");
            for metaclass in &ecore_missing_emission {
                println!("  {metaclass}");
            }
        }

        if !ecore_feature_gaps.is_empty() {
            println!();
            println!("Ecore exact-name features missing emission properties:");
            for gap in ecore_feature_gaps.iter().take(50) {
                println!("  {}.{}", gap.metaclass, gap.feature);
            }
            if ecore_feature_gaps.len() > 50 {
                println!("  ... {} more", ecore_feature_gaps.len() - 50);
            }
        }

        if !rule_evidence_gaps.is_empty() {
            println!();
            println!("Lowering rule pilot sources missing evidence:");
            for gap in &rule_evidence_gaps {
                println!("  {}: {}", gap.construct, gap.source);
            }
        }

        if !grammar_missing_lowering_rules.is_empty() && args.verbose_rules {
            println!();
            println!("Grammar returns missing declarative lowering rules:");
            for metaclass in &grammar_missing_lowering_rules {
                println!("  {metaclass}");
            }
        }
    }

    if let Some(output_path) = args.write_rule_draft.as_deref() {
        let draft = generate_lowering_rule_draft(lowering_rules.as_ref(), &constructs, &emission);
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(output_path, serde_json::to_string_pretty(&draft)?)?;
        println!();
        println!(
            "Wrote declarative lowering rule draft: {} rules -> {}",
            draft.rules.len(),
            output_path.display()
        );
    }

    Ok(())
}

struct Args {
    constructs: PathBuf,
    emission: PathBuf,
    semantic_defaults: Option<PathBuf>,
    rules: Option<PathBuf>,
    verbose_rules: bool,
    write_rule_draft: Option<PathBuf>,
    min_reviewed_rules: usize,
    evidence: Option<PathBuf>,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut constructs = PathBuf::from(
            "resources/metamodels/sysml-2.0-metamodel-0.57.0/mappings/metamodel_constructs.seed.json",
        );
        let mut emission = PathBuf::from(
            "resources/metamodels/sysml-2.0-metamodel-0.57.0/mappings/kir_emission.seed.json",
        );
        let mut rules = Some(PathBuf::from(
            "resources/metamodels/sysml-2.0-metamodel-0.57.0/mappings/lowering_rules.seed.json",
        ));
        let mut semantic_defaults = Some(PathBuf::from(
            "resources/metamodels/sysml-2.0-metamodel-0.57.0/mappings/semantic_defaults.seed.json",
        ));
        let mut verbose_rules = false;
        let mut write_rule_draft = None;
        let mut min_reviewed_rules = 0usize;
        let mut evidence = None;
        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--constructs" => {
                    index += 1;
                    constructs =
                        PathBuf::from(args.get(index).ok_or("missing --constructs value")?);
                }
                "--emission" => {
                    index += 1;
                    emission = PathBuf::from(args.get(index).ok_or("missing --emission value")?);
                }
                "--evidence" => {
                    index += 1;
                    evidence = Some(PathBuf::from(
                        args.get(index).ok_or("missing --evidence value")?,
                    ));
                }
                "--rules" => {
                    index += 1;
                    rules = Some(PathBuf::from(
                        args.get(index).ok_or("missing --rules value")?,
                    ));
                }
                "--semantic-defaults" => {
                    index += 1;
                    semantic_defaults = Some(PathBuf::from(
                        args.get(index).ok_or("missing --semantic-defaults value")?,
                    ));
                }
                "--no-semantic-defaults" => {
                    semantic_defaults = None;
                }
                "--no-rules" => {
                    rules = None;
                }
                "--verbose-rules" => {
                    verbose_rules = true;
                }
                "--write-rule-draft" => {
                    index += 1;
                    write_rule_draft = Some(PathBuf::from(
                        args.get(index).ok_or("missing --write-rule-draft value")?,
                    ));
                }
                "--min-reviewed-rules" => {
                    index += 1;
                    min_reviewed_rules = args
                        .get(index)
                        .ok_or("missing --min-reviewed-rules value")?
                        .parse()?;
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
            index += 1;
        }

        Ok(Self {
            constructs,
            emission,
            semantic_defaults,
            rules,
            verbose_rules,
            write_rule_draft,
            min_reviewed_rules,
            evidence,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --bin audit_lowering -- [--constructs PATH] [--emission PATH] [--rules PATH|--no-rules] [--semantic-defaults PATH|--no-semantic-defaults] [--verbose-rules] [--write-rule-draft PATH] [--min-reviewed-rules N] [--evidence PATH]"
    );
}

fn read_json(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let input = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&input)?)
}

fn read_pilot_evidence(path: &Path) -> Result<PilotLoweringEvidence, Box<dyn std::error::Error>> {
    let input = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&input)?)
}

fn read_lowering_rules(path: &Path) -> Result<LoweringRuleSeed, Box<dyn std::error::Error>> {
    let input = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&input)?)
}

fn construct_metaclasses(document: &Value) -> BTreeSet<String> {
    document
        .get("constructs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("metaclass"))
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn construct_names(document: &Value) -> BTreeSet<String> {
    document
        .get("constructs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("construct"))
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn emission_metaclasses(document: &Value) -> BTreeSet<String> {
    document
        .get("metaclasses")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|metaclasses| metaclasses.keys())
        .cloned()
        .collect()
}

fn emission_properties(document: &Value) -> BTreeMap<String, BTreeSet<String>> {
    document
        .get("metaclasses")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|metaclasses| metaclasses.iter())
        .map(|(metaclass, rule)| {
            let properties = rule
                .get("emit")
                .and_then(|emit| emit.get("properties"))
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|properties| properties.keys())
                .cloned()
                .collect::<BTreeSet<_>>();
            (metaclass.clone(), properties)
        })
        .collect()
}

fn construct_entries(document: &Value) -> Vec<(String, String)> {
    document
        .get("constructs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let construct = entry.get("construct")?.as_str()?.to_string();
            let metaclass = entry.get("metaclass")?.as_str()?.to_string();
            Some((construct, metaclass))
        })
        .collect()
}

fn construct_keywords(document: &Value) -> BTreeMap<String, String> {
    let mut keywords = BTreeMap::new();
    for registry_name in ["definitions", "usages"] {
        let Some(registry) = document
            .get("keyword_registry")
            .and_then(|registry| registry.get(registry_name))
            .and_then(Value::as_object)
        else {
            continue;
        };
        for (keyword, construct) in registry {
            if let Some(construct) = construct.as_str() {
                keywords
                    .entry(construct.to_string())
                    .or_insert_with(|| keyword.clone());
            }
        }
    }
    keywords
}

fn emission_id_template(document: &Value, metaclass: &str) -> Option<String> {
    document
        .get("metaclasses")?
        .get(metaclass)?
        .get("id_template")?
        .as_str()
        .map(str::to_string)
}

fn generate_lowering_rule_draft(
    existing: Option<&LoweringRuleSeed>,
    constructs: &Value,
    emission: &Value,
) -> LoweringRuleSeed {
    let mut draft = existing.cloned().unwrap_or_else(|| LoweringRuleSeed {
        schema_version: 1,
        source: BTreeMap::new(),
        rules: Vec::new(),
    });
    draft.source.insert(
        "kind".to_string(),
        json!("mercurio-generated-lowering-rule-draft"),
    );
    draft.source.insert(
        "note".to_string(),
        json!("Generated by audit_lowering --write-rule-draft from construct and emission seeds. Review before promoting."),
    );

    let mut seen_metaclasses = lowering_rule_metaclasses(&draft);
    let keywords = construct_keywords(constructs);
    let emission_properties = emission_properties(emission);

    for (construct, metaclass) in construct_entries(constructs) {
        if seen_metaclasses.contains(&metaclass) {
            continue;
        }
        let Some(properties) = emission_properties.get(&metaclass) else {
            continue;
        };
        let Some(id_template) = emission_id_template(emission, &metaclass) else {
            continue;
        };
        draft.rules.push(generated_lowering_rule(
            &construct,
            &metaclass,
            keywords.get(&construct).cloned(),
            id_template,
            properties,
        ));
        seen_metaclasses.insert(metaclass);
    }

    draft.rules.sort_by(|left, right| {
        left.metaclass
            .cmp(&right.metaclass)
            .then_with(|| left.construct.cmp(&right.construct))
    });
    draft
}

fn generated_lowering_rule(
    construct: &str,
    metaclass: &str,
    keyword: Option<String>,
    id_template: String,
    properties: &BTreeSet<String>,
) -> LoweringRule {
    let element = infer_collect_element(construct, metaclass);
    LoweringRule {
        construct: construct.to_string(),
        metaclass: metaclass.to_string(),
        ast: LoweringAstPattern {
            node: infer_ast_node(&element).to_string(),
            keyword,
        },
        status: Some("generated-draft".to_string()),
        collect: LoweringCollectRule {
            element,
            name: "$ast.name".to_string(),
            owner: "$scope.owner".to_string(),
            fields: inferred_collect_fields(construct),
        },
        elaborate: Vec::new(),
        emit: LoweringEmitRule {
            id_template,
            properties: properties
                .iter()
                .map(|property| (property.clone(), lowering_property_value(property)))
                .collect(),
        },
        pilot_sources: LoweringPilotSources {
            grammar_rules: vec![construct.to_string()],
            ecore_class: Some(metaclass.to_string()),
            transform_observations: Vec::new(),
        },
    }
}

fn lowering_property_value(property: &str) -> String {
    match property {
        "declared_name" => "$declared_name",
        "name" => "$name",
        "owner" => "$owner_id",
        "type" => "$type_ref",
        "featuring_type" => "$featuring_type_ref",
        "direction" => "$direction",
        "members" | "member_ids" => "$member_ids",
        "features" | "owned_feature_ids" => "$owned_feature_ids",
        "specializes" => "$specializes_refs",
        "specialized_features" => "$specialized_feature_refs",
        "subsetted_features" => "$subsetted_feature_refs",
        "redefined_features" => "$redefined_feature_refs",
        "metatype" => "$metatype_ref",
        "is_abstract" => "$is_abstract",
        "is_derived" => "$is_derived",
        "is_end" => "$is_end",
        "is_ordered" => "$is_ordered",
        "is_unique" => "$is_unique",
        "is_variable" => "$is_variable",
        other => return format!("${other}"),
    }
    .to_string()
}

fn infer_collect_element(construct: &str, metaclass: &str) -> String {
    if construct == "Package" || metaclass.ends_with("::Package") {
        "package".to_string()
    } else if construct.contains("Import") || metaclass.contains("Import") {
        "import".to_string()
    } else if construct.ends_with("Definition")
        || construct.ends_with("Classifier")
        || metaclass.ends_with("Definition")
    {
        "definition".to_string()
    } else {
        "usage".to_string()
    }
}

fn infer_ast_node(element: &str) -> &'static str {
    match element {
        "package" => "PackageDecl",
        "import" => "ImportDecl",
        "definition" => "GenericDefinitionDecl",
        _ => "GenericUsageDecl",
    }
}

fn inferred_collect_fields(construct: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    if construct.ends_with("Definition") {
        fields.insert(
            "is_abstract".to_string(),
            "$ast.modifiers contains abstract".to_string(),
        );
        fields.insert("members".to_string(), "$ast.members[usage]".to_string());
        fields.insert(
            "specializes".to_string(),
            "$ast.specializes or semantic_default".to_string(),
        );
    } else if construct.ends_with("Usage") {
        fields.insert(
            "members".to_string(),
            "$ast.body_members[usage]".to_string(),
        );
        fields.insert(
            "specializes".to_string(),
            "$ast.specializes or semantic_default".to_string(),
        );
        fields.insert("type".to_string(), "$ast.ty".to_string());
    }
    fields
}

fn grammar_returns(document: &PilotLoweringEvidence) -> BTreeMap<String, String> {
    document
        .grammar_rules
        .iter()
        .map(|rule| (rule.rule.clone(), rule.returns.clone()))
        .collect()
}

fn ecore_classes(document: &PilotLoweringEvidence) -> BTreeSet<String> {
    document
        .ecore_classes
        .iter()
        .map(|class| format!("{}::{}", class.package, class.name))
        .collect()
}

fn lowering_rule_constructs(document: &LoweringRuleSeed) -> BTreeSet<String> {
    document
        .rules
        .iter()
        .map(|rule| rule.construct.clone())
        .collect()
}

fn lowering_rule_metaclasses(document: &LoweringRuleSeed) -> BTreeSet<String> {
    document
        .rules
        .iter()
        .map(|rule| rule.metaclass.clone())
        .collect()
}

fn lowering_rule_status_counts(document: &LoweringRuleSeed) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for rule in &document.rules {
        let status = rule.status.as_deref().unwrap_or("unspecified").to_string();
        *counts.entry(status).or_insert(0) += 1;
    }
    counts
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RulePropertyGap {
    metaclass: String,
    property: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CollectExpressionGap {
    construct: String,
    slot: String,
    expression: String,
}

fn lowering_collect_expression_count(rules: &LoweringRuleSeed) -> usize {
    rules
        .rules
        .iter()
        .map(|rule| 3 + rule.collect.fields.len())
        .sum()
}

fn unsupported_collect_expressions(rules: &LoweringRuleSeed) -> Vec<CollectExpressionGap> {
    let mut gaps = Vec::new();
    for rule in &rules.rules {
        for (slot, expression) in collect_rule_expressions(rule) {
            if !has_runtime_collect_expression(&expression) {
                gaps.push(CollectExpressionGap {
                    construct: rule.construct.clone(),
                    slot,
                    expression,
                });
            }
        }
    }
    gaps.sort();
    gaps
}

fn collect_rule_expressions(rule: &LoweringRule) -> Vec<(String, String)> {
    let mut expressions = vec![
        ("element".to_string(), rule.collect.element.clone()),
        ("name".to_string(), rule.collect.name.clone()),
        ("owner".to_string(), rule.collect.owner.clone()),
    ];
    expressions.extend(
        rule.collect
            .fields
            .iter()
            .map(|(field, expression)| (format!("fields.{field}"), expression.clone())),
    );
    expressions
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ElaborationHookGap {
    construct: String,
    rule_id: String,
}

fn lowering_elaboration_rule_count(rules: &LoweringRuleSeed) -> usize {
    rules.rules.iter().map(|rule| rule.elaborate.len()).sum()
}

fn unimplemented_elaboration_rules(rules: &LoweringRuleSeed) -> Vec<ElaborationHookGap> {
    let mut gaps = Vec::new();
    for rule in &rules.rules {
        for step in &rule.elaborate {
            if !has_runtime_elaboration_hook(&step.id) {
                gaps.push(ElaborationHookGap {
                    construct: rule.construct.clone(),
                    rule_id: step.id.clone(),
                });
            }
        }
    }
    gaps.sort();
    gaps
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RuleIdTemplateGap {
    metaclass: String,
    rule_template: String,
    emission_template: String,
}

fn lowering_rule_id_template_gaps(
    rules: &LoweringRuleSeed,
    emission: &Value,
) -> Vec<RuleIdTemplateGap> {
    let mut gaps = Vec::new();
    for rule in &rules.rules {
        let Some(emission_template) = emission_id_template(emission, &rule.metaclass) else {
            continue;
        };
        if rule.emit.id_template != emission_template {
            gaps.push(RuleIdTemplateGap {
                metaclass: rule.metaclass.clone(),
                rule_template: rule.emit.id_template.clone(),
                emission_template,
            });
        }
    }
    gaps.sort();
    gaps
}

fn lowering_rule_property_gaps(
    rules: &LoweringRuleSeed,
    emission_properties: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<RulePropertyGap> {
    let mut gaps = Vec::new();
    for rule in &rules.rules {
        let Some(properties) = emission_properties.get(&rule.metaclass) else {
            continue;
        };
        for property in rule.emit.properties.keys() {
            if !properties.contains(property) {
                gaps.push(RulePropertyGap {
                    metaclass: rule.metaclass.clone(),
                    property: property.clone(),
                });
            }
        }
    }
    gaps.sort();
    gaps
}

fn emission_property_gaps(
    rules: &LoweringRuleSeed,
    emission_properties: &BTreeMap<String, BTreeSet<String>>,
    construct_metaclasses: &BTreeSet<String>,
) -> Vec<RulePropertyGap> {
    let rule_properties = rules
        .rules
        .iter()
        .map(|rule| {
            (
                rule.metaclass.clone(),
                rule.emit
                    .properties
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut gaps = Vec::new();
    for metaclass in construct_metaclasses {
        let Some(emitted_properties) = emission_properties.get(metaclass) else {
            continue;
        };
        let Some(rule_properties) = rule_properties.get(metaclass) else {
            continue;
        };
        for property in emitted_properties {
            if !rule_properties.contains(property) {
                gaps.push(RulePropertyGap {
                    metaclass: metaclass.clone(),
                    property: property.clone(),
                });
            }
        }
    }
    gaps.sort();
    gaps
}

fn semantic_default_usage_family_defaults(document: &Value) -> BTreeSet<String> {
    document
        .get("usage_family_defaults")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|defaults| defaults.keys())
        .cloned()
        .collect()
}

fn semantic_default_reference_usage_modifier_rules(document: &Value) -> usize {
    document
        .get("reference_usage_semantics")
        .and_then(|semantics| semantics.get("modifier_rules"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or_default()
}

fn semantic_default_definition_context_construct_refs(document: &Value) -> BTreeSet<String> {
    document
        .get("definition_context")
        .and_then(|context| context.get("abstract_constructs"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn semantic_default_usage_context_construct_refs(document: &Value) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    let Some(context) = document.get("usage_context").and_then(Value::as_object) else {
        return refs;
    };
    for key in [
        "non_variable_owner_constructs",
        "no_type_context_owner_constructs",
        "non_owned_member_constructs",
    ] {
        refs.extend(
            context
                .get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string),
        );
    }
    refs
}

fn semantic_default_usage_type_defaults(document: &Value) -> BTreeSet<String> {
    document
        .get("usage_type_defaults")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|defaults| defaults.keys())
        .cloned()
        .collect()
}

fn semantic_default_usage_property_defaults(document: &Value) -> BTreeSet<String> {
    document
        .get("usage_property_defaults")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|defaults| defaults.keys())
        .cloned()
        .collect()
}

fn semantic_default_usage_property_default_rule_count(document: &Value) -> usize {
    document
        .get("usage_property_defaults")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|defaults| defaults.values())
        .map(|rules| rules.as_array().map(Vec::len).unwrap_or_default())
        .sum()
}

fn semantic_default_usage_actions(document: &Value) -> BTreeSet<String> {
    document
        .get("usage_actions")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|actions| actions.keys())
        .cloned()
        .collect()
}

fn semantic_default_usage_action_rule_count(document: &Value) -> usize {
    document
        .get("usage_actions")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|actions| actions.values())
        .map(|rules| rules.as_array().map(Vec::len).unwrap_or_default())
        .sum()
}

fn semantic_default_usage_specialization_policies(document: &Value) -> BTreeSet<String> {
    document
        .get("usage_specialization_policies")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|policies| policies.keys())
        .cloned()
        .collect()
}

fn semantic_default_usage_resolution_policies(document: &Value) -> BTreeSet<String> {
    document
        .get("usage_resolution_policies")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|policies| policies.keys())
        .cloned()
        .collect()
}

fn semantic_default_usage_traversal_policies(document: &Value) -> BTreeSet<String> {
    document
        .get("usage_traversal_policies")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|policies| policies.keys())
        .cloned()
        .collect()
}

fn semantic_default_usage_id_policies(document: &Value) -> BTreeSet<String> {
    document
        .get("usage_id_policies")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|policies| policies.keys())
        .cloned()
        .collect()
}

fn semantic_default_definition_companion_policies(document: &Value) -> BTreeSet<String> {
    document
        .get("definition_companion_policies")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|policies| policies.keys())
        .cloned()
        .collect()
}

fn semantic_default_construct_refs(document: &Value) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    refs.extend(semantic_default_definition_context_construct_refs(document));
    refs.extend(semantic_default_usage_context_construct_refs(document));
    refs.extend(semantic_default_usage_type_defaults(document));
    refs.extend(semantic_default_usage_property_defaults(document));
    refs.extend(semantic_default_usage_actions(document));
    refs.extend(semantic_default_usage_specialization_policies(document));
    refs.extend(semantic_default_usage_resolution_policies(document));
    refs.extend(semantic_default_usage_traversal_policies(document));
    refs.extend(semantic_default_usage_id_policies(document));
    refs.extend(semantic_default_definition_companion_policies(document));
    refs.extend(semantic_default_usage_subset_defaults(document));
    refs.extend(semantic_default_usage_family_defaults(document));
    refs.extend(semantic_default_owner_override_refs(document));
    refs
}

fn semantic_default_usage_subset_defaults(document: &Value) -> BTreeSet<String> {
    document
        .get("usage_subset_defaults")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|defaults| defaults.keys())
        .cloned()
        .collect()
}

fn semantic_default_owner_override_refs(document: &Value) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    collect_owner_override_refs(
        document,
        "usage_family_defaults",
        "owner_subsetted_feature_refs",
        &mut refs,
    );
    collect_owner_override_refs(
        document,
        "usage_type_defaults",
        "owner_type_refs",
        &mut refs,
    );
    collect_usage_property_default_owner_refs(document, &mut refs);
    collect_owner_override_refs(
        document,
        "usage_subset_defaults",
        "owner_subsetted_feature_refs",
        &mut refs,
    );
    collect_owner_override_refs(
        document,
        "usage_subset_defaults",
        "specialized_feature_subset.owner_append_refs",
        &mut refs,
    );
    collect_nested_owner_override_refs(
        document,
        "usage_subset_defaults",
        "modifier_owner_subsetted_feature_refs",
        &mut refs,
    );
    refs
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OwnerOverrideGap {
    construct: String,
    owner_construct: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PlaceholderGap {
    path: String,
    placeholder: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct UsageActionGap {
    construct: String,
    action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct UsageSpecializationPolicyGap {
    construct: String,
    policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct UsageResolutionPolicyGap {
    construct: String,
    policy: String,
}

fn unsupported_semantic_default_usage_actions(document: &Value) -> Vec<UsageActionGap> {
    let mut gaps = Vec::new();
    let Some(actions) = document.get("usage_actions").and_then(Value::as_object) else {
        return gaps;
    };
    for (construct, rules) in actions {
        let Some(rules) = rules.as_array() else {
            continue;
        };
        for rule in rules {
            let Some(action) = rule.get("action").and_then(Value::as_str) else {
                continue;
            };
            if !is_supported_semantic_default_usage_action(action) {
                gaps.push(UsageActionGap {
                    construct: construct.clone(),
                    action: action.to_string(),
                });
            }
        }
    }
    gaps.sort();
    gaps
}

fn is_supported_semantic_default_usage_action(action: &str) -> bool {
    matches!(
        action,
        "attach_metadata_application" | "source_from_previous_sibling_state"
    )
}

fn unsupported_semantic_default_usage_specialization_policies(
    document: &Value,
) -> Vec<UsageSpecializationPolicyGap> {
    let mut gaps = Vec::new();
    let Some(policies) = document
        .get("usage_specialization_policies")
        .and_then(Value::as_object)
    else {
        return gaps;
    };
    for (construct, policy) in policies {
        for key in ["specialization_refs_policy", "materialized_refs_policy"] {
            let Some(policy) = policy.get(key).and_then(Value::as_str) else {
                continue;
            };
            if !is_supported_semantic_default_usage_specialization_policy(policy) {
                gaps.push(UsageSpecializationPolicyGap {
                    construct: construct.clone(),
                    policy: policy.to_string(),
                });
            }
        }
    }
    gaps.sort();
    gaps
}

fn is_supported_semantic_default_usage_specialization_policy(policy: &str) -> bool {
    matches!(
        policy,
        "merge_feature_refs_into_semantic_specializations"
            | "prepend_feature_for_specialized_actions_without_multiplicity"
            | "suppress_feature_refs_for_explicit_type_specialized_features_without_redefinitions"
    )
}

fn unsupported_semantic_default_usage_resolution_policies(
    document: &Value,
) -> Vec<UsageResolutionPolicyGap> {
    let mut gaps = Vec::new();
    let Some(policies) = document
        .get("usage_resolution_policies")
        .and_then(Value::as_object)
    else {
        return gaps;
    };
    for (construct, policy) in policies {
        for key in [
            "reference_target_policy",
            "connection_end_specialization_policy",
        ] {
            let Some(policy) = policy.get(key).and_then(Value::as_str) else {
                continue;
            };
            if !is_supported_semantic_default_usage_resolution_policy(policy) {
                gaps.push(UsageResolutionPolicyGap {
                    construct: construct.clone(),
                    policy: policy.to_string(),
                });
            }
        }
    }
    gaps.sort();
    gaps
}

fn is_supported_semantic_default_usage_resolution_policy(policy: &str) -> bool {
    matches!(
        policy,
        "annotation_target_then_type_then_reference"
            | "from_parent_connection_type_member"
            | "type_then_reference"
    )
}

fn unsupported_semantic_default_placeholders(document: &Value) -> Vec<PlaceholderGap> {
    let mut gaps = Vec::new();
    collect_unsupported_placeholders(document, "$", &mut gaps);
    gaps.sort();
    gaps.dedup();
    gaps
}

fn collect_unsupported_placeholders(value: &Value, path: &str, gaps: &mut Vec<PlaceholderGap>) {
    match value {
        Value::String(text) => {
            for placeholder in semantic_default_placeholders(text) {
                if !is_supported_semantic_default_placeholder(&placeholder) {
                    gaps.push(PlaceholderGap {
                        path: path.to_string(),
                        placeholder,
                    });
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_unsupported_placeholders(item, &format!("{path}[{index}]"), gaps);
            }
        }
        Value::Object(entries) => {
            for (key, item) in entries {
                collect_unsupported_placeholders(item, &format!("{path}.{key}"), gaps);
            }
        }
        _ => {}
    }
}

fn semantic_default_placeholders(value: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let chars = value.char_indices().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        let (offset, ch) = chars[index];
        if ch != '$' {
            index += 1;
            continue;
        }
        let start = offset;
        let mut end = offset + ch.len_utf8();
        index += 1;
        while index < chars.len() {
            let (next_offset, next_ch) = chars[index];
            if !(next_ch == '_' || next_ch.is_ascii_alphanumeric()) {
                break;
            }
            end = next_offset + next_ch.len_utf8();
            index += 1;
        }
        placeholders.push(value[start..end].to_string());
    }
    placeholders
}

fn is_supported_semantic_default_placeholder(placeholder: &str) -> bool {
    matches!(
        placeholder,
        "$allocation_source"
            | "$allocation_target"
            | "$declared_name"
            | "$metadata_body"
            | "$metadata_locale"
            | "$modifier_value_trigger"
            | "$modifier_value_trigger_kind"
            | "$owner_id"
            | "$owner_qualified_name"
            | "$qualified_name"
            | "$reference_target"
            | "$reference_target_or_owner"
            | "$sibling_state_id_transition_target"
    )
}

fn semantic_default_owner_override_gaps(
    document: &Value,
    construct_names: &BTreeSet<String>,
) -> Vec<OwnerOverrideGap> {
    let mut gaps = Vec::new();
    collect_owner_override_gaps(
        document,
        "usage_family_defaults",
        "owner_subsetted_feature_refs",
        construct_names,
        &mut gaps,
    );
    collect_owner_override_gaps(
        document,
        "usage_type_defaults",
        "owner_type_refs",
        construct_names,
        &mut gaps,
    );
    collect_usage_property_default_owner_gaps(document, construct_names, &mut gaps);
    collect_owner_override_gaps(
        document,
        "usage_subset_defaults",
        "owner_subsetted_feature_refs",
        construct_names,
        &mut gaps,
    );
    collect_owner_override_gaps(
        document,
        "usage_subset_defaults",
        "specialized_feature_subset.owner_append_refs",
        construct_names,
        &mut gaps,
    );
    collect_nested_owner_override_gaps(
        document,
        "usage_subset_defaults",
        "modifier_owner_subsetted_feature_refs",
        construct_names,
        &mut gaps,
    );
    gaps.sort();
    gaps
}

fn collect_usage_property_default_owner_gaps(
    document: &Value,
    construct_names: &BTreeSet<String>,
    gaps: &mut Vec<OwnerOverrideGap>,
) {
    let Some(defaults) = document
        .get("usage_property_defaults")
        .and_then(Value::as_object)
    else {
        return;
    };
    for (construct, rules) in defaults {
        let Some(rules) = rules.as_array() else {
            continue;
        };
        for rule in rules {
            let Some(owner_construct) = rule.get("owner_construct").and_then(Value::as_str) else {
                continue;
            };
            if !construct_names.contains(owner_construct) {
                gaps.push(OwnerOverrideGap {
                    construct: construct.clone(),
                    owner_construct: owner_construct.to_string(),
                });
            }
        }
    }
}

fn collect_usage_property_default_owner_refs(document: &Value, refs: &mut BTreeSet<String>) {
    let Some(defaults) = document
        .get("usage_property_defaults")
        .and_then(Value::as_object)
    else {
        return;
    };
    for rules in defaults.values() {
        let Some(rules) = rules.as_array() else {
            continue;
        };
        refs.extend(
            rules
                .iter()
                .filter_map(|rule| rule.get("owner_construct"))
                .filter_map(Value::as_str)
                .map(str::to_string),
        );
    }
}

fn collect_owner_override_refs(
    document: &Value,
    section: &str,
    owner_key: &str,
    refs: &mut BTreeSet<String>,
) {
    let Some(defaults) = document.get(section).and_then(Value::as_object) else {
        return;
    };
    for default in defaults.values() {
        let Some(overrides) = nested_value(default, owner_key).and_then(Value::as_object) else {
            continue;
        };
        refs.extend(overrides.keys().cloned());
    }
}

fn collect_nested_owner_override_refs(
    document: &Value,
    section: &str,
    owner_key: &str,
    refs: &mut BTreeSet<String>,
) {
    let Some(defaults) = document.get(section).and_then(Value::as_object) else {
        return;
    };
    for default in defaults.values() {
        let Some(modifier_overrides) = nested_value(default, owner_key).and_then(Value::as_object)
        else {
            continue;
        };
        for owner_defaults in modifier_overrides.values() {
            let Some(owner_defaults) = owner_defaults.as_object() else {
                continue;
            };
            refs.extend(owner_defaults.keys().cloned());
        }
    }
}

fn collect_owner_override_gaps(
    document: &Value,
    section: &str,
    owner_key: &str,
    construct_names: &BTreeSet<String>,
    gaps: &mut Vec<OwnerOverrideGap>,
) {
    let Some(defaults) = document.get(section).and_then(Value::as_object) else {
        return;
    };
    for (construct, default) in defaults {
        let Some(overrides) = nested_value(default, owner_key).and_then(Value::as_object) else {
            continue;
        };
        for owner_construct in overrides.keys() {
            if !construct_names.contains(owner_construct) {
                gaps.push(OwnerOverrideGap {
                    construct: construct.clone(),
                    owner_construct: owner_construct.clone(),
                });
            }
        }
    }
}

fn collect_nested_owner_override_gaps(
    document: &Value,
    section: &str,
    owner_key: &str,
    construct_names: &BTreeSet<String>,
    gaps: &mut Vec<OwnerOverrideGap>,
) {
    let Some(defaults) = document.get(section).and_then(Value::as_object) else {
        return;
    };
    for (construct, default) in defaults {
        let Some(modifier_overrides) = nested_value(default, owner_key).and_then(Value::as_object)
        else {
            continue;
        };
        for owner_defaults in modifier_overrides.values() {
            let Some(owner_defaults) = owner_defaults.as_object() else {
                continue;
            };
            for owner_construct in owner_defaults.keys() {
                if !construct_names.contains(owner_construct) {
                    gaps.push(OwnerOverrideGap {
                        construct: construct.clone(),
                        owner_construct: owner_construct.clone(),
                    });
                }
            }
        }
    }
}

fn nested_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(value, |current, segment| current.get(segment))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HardcodedLoweringPolicy {
    construct: &'static str,
    category: &'static str,
    location: &'static str,
    extraction: &'static str,
}

fn hardcoded_lowering_policy_burndown() -> Vec<HardcodedLoweringPolicy> {
    Vec::new()
}

fn hardcoded_lowering_policy_counts(
    policies: &[HardcodedLoweringPolicy],
) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for policy in policies {
        *counts.entry(policy.category).or_default() += 1;
    }
    counts
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RuleEvidenceGap {
    construct: String,
    source: String,
}

fn lowering_rule_evidence_gaps(
    rules: &LoweringRuleSeed,
    evidence: &PilotLoweringEvidence,
) -> Vec<RuleEvidenceGap> {
    let grammar_rules = evidence
        .grammar_rules
        .iter()
        .map(|rule| rule.rule.clone())
        .collect::<BTreeSet<_>>();
    let ecore_classes = ecore_classes(evidence);
    let transform_observations = evidence
        .transform_observations
        .iter()
        .map(|observation| observation.construct.clone())
        .collect::<BTreeSet<_>>();

    let mut gaps = Vec::new();
    for rule in &rules.rules {
        for grammar_rule in &rule.pilot_sources.grammar_rules {
            if !grammar_rules.contains(grammar_rule) {
                gaps.push(RuleEvidenceGap {
                    construct: rule.construct.clone(),
                    source: format!("grammar:{grammar_rule}"),
                });
            }
        }
        if let Some(ecore_class) = &rule.pilot_sources.ecore_class
            && !ecore_classes.contains(ecore_class)
        {
            gaps.push(RuleEvidenceGap {
                construct: rule.construct.clone(),
                source: format!("ecore:{ecore_class}"),
            });
        }
        for observation in &rule.pilot_sources.transform_observations {
            if !transform_observations.contains(observation) {
                gaps.push(RuleEvidenceGap {
                    construct: rule.construct.clone(),
                    source: format!("transform:{observation}"),
                });
            }
        }
    }

    gaps.sort();
    gaps
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EcoreFeatureGap {
    metaclass: String,
    feature: String,
}

fn ecore_feature_gaps(
    evidence: &PilotLoweringEvidence,
    emission_properties: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<EcoreFeatureGap> {
    let mut gaps = Vec::new();

    for class in &evidence.ecore_classes {
        let metaclass = format!("{}::{}", class.package, class.name);
        let Some(properties) = emission_properties.get(&metaclass) else {
            continue;
        };

        for feature in &class.structural_features {
            if feature.derived || feature.transient || feature.volatile {
                continue;
            }
            if properties.contains(&feature.name) {
                continue;
            }
            gaps.push(EcoreFeatureGap {
                metaclass: metaclass.clone(),
                feature: feature.name.clone(),
            });
        }
    }

    gaps.sort();
    gaps
}
