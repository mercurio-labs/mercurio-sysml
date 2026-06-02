use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use mercurio_core::ir::{KirDocument, KirElement};
use mercurio_core::{DerivedFeatureManifest, DerivedFeatureRegistry};
use serde::Serialize;
use serde_json::Value;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(relative)
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse()?;
    let mut features = Vec::new();

    for input in &args.inputs {
        let document = KirDocument::from_path(input)?;
        for element in &document.elements {
            if is_derived_feature(element) {
                features.push(inventory_entry(input, element));
            }
        }
    }

    features.sort_by(|left, right| left.id.cmp(&right.id));
    propagate_canonical_statuses(&mut features);
    let manifest_candidates = candidate_manifest(&features);
    let validated_manifest_candidates = validate_candidate_manifest(&manifest_candidates)?;
    let unresolved = unresolved_document(&features);
    let mut summary = summarize(&features);
    summary.manifest_candidate_specs = manifest_candidates.derived_features.len();
    summary.validated_manifest_candidate_specs = validated_manifest_candidates;

    if let Some(parent) = args.inventory_out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &args.inventory_out,
        serde_json::to_string_pretty(&InventoryDocument {
            summary: summary.clone(),
            features: features.clone(),
        })? + "\n",
    )?;

    if let Some(parent) = args.manifest_out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &args.manifest_out,
        serde_json::to_string_pretty(&manifest_candidates)? + "\n",
    )?;

    if let Some(parent) = args.unresolved_out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &args.unresolved_out,
        serde_json::to_string_pretty(&unresolved)? + "\n",
    )?;

    if let Some(parent) = args.report_out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &args.report_out,
        render_report(&summary, &features, &unresolved),
    )?;

    println!(
        "wrote {} derived features to {}",
        features.len(),
        args.inventory_out.display()
    );
    println!(
        "wrote {} manifest candidate specs to {} ({} validated)",
        manifest_candidates.derived_features.len(),
        args.manifest_out.display(),
        validated_manifest_candidates
    );
    println!(
        "wrote {} unresolved derived feature specs to {}",
        unresolved.summary.no_source_anchor + unresolved.summary.multiple_source_anchors,
        args.unresolved_out.display()
    );
    println!("wrote burn-down report to {}", args.report_out.display());
    Ok(())
}

fn propagate_canonical_statuses(features: &mut [DerivedFeatureEntry]) {
    let mut canonical_status = BTreeMap::new();
    for feature in features.iter() {
        let rank = status_rank(feature.status);
        canonical_status
            .entry(feature.canonical_id.clone())
            .and_modify(|(_, existing_rank)| {
                if rank < *existing_rank {
                    *existing_rank = rank;
                }
            })
            .or_insert((feature.status, rank));
    }
    for feature in features {
        if let Some((status, _)) = canonical_status.get(&feature.canonical_id) {
            feature.status = *status;
        }
    }
}

fn status_rank(status: BurnStatus) -> u8 {
    match status {
        BurnStatus::ImplementedCore => 0,
        BurnStatus::NeedsPilotComparison => 1,
        BurnStatus::ManualAnalysis => 2,
    }
}

#[derive(Debug)]
struct Args {
    inputs: Vec<PathBuf>,
    inventory_out: PathBuf,
    manifest_out: PathBuf,
    unresolved_out: PathBuf,
    report_out: PathBuf,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut inputs = vec![
            repo_path("resources/kernel/kerml-kernel.kir.json"),
            repo_path("resources/metamodels/sysml-2.0-metamodel-0.57.0/stdlib/sysml-library.kir.json"),
        ];
        let mut inventory_out =
            repo_path("docs/development/generated/derived-feature-inventory.json");
        let mut manifest_out =
            repo_path("docs/development/generated/derived-feature-manifest-candidates.json");
        let mut unresolved_out =
            repo_path("docs/development/generated/derived-feature-unresolved.json");
        let mut report_out = repo_path("docs/development/DERIVED_FEATURE_BURNDOWN.md");

