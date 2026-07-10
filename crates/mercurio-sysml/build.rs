use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use mercurio_core::{KirDocument, KirFieldKind, generate_rust_stdlib_consts};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RegistryEntry {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GeneratedFieldSpecs {
    fields: Vec<GeneratedFieldSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct MetamodelConstructSeed {
    keyword_registry: KeywordRegistrySeed,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct GrammarExtract {
    rule_call_graph: Vec<GrammarRuleCall>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct GrammarRuleCall {
    rule: String,
    calls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct MetamodelExtract {
    generalizations: Vec<MetamodelGeneralization>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct MetamodelGeneralization {
    specific: String,
    general: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct KeywordRegistrySeed {
    definitions: BTreeMap<String, String>,
    usages: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct GeneratedFieldSpec {
    field: String,
    kind: String,
}

#[derive(Debug, Deserialize)]
struct FieldSpecOverlay {
    entries: Vec<FieldSpecOverlayEntry>,
}

#[derive(Debug, Deserialize)]
struct FieldSpecOverlayEntry {
    field: String,
    kind: String,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let resources = manifest_dir.join("../../resources");
    let registry_path = resources.join("metamodels/registry.json");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    println!("cargo:rerun-if-changed={}", registry_path.display());
    println!(
        "cargo:rerun-if-changed={}",
        resources.join("kernel").display()
    );
    let registry_text =
        fs::read_to_string(&registry_path).expect("failed to read SysML metamodel registry");
    let metamodels: Vec<RegistryEntry> =
        serde_json::from_str(&registry_text).expect("failed to parse SysML metamodel registry");

    let mut runtime_field_specs = None;
    let mut runtime_vocabulary = None;
    let mut runtime_generalizations = None;
    let mut runtime_containment_mismatches = None;
    for metamodel in metamodels {
        println!(
            "cargo:rerun-if-changed={}",
            resources.join("metamodels").join(&metamodel.id).display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            resources
                .join("metamodels")
                .join(&metamodel.id)
                .join("mappings/field_specs.generated.json")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            resources
                .join("metamodels")
                .join(&metamodel.id)
                .join("mappings/field_specs.overlay.json")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            resources
                .join("metamodels")
                .join(&metamodel.id)
                .join("mappings/metamodel_constructs.seed.json")
                .display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            resources
                .join("metamodels")
                .join(&metamodel.id)
                .join("metamodel.extract.json")
                .display()
        );
        let field_specs = load_generated_field_specs(&resources, &metamodel.id);
        match &runtime_field_specs {
            Some(existing) if existing != &field_specs => {
                panic!(
                    "generated field specs for `{}` differ from the first metamodel registry entry",
                    metamodel.id
                );
            }
            None => runtime_field_specs = Some(field_specs.clone()),
            _ => {}
        }

        let vocabulary = load_writable_vocabulary(&resources, &metamodel.id);
        match &runtime_vocabulary {
            Some(existing) if existing != &vocabulary => {
                panic!(
                    "generated writable vocabulary for `{}` differs from the first metamodel registry entry",
                    metamodel.id
                );
            }
            None => runtime_vocabulary = Some(vocabulary.clone()),
            _ => {}
        }

        let containment_mismatches =
            load_grammar_containment_mismatches(&resources, &metamodel.id, &vocabulary);
        match &runtime_containment_mismatches {
            Some(existing) if existing != &containment_mismatches => {
                panic!(
                    "generated grammar containment mismatches for `{}` differ from the first metamodel registry entry",
                    metamodel.id
                );
            }
            None => runtime_containment_mismatches = Some(containment_mismatches.clone()),
            _ => {}
        }

        let generalizations = load_metamodel_generalizations(&resources, &metamodel.id);
        match &runtime_generalizations {
            Some(existing) if existing != &generalizations => {
                panic!(
                    "generated metamodel generalizations for `{}` differ from the first metamodel registry entry",
                    metamodel.id
                );
            }
            None => runtime_generalizations = Some(generalizations.clone()),
            _ => {}
        }

        let document = load_baseline_for_build(&resources, &metamodel.id, &field_specs);
        let rust_source = generate_rust_stdlib_consts(&document, &metamodel.id);
        let out = out_dir.join(format!("stdlib_consts_{}.rs", metamodel.id));
        fs::write(out, rust_source).expect("failed to write generated stdlib constants");
    }

    let runtime_field_specs =
        runtime_field_specs.expect("metamodel registry must contain at least one entry");
    fs::write(
        out_dir.join("sysml_field_specs.rs"),
        generate_field_specs_rust(&runtime_field_specs),
    )
    .expect("failed to write generated SysML field specs");

    let runtime_vocabulary =
        runtime_vocabulary.expect("metamodel registry must contain at least one entry");
    fs::write(
        out_dir.join("sysml_writable_vocabulary.rs"),
        generate_writable_vocabulary_rust(&runtime_vocabulary),
    )
    .expect("failed to write generated SysML writable vocabulary");

    let runtime_generalizations =
        runtime_generalizations.expect("metamodel registry must contain at least one entry");
    fs::write(
        out_dir.join("sysml_metamodel_generalizations.rs"),
        generate_metamodel_generalizations_rust(&runtime_generalizations),
    )
    .expect("failed to write generated SysML metamodel generalizations");

    let runtime_containment_mismatches =
        runtime_containment_mismatches.expect("metamodel registry must contain at least one entry");
    fs::write(
        out_dir.join("sysml_grammar_containment.rs"),
        generate_grammar_containment_rust(&runtime_containment_mismatches),
    )
    .expect("failed to write generated SysML grammar containment facts");
}

fn load_baseline_for_build(
    resources: &Path,
    metamodel_id: &str,
    field_specs: &[(String, KirFieldKind)],
) -> KirDocument {
    let kernel = KirDocument::from_path_with_registered_fields(
        &resources.join("kernel/kerml-kernel.kir.json"),
        field_specs
            .iter()
            .map(|(name, kind)| (name.as_str(), *kind)),
    )
    .expect("failed to load kernel KIR");
    let sysml = KirDocument::from_path_with_registered_fields(
        &resources
            .join("metamodels")
            .join(metamodel_id)
            .join("stdlib/sysml-library.kir.json"),
        field_specs
            .iter()
            .map(|(name, kind)| (name.as_str(), *kind)),
    )
    .expect("failed to load SysML stdlib KIR");
    KirDocument::merge([kernel, sysml]).expect("failed to merge stdlib KIR")
}

fn load_generated_field_specs(resources: &Path, metamodel_id: &str) -> Vec<(String, KirFieldKind)> {
    let generated_path = resources
        .join("metamodels")
        .join(metamodel_id)
        .join("mappings/field_specs.generated.json");
    let text = fs::read_to_string(&generated_path).unwrap_or_else(|err| {
        panic!(
            "failed to read generated SysML field specs `{}`: {err}",
            generated_path.display()
        )
    });
    let generated: GeneratedFieldSpecs = serde_json::from_str(&text).unwrap_or_else(|err| {
        panic!(
            "failed to parse generated SysML field specs `{}`: {err}",
            generated_path.display()
        )
    });
    let overlay_path = resources
        .join("metamodels")
        .join(metamodel_id)
        .join("mappings/field_specs.overlay.json");
    let overlay_text = fs::read_to_string(&overlay_path).unwrap_or_else(|err| {
        panic!(
            "failed to read SysML field spec overlay `{}`: {err}",
            overlay_path.display()
        )
    });
    let overlay: FieldSpecOverlay = serde_json::from_str(&overlay_text).unwrap_or_else(|err| {
        panic!(
            "failed to parse SysML field spec overlay `{}`: {err}",
            overlay_path.display()
        )
    });
    let mut fields = BTreeMap::new();
    for field in generated.fields {
        let kind = parse_field_kind(&field.kind).unwrap_or_else(|| {
            panic!(
                "unknown generated SysML field kind `{}` for `{}` in `{}`",
                field.kind,
                field.field,
                generated_path.display()
            )
        });
        fields.insert(field.field, kind);
    }
    for field in overlay.entries {
        let kind = parse_field_kind(&field.kind).unwrap_or_else(|| {
            panic!(
                "unknown overlay SysML field kind `{}` for `{}` in `{}`",
                field.kind,
                field.field,
                overlay_path.display()
            )
        });
        fields.insert(field.field, kind);
    }
    fields.into_iter().collect()
}

fn parse_field_kind(value: &str) -> Option<KirFieldKind> {
    match value {
        "Scalar" => Some(KirFieldKind::Scalar),
        "Reference" => Some(KirFieldKind::Reference),
        "ReferenceList" => Some(KirFieldKind::ReferenceList),
        "Expression" => Some(KirFieldKind::Expression),
        "Metadata" => Some(KirFieldKind::Metadata),
        _ => None,
    }
}

fn load_writable_vocabulary(resources: &Path, metamodel_id: &str) -> MetamodelConstructSeed {
    let seed_path = resources
        .join("metamodels")
        .join(metamodel_id)
        .join("mappings/metamodel_constructs.seed.json");
    let text = fs::read_to_string(&seed_path).unwrap_or_else(|err| {
        panic!(
            "failed to read SysML metamodel construct seed `{}`: {err}",
            seed_path.display()
        )
    });
    serde_json::from_str(&text).unwrap_or_else(|err| {
        panic!(
            "failed to parse SysML metamodel construct seed `{}`: {err}",
            seed_path.display()
        )
    })
}

fn load_grammar_containment_mismatches(
    resources: &Path,
    metamodel_id: &str,
    seed: &MetamodelConstructSeed,
) -> Vec<(String, String)> {
    let grammar_path = resources
        .join("metamodels")
        .join(metamodel_id)
        .join("grammar.extract.json");
    let text = fs::read_to_string(&grammar_path).unwrap_or_else(|err| {
        panic!(
            "failed to read SysML grammar extract `{}`: {err}",
            grammar_path.display()
        )
    });
    let grammar: GrammarExtract = serde_json::from_str(&text).unwrap_or_else(|err| {
        panic!(
            "failed to parse SysML grammar extract `{}`: {err}",
            grammar_path.display()
        )
    });

    let mut graph = BTreeMap::<String, Vec<String>>::new();
    for entry in grammar.rule_call_graph {
        let calls = graph.entry(entry.rule).or_default();
        for call in entry.calls {
            if !calls.contains(&call) {
                calls.push(call);
            }
        }
    }

    let token_rules = grammar_containment_token_rules(seed, &graph);
    let terminal_rules = token_rules.values().cloned().collect::<Vec<_>>();
    let mut mismatches = Vec::new();
    for (container, container_rule) in &token_rules {
        for (child, child_rule) in &token_rules {
            if !grammar_rule_reaches(container_rule, child_rule, &graph, &terminal_rules) {
                mismatches.push((container.clone(), child.clone()));
            }
        }
    }
    mismatches.sort();
    mismatches.dedup();
    mismatches
}

fn grammar_containment_token_rules(
    seed: &MetamodelConstructSeed,
    graph: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, String> {
    let mut tokens = BTreeMap::new();
    push_grammar_token_rule(&mut tokens, graph, "package", "Package");
    push_grammar_token_rule(&mut tokens, graph, "Package", "Package");

    for (keyword, construct) in &seed.keyword_registry.definitions {
        push_grammar_token_rule(&mut tokens, graph, keyword, construct);
        push_grammar_token_rule(&mut tokens, graph, &format!("{keyword} def"), construct);
        push_grammar_token_rule(&mut tokens, graph, construct, construct);
    }
    for (keyword, construct) in &seed.keyword_registry.usages {
        push_grammar_token_rule(&mut tokens, graph, keyword, construct);
        push_grammar_token_rule(&mut tokens, graph, construct, construct);
    }
    tokens
}

fn push_grammar_token_rule(
    tokens: &mut BTreeMap<String, String>,
    graph: &BTreeMap<String, Vec<String>>,
    token: &str,
    rule: &str,
) {
    if graph.contains_key(rule) {
        tokens.insert(token.to_string(), rule.to_string());
    }
}

fn grammar_rule_reaches(
    source: &str,
    target: &str,
    graph: &BTreeMap<String, Vec<String>>,
    terminal_rules: &[String],
) -> bool {
    let mut stack = graph.get(source).cloned().unwrap_or_default();
    let mut visited = Vec::<String>::new();
    while let Some(current) = stack.pop() {
        if current == target {
            return true;
        }
        if terminal_rules.contains(&current) {
            continue;
        }
        if visited.contains(&current) {
            continue;
        }
        visited.push(current.clone());
        if let Some(calls) = graph.get(&current) {
            stack.extend(calls.iter().cloned());
        }
    }
    false
}

fn load_metamodel_generalizations(
    resources: &Path,
    metamodel_id: &str,
) -> Vec<MetamodelGeneralization> {
    let extract_path = resources
        .join("metamodels")
        .join(metamodel_id)
        .join("metamodel.extract.json");
    let text = fs::read_to_string(&extract_path).unwrap_or_else(|err| {
        panic!(
            "failed to read SysML metamodel extract `{}`: {err}",
            extract_path.display()
        )
    });
    let mut extract: MetamodelExtract = serde_json::from_str(&text).unwrap_or_else(|err| {
        panic!(
            "failed to parse SysML metamodel extract `{}`: {err}",
            extract_path.display()
        )
    });
    extract.generalizations.sort_by(|left, right| {
        (&left.specific, &left.general).cmp(&(&right.specific, &right.general))
    });
    extract.generalizations
}

fn generate_field_specs_rust(field_specs: &[(String, KirFieldKind)]) -> String {
    let mut source = String::from(
        "pub fn sysml_field_specs() -> &'static [(&'static str, KirFieldKind)] {\n    &[\n",
    );
    for (field, kind) in field_specs {
        source.push_str(&format!(
            "        ({:?}, KirFieldKind::{}),\n",
            field,
            field_kind_variant(*kind)
        ));
    }
    source.push_str("    ]\n}\n");
    source
}

fn generate_writable_vocabulary_rust(seed: &MetamodelConstructSeed) -> String {
    let definitions = seed
        .keyword_registry
        .definitions
        .iter()
        .map(|(keyword, kind)| (keyword.as_str(), kind.as_str()))
        .collect::<Vec<_>>();
    let usages = seed
        .keyword_registry
        .usages
        .iter()
        .map(|(keyword, kind)| (keyword.as_str(), kind.as_str()))
        .collect::<Vec<_>>();
    let definition_keywords = definitions
        .iter()
        .map(|(keyword, _)| *keyword)
        .collect::<Vec<_>>();
    let usage_keywords = usages
        .iter()
        .map(|(keyword, _)| *keyword)
        .collect::<Vec<_>>();
    let usage_definition_pairs = usages
        .iter()
        .filter_map(|(usage_keyword, _)| {
            seed.keyword_registry
                .definitions
                .contains_key(*usage_keyword)
                .then_some((*usage_keyword, *usage_keyword))
        })
        .collect::<Vec<_>>();
    let relationship_keywords = usages
        .iter()
        .filter_map(|(keyword, construct)| {
            is_relationship_usage_construct(construct).then_some(*keyword)
        })
        .collect::<Vec<_>>();

    let mut source = String::new();
    source.push_str("pub fn sysml_definition_keywords() -> &'static [&'static str] {\n    &[\n");
    push_string_slice_entries(&mut source, &definition_keywords);
    source.push_str("    ]\n}\n\n");

    source.push_str("pub fn sysml_usage_keywords() -> &'static [&'static str] {\n    &[\n");
    push_string_slice_entries(&mut source, &usage_keywords);
    source.push_str("    ]\n}\n\n");

    source.push_str("pub fn sysml_relationship_keywords() -> &'static [&'static str] {\n    &[\n");
    push_string_slice_entries(&mut source, &relationship_keywords);
    source.push_str("    ]\n}\n\n");

    source.push_str(
        "pub fn sysml_definition_element_kinds() -> &'static [(&'static str, &'static str)] {\n    &[\n",
    );
    push_pair_slice_entries(&mut source, &definitions);
    source.push_str("    ]\n}\n\n");

    source.push_str(
        "pub fn sysml_usage_element_kinds() -> &'static [(&'static str, &'static str)] {\n    &[\n",
    );
    push_pair_slice_entries(&mut source, &usages);
    source.push_str("    ]\n}\n\n");

    source.push_str(
        "pub fn sysml_usage_definition_pairs() -> &'static [(&'static str, &'static str)] {\n    &[\n",
    );
    push_pair_slice_entries(&mut source, &usage_definition_pairs);
    source.push_str("    ]\n}\n");
    source
}

fn generate_metamodel_generalizations_rust(generalizations: &[MetamodelGeneralization]) -> String {
    let entries = generalizations
        .iter()
        .map(|generalization| {
            (
                generalization.specific.as_str(),
                generalization.general.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let mut source = String::from(
        "pub fn sysml_metamodel_generalizations() -> &'static [(&'static str, &'static str)] {\n    &[\n",
    );
    push_pair_slice_entries(&mut source, &entries);
    source.push_str("    ]\n}\n");
    source
}

fn generate_grammar_containment_rust(mismatches: &[(String, String)]) -> String {
    let entries = mismatches
        .iter()
        .map(|(container, child)| (container.as_str(), child.as_str()))
        .collect::<Vec<_>>();
    let mut source = String::from(
        "pub fn sysml_grammar_containment_mismatches() -> &'static [(&'static str, &'static str)] {\n    &[\n",
    );
    push_pair_slice_entries(&mut source, &entries);
    source.push_str("    ]\n}\n");
    source
}

fn push_string_slice_entries(source: &mut String, entries: &[&str]) {
    for entry in entries {
        source.push_str(&format!("        {entry:?},\n"));
    }
}

fn push_pair_slice_entries(source: &mut String, entries: &[(&str, &str)]) {
    for (left, right) in entries {
        source.push_str(&format!("        ({left:?}, {right:?}),\n"));
    }
}

fn is_relationship_usage_construct(construct: &str) -> bool {
    matches!(
        construct,
        "AllocateUsage"
            | "BindingConnectorAsUsage"
            | "ConnectionUsage"
            | "DependencyUsage"
            | "FlowUsage"
            | "SatisfyUsage"
            | "SuccessionUsage"
            | "TransitionUsage"
            | "VerifyUsage"
    )
}

fn field_kind_variant(kind: KirFieldKind) -> &'static str {
    match kind {
        KirFieldKind::Scalar => "Scalar",
        KirFieldKind::Reference => "Reference",
        KirFieldKind::ReferenceList => "ReferenceList",
        KirFieldKind::Expression => "Expression",
        KirFieldKind::Metadata => "Metadata",
    }
}