        let args = env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--input" => {
                    index += 1;
                    inputs.push(PathBuf::from(take_arg(&args, index, "--input")?));
                }
                "--only-input" => {
                    index += 1;
                    inputs = vec![PathBuf::from(take_arg(&args, index, "--only-input")?)];
                }
                "--inventory-out" => {
                    index += 1;
                    inventory_out = PathBuf::from(take_arg(&args, index, "--inventory-out")?);
                }
                "--manifest-out" => {
                    index += 1;
                    manifest_out = PathBuf::from(take_arg(&args, index, "--manifest-out")?);
                }
                "--unresolved-out" => {
                    index += 1;
                    unresolved_out = PathBuf::from(take_arg(&args, index, "--unresolved-out")?);
                }
                "--report-out" => {
                    index += 1;
                    report_out = PathBuf::from(take_arg(&args, index, "--report-out")?);
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument `{other}`").into()),
            }
            index += 1;
        }

        Ok(Self {
            inputs,
            inventory_out,
            manifest_out,
            unresolved_out,
            report_out,
        })
    }
}

fn take_arg(args: &[String], index: usize, flag: &str) -> Result<String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| format!("missing value for {flag}").into())
}

fn print_usage() {
    println!(
        "usage: derived_feature_burndown [--only-input <kir.json>] [--input <kir.json>] [--inventory-out <path>] [--manifest-out <path>] [--unresolved-out <path>] [--report-out <path>]"
    );
}

#[derive(Debug, Clone, Serialize)]
struct InventoryDocument {
    summary: InventorySummary,
    features: Vec<DerivedFeatureEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct CandidateManifest {
    metamodel: String,
    derived_features: Vec<CandidateManifestSpec>,
}

#[derive(Debug, Clone, Serialize)]
struct CandidateManifestSpec {
    owner: String,
    feature: String,
    #[serde(flatten)]
    rule: CandidateManifestRule,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CandidateManifestRule {
    SubsetChain {
        source: String,
        target_feature: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_type: Option<String>,
    },
    IntersectionSubsetChain {
        sources: Vec<String>,
        target_feature: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        target_type: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
struct UnresolvedDocument {
    summary: UnresolvedSummary,
    no_source_anchor: Vec<UnresolvedFeatureEntry>,
    multiple_source_anchors: Vec<UnresolvedFeatureEntry>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct UnresolvedSummary {
    no_source_anchor: usize,
    multiple_source_anchors: usize,
    by_rule: BTreeMap<String, usize>,
    by_owner: BTreeMap<String, usize>,
    by_resolution_family: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
struct UnresolvedFeatureEntry {
    #[serde(flatten)]
    feature: DerivedFeatureEntry,
    resolution_family: ResolutionFamily,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum ResolutionFamily {
    CoreStructuralNative,
    CoreBooleanNative,
    KernelExpressionNative,
    SysmlBooleanNative,
    SysmlTextNative,
    SysmlDomainNative,
}

impl ResolutionFamily {
    fn as_str(self) -> &'static str {
        match self {
            Self::CoreStructuralNative => "core_structural_native",
            Self::CoreBooleanNative => "core_boolean_native",
            Self::KernelExpressionNative => "kernel_expression_native",
            Self::SysmlBooleanNative => "sysml_boolean_native",
            Self::SysmlTextNative => "sysml_text_native",
            Self::SysmlDomainNative => "sysml_domain_native",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
struct InventorySummary {
    total: usize,
    unique_total: usize,
    manifest_candidate_specs: usize,
    validated_manifest_candidate_specs: usize,
    unique_by_status: BTreeMap<String, usize>,
    unique_by_rule: BTreeMap<String, usize>,
    by_status: BTreeMap<String, usize>,
    by_rule: BTreeMap<String, usize>,
    by_library: BTreeMap<String, usize>,
    by_owner: BTreeMap<String, usize>,
    subset_chain_by_anchor: BTreeMap<String, usize>,
    subset_chain_by_candidate_source_count: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
struct DerivedFeatureEntry {
    id: String,
    canonical_id: String,
    library: String,
    owner: Option<String>,
    declared_name: Option<String>,
    type_refs: Vec<String>,
    type_label: Option<String>,
    subsets: Vec<String>,
    specializes: Vec<String>,
    redefines: Vec<String>,
    source_feature: Option<String>,
    candidate_sources: Vec<String>,
    rule: RuleKind,
    status: BurnStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum RuleKind {
    InverseOwnership,
    TypedSubset,
    SubsetChain,
    Redefinition,
    NameDerivation,
    AnnotationTarget,
    LibraryBoolean,
    Manual,
}

impl RuleKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::InverseOwnership => "inverse_ownership",
            Self::TypedSubset => "typed_subset",
            Self::SubsetChain => "subset_chain",
            Self::Redefinition => "redefinition",
            Self::NameDerivation => "name_derivation",
            Self::AnnotationTarget => "annotation_target",
            Self::LibraryBoolean => "library_boolean",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum BurnStatus {
    ImplementedCore,
    NeedsPilotComparison,
    ManualAnalysis,
}

impl BurnStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::ImplementedCore => "implemented_core",
            Self::NeedsPilotComparison => "needs_pilot_comparison",
            Self::ManualAnalysis => "manual_analysis",
        }
    }
}

fn is_derived_feature(element: &KirElement) -> bool {
    element
        .properties
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("is_derived"))
        .and_then(Value::as_bool)
        == Some(true)
        || element
            .properties
            .get("is_derived")
            .and_then(Value::as_bool)
            == Some(true)
}

fn inventory_entry(path: &PathBuf, element: &KirElement) -> DerivedFeatureEntry {
    let metadata = element
        .properties
        .get("metadata")
        .and_then(Value::as_object);
    let owner = string_property(element, metadata, "owner");
    let declared_name = string_property(element, metadata, "declared_name")
        .or_else(|| element.id.rsplit("::").next().map(str::to_string));
    let type_refs = string_list_property(element, "type");
    let type_label = string_property(element, metadata, "type_label");
    let subsets = string_list_property(element, "subsets")
        .into_iter()
        .chain(string_list_property(element, "subsetted_features"))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let specializes = string_list_property(element, "specializes");
    let redefines = string_list_property(element, "redefines")
        .into_iter()
        .chain(string_list_property(element, "redefined_features"))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let source_feature = string_property(element, metadata, "source_feature");
    let candidate_sources = candidate_derivation_sources(&type_refs, &specializes, &redefines);
    let rule = classify(
        declared_name.as_deref(),
        &type_refs,
        type_label.as_deref(),
        &subsets,
        &specializes,
        &redefines,
    );
    let status = if is_implemented_core_feature(owner.as_deref(), declared_name.as_deref()) {
        BurnStatus::ImplementedCore
    } else {
        match rule {
            RuleKind::Manual => BurnStatus::ManualAnalysis,
            _ => BurnStatus::NeedsPilotComparison,
        }
    };

    let canonical_id = source_feature.clone().unwrap_or_else(|| element.id.clone());

    DerivedFeatureEntry {
        id: element.id.clone(),
        canonical_id,
        library: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string(),
        owner,
        declared_name,
        type_refs,
        type_label,
        subsets,
        specializes,
        redefines,
        source_feature,
        candidate_sources,
        rule,
        status,
    }
}

fn candidate_derivation_sources(
    type_refs: &[String],
    specializes: &[String],
    redefines: &[String],
) -> Vec<String> {
    specializes
        .iter()
        .chain(redefines)
        .filter(|source| source.contains("::"))
        .filter(|source| !type_refs.iter().any(|type_ref| type_ref == *source))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_implemented_core_feature(owner: Option<&str>, declared_name: Option<&str>) -> bool {
    matches!(
        (owner.unwrap_or_default(), declared_name.unwrap_or_default()),
        ("KerML::Root::Element", "documentation")
            | ("KerML::Root::Element", "isLibraryElement")
            | ("KerML::Root::Element", "name")
            | ("KerML::Root::Element", "ownedElement")
            | ("KerML::Root::Element", "owner")
            | ("KerML::Root::Element", "owningNamespace")
            | ("KerML::Root::Element", "qualifiedName")
            | ("KerML::Root::Element", "shortName")
            | ("KerML::Root::Documentation", "documentedElement")
            | ("KerML::Root::Import", "importedElement")
            | ("KerML::Root::Membership", "memberElementId")
            | ("KerML::Root::Namespace", "member")
            | ("KerML::Root::Namespace", "membership")
            | ("KerML::Root::Relationship", "relatedElement")
            | ("KerML::Root::AnnotatingElement", "annotation")
            | ("KerML::Core::Feature", "chainingFeature")
            | ("KerML::Core::Feature", "crossFeature")
            | ("KerML::Core::Feature", "featureTarget")
            | ("KerML::Core::Feature", "featuringType")
            | ("KerML::Core::Type", "differencingType")
            | ("KerML::Core::Type", "featureMembership")
            | ("KerML::Core::Type", "intersectingType")
            | ("KerML::Core::Type", "isConjugated")
            | ("KerML::Core::Type", "unioningType")
            | ("KerML::Kernel::Flow", "payloadType")
            | ("KerML::Kernel::Flow", "sourceOutputFeature")
            | ("KerML::Kernel::Flow", "targetInputFeature")
            | ("SysML::Systems::RequirementDefinition", "text")
            | ("SysML::Systems::RequirementUsage", "text")
            | ("SysML::Systems::Usage", "isReference")
    )
}

fn classify(
    declared_name: Option<&str>,
    type_refs: &[String],
    type_label: Option<&str>,
    subsets: &[String],
    specializes: &[String],
    redefines: &[String],
) -> RuleKind {
    match declared_name.unwrap_or_default() {
        "owner" | "ownedElement" | "ownedRelationship" | "owningRelationship"
        | "owningMembership" | "owningNamespace" => return RuleKind::InverseOwnership,
        "name" | "shortName" | "qualifiedName" => return RuleKind::NameDerivation,
        "isLibraryElement" => return RuleKind::LibraryBoolean,
        "annotatedElement" | "documentedElement" => return RuleKind::AnnotationTarget,
        _ => {}
    }

    if !redefines.is_empty() {
        return RuleKind::Redefinition;
    }
    if !subsets.is_empty() && (!type_refs.is_empty() || type_label.is_some()) {
        return RuleKind::TypedSubset;
    }
    if !subsets.is_empty() || !specializes.is_empty() {
        return RuleKind::SubsetChain;
    }
    RuleKind::Manual
}

fn string_property(
    element: &KirElement,
    metadata: Option<&serde_json::Map<String, Value>>,
    key: &str,
) -> Option<String> {
    element
        .properties
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            element
                .properties
                .get(key)
                .and_then(Value::as_array)
                .and_then(|values| values.first())
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .or_else(|| {
            metadata?
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn string_list_property(element: &KirElement, key: &str) -> Vec<String> {
    match element.properties.get(key) {
        Some(Value::String(value)) => vec![value.clone()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn summarize(features: &[DerivedFeatureEntry]) -> InventorySummary {
    let mut summary = InventorySummary {
        total: features.len(),
        ..InventorySummary::default()
    };
    let unique_features = unique_features(features);
    for feature in features {
        *summary
            .by_status
            .entry(feature.status.as_str().to_string())
            .or_default() += 1;
        *summary
            .by_rule
            .entry(feature.rule.as_str().to_string())
            .or_default() += 1;
        *summary
            .by_library
            .entry(feature.library.clone())
            .or_default() += 1;
        if let Some(owner) = &feature.owner {
            *summary.by_owner.entry(owner.clone()).or_default() += 1;
        }
    }
    summary.unique_total = unique_features.len();
    for feature in &unique_features {
        *summary
            .unique_by_status
            .entry(feature.status.as_str().to_string())
            .or_default() += 1;
        *summary
            .unique_by_rule
            .entry(feature.rule.as_str().to_string())
            .or_default() += 1;
        if feature.rule == RuleKind::SubsetChain
            && feature.status == BurnStatus::NeedsPilotComparison
        {
            *summary
                .subset_chain_by_candidate_source_count
                .entry(feature.candidate_sources.len().to_string())
                .or_default() += 1;
            for source in &feature.candidate_sources {
                *summary
                    .subset_chain_by_anchor
                    .entry(source.clone())
                    .or_default() += 1;
            }
        }
    }
    summary
}

fn candidate_manifest(features: &[DerivedFeatureEntry]) -> CandidateManifest {
    let derived_features = unique_features(features)
        .into_iter()
        .filter(|feature| feature.status == BurnStatus::NeedsPilotComparison)
        .filter(|feature| !feature.candidate_sources.is_empty())
        .filter_map(|feature| {
            let target_feature = feature.canonical_id;
            let target_type = feature.type_refs.into_iter().next();
            let rule = if feature.candidate_sources.len() == 1 {
                CandidateManifestRule::SubsetChain {
                    source: feature.candidate_sources.into_iter().next()?,
                    target_feature,
                    target_type,
                }
            } else {
                CandidateManifestRule::IntersectionSubsetChain {
                    sources: feature.candidate_sources,
                    target_feature,
                    target_type,
                }
            };
            Some(CandidateManifestSpec {
                owner: feature.owner?,
                feature: feature.declared_name?,
                rule,
            })
        })
        .collect();

    CandidateManifest {
        metamodel: "candidate:derived-feature-burndown".to_string(),
        derived_features,
    }
}

fn validate_candidate_manifest(manifest: &CandidateManifest) -> Result<usize> {
    let core_manifest: DerivedFeatureManifest =
        serde_json::from_value(serde_json::to_value(manifest)?)?;
    let count = core_manifest.derived_features.len();
    DerivedFeatureRegistry::with_manifest_and_builtins(Some(core_manifest))?;
    Ok(count)
}

fn unresolved_document(features: &[DerivedFeatureEntry]) -> UnresolvedDocument {
    let mut no_source_anchor = Vec::new();
    let multiple_source_anchors = Vec::new();
    let mut summary = UnresolvedSummary::default();

    for feature in unique_features(features)
        .into_iter()
        .filter(|feature| feature.status == BurnStatus::NeedsPilotComparison)
        .filter(|feature| feature.candidate_sources.is_empty())
    {
        let resolution_family = resolution_family(&feature);
        *summary
            .by_rule
            .entry(feature.rule.as_str().to_string())
            .or_default() += 1;
        *summary
            .by_resolution_family
            .entry(resolution_family.as_str().to_string())
            .or_default() += 1;
        if let Some(owner) = &feature.owner {
            *summary.by_owner.entry(owner.clone()).or_default() += 1;
        }

        summary.no_source_anchor += 1;
        no_source_anchor.push(UnresolvedFeatureEntry {
            feature,
            resolution_family,
        });
    }

    UnresolvedDocument {
        summary,
        no_source_anchor,
        multiple_source_anchors,
    }
}

fn resolution_family(feature: &DerivedFeatureEntry) -> ResolutionFamily {
    let owner = feature.owner.as_deref().unwrap_or_default();
    let declared_name = feature.declared_name.as_deref().unwrap_or_default();
    if declared_name == "isConjugated" {
        return ResolutionFamily::CoreBooleanNative;
    }
    if declared_name == "isModelLevelEvaluable" {
        return ResolutionFamily::KernelExpressionNative;
    }
    if declared_name == "isReference" {
        return ResolutionFamily::SysmlBooleanNative;
    }
    if declared_name == "text" {
        return ResolutionFamily::SysmlTextNative;
    }
    if owner.starts_with("SysML::") {
        return ResolutionFamily::SysmlDomainNative;
    }
    ResolutionFamily::CoreStructuralNative
}

fn unique_features(features: &[DerivedFeatureEntry]) -> Vec<DerivedFeatureEntry> {
    let mut unique_features = BTreeMap::<String, DerivedFeatureEntry>::new();
    for feature in features {
        unique_features
            .entry(feature.canonical_id.clone())
            .and_modify(|existing| merge_unique_entry(existing, feature))
            .or_insert_with(|| feature.clone());
    }
    unique_features.into_values().collect()
}

fn merge_unique_entry(existing: &mut DerivedFeatureEntry, duplicate: &DerivedFeatureEntry) {
    if existing.owner.is_none() {
        existing.owner = duplicate.owner.clone();
    }
    if existing.declared_name.is_none() {
        existing.declared_name = duplicate.declared_name.clone();
    }
    if existing.type_label.is_none() {
        existing.type_label = duplicate.type_label.clone();
    }
    existing.type_refs = merge_strings(&existing.type_refs, &duplicate.type_refs);
    existing.subsets = merge_strings(&existing.subsets, &duplicate.subsets);
    existing.specializes = merge_strings(&existing.specializes, &duplicate.specializes);
    existing.redefines = merge_strings(&existing.redefines, &duplicate.redefines);
    existing.candidate_sources =
        merge_strings(&existing.candidate_sources, &duplicate.candidate_sources);
    if existing.status != BurnStatus::ImplementedCore
        && duplicate.status == BurnStatus::ImplementedCore
    {
        existing.status = BurnStatus::ImplementedCore;
    }
}

fn merge_strings(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .chain(right)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn render_report(
    summary: &InventorySummary,
    features: &[DerivedFeatureEntry],
    unresolved: &UnresolvedDocument,
) -> String {
    let mut output = String::new();
    output.push_str("# Derived Feature Burn-Down\n\n");
    output.push_str(
        "Generated by `cargo run -q -p mercurio-tools --bin derived_feature_burndown`.\n\n",
    );
    output.push_str("## Summary\n\n");
    output.push_str(&format!("- Total derived features: {}\n", summary.total));
    output.push_str(&format!(
        "- Unique source features: {}\n",
        summary.unique_total
    ));
    output.push_str(&format!(
        "- Manifest candidate specs: {}\n",
        summary.manifest_candidate_specs
    ));
    output.push_str(&format!(
        "- Manifest candidate specs validated by core registry: {}\n",
        summary.validated_manifest_candidate_specs
    ));
    output.push_str(&format!(
        "- Unresolved without source anchor: {}\n",
        unresolved.summary.no_source_anchor
    ));
    output.push_str(&format!(
        "- Unresolved with multiple source anchors: {}\n",
        unresolved.summary.multiple_source_anchors
    ));
    output.push_str("- Scope: `resources/kernel/kerml-kernel.kir.json` and `resources/metamodels/sysml-2.0-metamodel-0.57.0/stdlib/sysml-library.kir.json`\n\n");

    output.push_str("## Status\n\n");
    output.push_str("| Status | Unique | Raw rows |\n");
    output.push_str("| --- | ---: | ---: |\n");
    for (status, unique_count) in &summary.unique_by_status {
        let raw_count = summary.by_status.get(status).copied().unwrap_or_default();
        output.push_str(&format!("| `{status}` | {unique_count} | {raw_count} |\n"));
    }
    output.push('\n');

    output.push_str("## Rule Buckets\n\n");
    output.push_str("| Rule | Unique | Raw rows | Next action |\n");
    output.push_str("| --- | ---: | ---: | --- |\n");
    for (rule, unique_count) in &summary.unique_by_rule {
        let raw_count = summary.by_rule.get(rule).copied().unwrap_or_default();
        output.push_str(&format!(
            "| `{rule}` | {unique_count} | {raw_count} | {} |\n",
            next_action(rule)
        ));
    }

    output.push_str("\n## Subset Chain Candidate Sources\n\n");
    output.push_str(
        "This table covers unresolved features classified in the `subset_chain` bucket. Single-source anchors generate `subset_chain` specs; multiple-source anchors generate `intersection_subset_chain` specs.\n\n",
    );
    output.push_str("| Candidate source count | Unique features |\n");
    output.push_str("| ---: | ---: |\n");
    for (count, feature_count) in &summary.subset_chain_by_candidate_source_count {
        output.push_str(&format!("| {count} | {feature_count} |\n"));
    }

    output.push_str("\n## Subset Chain Anchors\n\n");
    output.push_str("| Source anchor | Unique features |\n");
    output.push_str("| --- | ---: |\n");
    let mut anchors = summary.subset_chain_by_anchor.iter().collect::<Vec<_>>();
    anchors.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    for (anchor, count) in anchors.into_iter().take(20) {
        output.push_str(&format!("| `{anchor}` | {count} |\n"));
    }

    output.push_str("\n## Unresolved Buckets\n\n");
    output.push_str("See `docs/development/generated/derived-feature-unresolved.json` for the full unresolved set grouped by missing versus ambiguous source anchors.\n\n");
    output.push_str("| Bucket | Unique features | Next action |\n");
    output.push_str("| --- | ---: | --- |\n");
    output.push_str(&format!(
        "| No source anchor | {} | Identify native functions or metamodel-specific operators; these cannot be safely generated as subset-chain specs. |\n",
        unresolved.summary.no_source_anchor
    ));
    output.push_str(&format!(
        "| Multiple source anchors | {} | Covered by generated `intersection_subset_chain` specs; keep Pilot comparison as validation work. |\n",
        unresolved.summary.multiple_source_anchors
    ));
    output.push_str("\n## Remaining Resolution Families\n\n");
    output.push_str("| Resolution family | Unique features |\n");
    output.push_str("| --- | ---: |\n");
    for (family, count) in &unresolved.summary.by_resolution_family {
        output.push_str(&format!("| `{family}` | {count} |\n"));
    }

    output.push_str("\n## Pilot Comparison Plan\n\n");
    output.push_str("1. Select fixtures that exercise each rule bucket, starting with documentation, ownership, namespace membership, and type-owned features.\n");
    output.push_str("2. Export the same fixtures through Pilot and Mercurio.\n");
    output.push_str(
        "3. Normalize both outputs to element id, kind, owner, and derived feature values.\n",
    );
    output.push_str("4. Mark each feature bucket `pilot_matched` once mismatches are either fixed or accepted as explicit compatibility differences.\n\n");

    output.push_str("## Initial Burn-Down\n\n");
    output.push_str("| Status | Rule | Feature | Owner | Type | Subsets |\n");
    output.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for feature in features.iter().take(250) {
        output.push_str(&format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |\n",
            feature.status.as_str(),
            feature.rule.as_str(),
            feature.declared_name.as_deref().unwrap_or(""),
            feature.owner.as_deref().unwrap_or(""),
            feature
                .type_label
                .as_deref()
                .or_else(|| feature.type_refs.first().map(String::as_str))
                .unwrap_or(""),
            feature.subsets.join(", ")
        ));
    }
    if features.len() > 250 {
        output.push_str(&format!(
            "\nInventory table truncated to 250 rows. See `docs/development/generated/derived-feature-inventory.json` for all {} entries.\n",
            features.len()
        ));
    }
    output
}

fn next_action(rule: &str) -> &'static str {
    match rule {
        "inverse_ownership" => {
            "Core `owner`/`ownedElement` are implemented; remaining ownership features need generic membership/relationship inverses."
        }
        "typed_subset" => {
            "Core typed subset support exists for documentation; broaden manifest generation for metamodel-declared subsets."
        }
        "subset_chain" => {
            "Core subset-chain operator exists; generate and compare concrete specs by source anchor."
        }
        "redefinition" => {
            "Compare to Pilot before implementing, because redefinition semantics vary by feature."
        }
        "name_derivation" => "Core name, shortName, and qualifiedName are implemented.",
        "annotation_target" => {
            "Documentation documentedElement is implemented; general Annotation/AnnotatingElement targets remain."
        }
        "library_boolean" => "Core isLibraryElement is implemented.",
        _ => "Manual rule analysis required.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_candidate_manifest_validates_against_core_registry() {
        let mut features = Vec::new();
        for input in [
            repo_path("resources/kernel/kerml-kernel.kir.json"),
            repo_path("resources/metamodels/sysml-2.0-metamodel-0.57.0/stdlib/sysml-library.kir.json"),
        ] {
            let document = KirDocument::from_path(&input).expect("default KIR input should load");
            for element in &document.elements {
                if is_derived_feature(element) {
                    features.push(inventory_entry(&input, element));
                }
            }
        }

        features.sort_by(|left, right| left.id.cmp(&right.id));
        propagate_canonical_statuses(&mut features);
        let manifest = candidate_manifest(&features);

        assert_eq!(validate_candidate_manifest(&manifest).unwrap(), 299);
    }
}
