use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};

use mercurio_core::KirFieldKind;
use mercurio_sysml::sysml_field_specs;
use serde::Deserialize;

const DEFAULT_PROFILE_ID: &str = "sysml-2.0-pilot-2026-04";

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let text = std::fs::read_to_string(&args.metamodel_extract)?;
    let extract: MetamodelExtract = serde_json::from_str(&text)?;
    let overlay = read_field_overlay(&args.field_overlay)?;

    let mut audit = Audit::default();
    audit_schema(&mut audit, &extract);
    audit_uniqueness(&mut audit, &extract);
    audit_relationship_integrity(&mut audit, &extract);
    audit_spot_checks(&mut audit, &extract);
    audit_field_spec_coverage(&mut audit, &extract, overlay.as_ref());

    audit.print(
        &args.profile_id,
        &args.metamodel_extract,
        &args.field_overlay,
    );
    if audit.errors > 0 || (args.deny_warnings && audit.warnings > 0) {
        std::process::exit(1);
    }
    Ok(())
}

#[derive(Debug)]
struct Args {
    profile_id: String,
    metamodel_extract: PathBuf,
    field_overlay: PathBuf,
    deny_warnings: bool,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut profile_id = DEFAULT_PROFILE_ID.to_string();
        let mut metamodel_extract = None;
        let mut field_overlay = None;
        let mut deny_warnings = false;

        let raw = env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "--profile-id" | "--profile" => profile_id = next_string(&raw, &mut index)?,
                "--metamodel-extract" | "--extract" => {
                    metamodel_extract = Some(PathBuf::from(next_string(&raw, &mut index)?));
                }
                "--field-overlay" | "--overlay" => {
                    field_overlay = Some(PathBuf::from(next_string(&raw, &mut index)?));
                }
                "--deny-warnings" => deny_warnings = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
            index += 1;
        }

        let metamodel_extract = metamodel_extract.unwrap_or_else(|| {
            PathBuf::from("resources")
                .join("metamodels")
                .join(&profile_id)
                .join("metamodel.extract.json")
        });
        let field_overlay = field_overlay.unwrap_or_else(|| {
            PathBuf::from("resources")
                .join("metamodels")
                .join(&profile_id)
                .join("mappings")
                .join("field_specs.overlay.json")
        });

        Ok(Self {
            profile_id,
            metamodel_extract,
            field_overlay,
            deny_warnings,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --features legacy-pilot-tools --bin audit_metamodel_extract -- [--profile-id ID] [--metamodel-extract PATH] [--field-overlay PATH] [--deny-warnings]"
    );
}

fn next_string(args: &[String], index: &mut usize) -> Result<String, Box<dyn std::error::Error>> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| "missing argument value".into())
}

#[derive(Debug, Deserialize)]
struct MetamodelExtract {
    schema: String,
    source: ExtractSource,
    packages: Vec<PackageExtract>,
    metaclasses: Vec<MetaclassExtract>,
    generalizations: Vec<GeneralizationExtract>,
    structural_features: Vec<StructuralFeatureExtract>,
    containment_features: Vec<ContainmentFeatureExtract>,
}

#[derive(Debug, Deserialize)]
struct ExtractSource {
    profile_id: String,
    source_files: Vec<SourceFile>,
}

#[derive(Debug, Deserialize)]
struct SourceFile {
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct PackageExtract {
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct MetaclassExtract {
    qualified_name: String,
}

#[derive(Debug, Deserialize)]
struct GeneralizationExtract {
    specific: String,
    general: String,
}

#[derive(Debug, Deserialize)]
struct StructuralFeatureExtract {
    owner: String,
    name: String,
    qualified_name: String,
    kind: String,
    target: Option<String>,
    upper_bound: i32,
    containment: bool,
    subsets: Vec<String>,
    redefines: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ContainmentFeatureExtract {
    owner: String,
    feature: String,
    target: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FieldSpecOverlay {
    schema: String,
    source: Option<serde_json::Value>,
    entries: Vec<FieldSpecOverlayEntry>,
}

#[derive(Debug, Deserialize)]
struct FieldSpecOverlayEntry {
    field: String,
    kind: String,
    classification: String,
    #[serde(default)]
    pilot_features: Vec<String>,
    reason: String,
}

fn read_field_overlay(path: &Path) -> Result<Option<FieldSpecOverlay>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&text)?))
}

#[derive(Default)]
struct Audit {
    errors: usize,
    warnings: usize,
    infos: Vec<String>,
}

impl Audit {
    fn error(&mut self, message: impl Into<String>) {
        self.errors += 1;
        self.infos.push(format!("ERROR {}", message.into()));
    }

    fn warn(&mut self, message: impl Into<String>) {
        self.warnings += 1;
        self.infos.push(format!("WARN  {}", message.into()));
    }

    fn ok(&mut self, message: impl Into<String>) {
        self.infos.push(format!("OK    {}", message.into()));
    }

    fn print(&self, profile_id: &str, metamodel_extract: &Path, field_overlay: &Path) {
        println!("Metamodel extract audit");
        println!("  profile: {profile_id}");
        println!("  extract: {}", metamodel_extract.display());
        println!("  field overlay: {}", field_overlay.display());
        println!();
        for info in &self.infos {
            println!("{info}");
        }
        println!();
        println!(
            "summary: {} error(s), {} warning(s)",
            self.errors, self.warnings
        );
    }
}

fn audit_schema(audit: &mut Audit, extract: &MetamodelExtract) {
    if extract.schema == "dev.mercurio.pilot-metamodel-extract.v1" {
        audit.ok("metamodel extract schema is current");
    } else {
        audit.error(format!(
            "unexpected metamodel extract schema `{}`",
            extract.schema
        ));
    }

    if extract.source.profile_id.is_empty() {
        audit.error("source profile_id is empty");
    } else {
        audit.ok(format!(
            "source profile id is `{}`",
            extract.source.profile_id
        ));
    }

    let missing_hashes = extract
        .source
        .source_files
        .iter()
        .filter(|file| file.path.is_empty() || file.sha256.len() != 64)
        .count();
    if missing_hashes == 0 {
        audit.ok(format!(
            "source block records {} hashed source file(s)",
            extract.source.source_files.len()
        ));
    } else {
        audit.error(format!(
            "{missing_hashes} source file entry(s) lack path or sha256"
        ));
    }
}

fn audit_uniqueness(audit: &mut Audit, extract: &MetamodelExtract) {
    audit_unique(
        audit,
        "package",
        extract
            .packages
            .iter()
            .map(|package| package.display_name.as_str()),
    );
    audit_unique(
        audit,
        "metaclass",
        extract
            .metaclasses
            .iter()
            .map(|class| class.qualified_name.as_str()),
    );
    audit_unique(
        audit,
        "structural feature",
        extract
            .structural_features
            .iter()
            .map(|feature| feature.qualified_name.as_str()),
    );
}

fn audit_unique<'a>(audit: &mut Audit, label: &str, values: impl Iterator<Item = &'a str>) {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    let mut count = 0;
    for value in values {
        count += 1;
        if !seen.insert(value.to_string()) {
            duplicates.insert(value.to_string());
        }
    }
    if duplicates.is_empty() {
        audit.ok(format!("{count} {label}(s) are unique"));
    } else {
        audit.error(format!(
            "duplicate {label}(s): {}",
            duplicates.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
}

fn audit_relationship_integrity(audit: &mut Audit, extract: &MetamodelExtract) {
    let classes = extract
        .metaclasses
        .iter()
        .map(|class| class.qualified_name.clone())
        .collect::<BTreeSet<_>>();
    let features = extract
        .structural_features
        .iter()
        .map(|feature| feature.qualified_name.clone())
        .collect::<BTreeSet<_>>();

    let mut missing_generalizations = Vec::new();
    for generalization in &extract.generalizations {
        if !classes.contains(&generalization.specific) {
            missing_generalizations.push(format!("specific `{}`", generalization.specific));
        }
        if !classes.contains(&generalization.general) {
            missing_generalizations.push(format!("general `{}`", generalization.general));
        }
    }
    if missing_generalizations.is_empty() {
        audit.ok(format!(
            "{} generalization endpoint(s) resolve",
            extract.generalizations.len() * 2
        ));
    } else {
        audit.error(format!(
            "unresolved generalization endpoint(s): {}",
            missing_generalizations.join(", ")
        ));
    }

    let mut unresolved_feature_refs = Vec::new();
    for feature in &extract.structural_features {
        if !classes.contains(&feature.owner) {
            unresolved_feature_refs.push(format!("owner `{}`", feature.owner));
        }
        if feature.kind == "reference"
            && let Some(target) = &feature.target
            && is_pilot_class_name(target)
            && !classes.contains(target)
        {
            unresolved_feature_refs.push(format!("target `{target}`"));
        }
        for reference in feature.subsets.iter().chain(feature.redefines.iter()) {
            if is_pilot_feature_name(reference) && !features.contains(reference) {
                unresolved_feature_refs.push(format!("feature reference `{reference}`"));
            }
        }
    }
    if unresolved_feature_refs.is_empty() {
        audit.ok(format!(
            "{} structural feature owner/target/reference endpoint(s) resolve",
            extract.structural_features.len()
        ));
    } else {
        audit.error(format!(
            "unresolved structural feature endpoint(s): {}",
            unresolved_feature_refs.join(", ")
        ));
    }

    let containment_feature_index = extract
        .structural_features
        .iter()
        .filter(|feature| feature.containment)
        .map(|feature| (feature.owner.as_str(), feature.name.as_str()))
        .collect::<BTreeSet<_>>();
    let mut missing_containments = Vec::new();
    for containment in &extract.containment_features {
        if !containment_feature_index
            .contains(&(containment.owner.as_str(), containment.feature.as_str()))
        {
            missing_containments.push(format!("{}::{}", containment.owner, containment.feature));
        }
        if let Some(target) = &containment.target
            && is_pilot_class_name(target)
            && !classes.contains(target)
        {
            missing_containments.push(format!("target `{target}`"));
        }
    }
    if missing_containments.is_empty() {
        audit.ok(format!(
            "{} containment feature projection(s) match structural features",
            extract.containment_features.len()
        ));
    } else {
        audit.error(format!(
            "invalid containment feature projection(s): {}",
            missing_containments.join(", ")
        ));
    }
}

fn audit_spot_checks(audit: &mut Audit, extract: &MetamodelExtract) {
    let generalizations = extract
        .generalizations
        .iter()
        .map(|entry| (entry.specific.as_str(), entry.general.as_str()))
        .collect::<BTreeSet<_>>();
    if generalizations.contains(&("SysML::RequirementUsage", "SysML::ConstraintUsage")) {
        audit.ok("RequirementUsage specializes ConstraintUsage");
    } else {
        audit.error("RequirementUsage does not specialize ConstraintUsage");
    }

    for owner in ["SysML::AttributeUsage", "SysML::PartUsage"] {
        let containments = extract
            .structural_features
            .iter()
            .filter(|feature| feature.owner == owner && feature.containment)
            .map(|feature| feature.name.clone())
            .collect::<Vec<_>>();
        if containments.is_empty() {
            audit.ok(format!("{owner} has no direct containment features"));
        } else {
            audit.error(format!(
                "{owner} unexpectedly has direct containment feature(s): {}",
                containments.join(", ")
            ));
        }
    }
}

fn audit_field_spec_coverage(
    audit: &mut Audit,
    extract: &MetamodelExtract,
    overlay: Option<&FieldSpecOverlay>,
) {
    let mut pilot_feature_names = BTreeSet::new();
    let mut pilot_feature_qualified_names = BTreeSet::new();
    for feature in &extract.structural_features {
        pilot_feature_names.insert(feature.name.clone());
        pilot_feature_names.insert(to_snake_case(&feature.name));
        pilot_feature_qualified_names.insert(feature.qualified_name.clone());
    }

    let mut covered = 0;
    let mut gaps = Vec::new();
    for (field, kind) in sysml_field_specs() {
        if pilot_feature_names.contains(*field) {
            covered += 1;
            match expected_field_kind(field, extract) {
                Some(expected) if expected != *kind => {
                    gaps.push(FieldSpecGap {
                        field: (*field).to_string(),
                        kind: *kind,
                        expected_kind: Some(expected),
                        gap: FieldSpecGapKind::KindOverride,
                    });
                }
                None => {
                    gaps.push(FieldSpecGap {
                        field: (*field).to_string(),
                        kind: *kind,
                        expected_kind: None,
                        gap: FieldSpecGapKind::AmbiguousDirect,
                    });
                }
                _ => {}
            }
        } else {
            gaps.push(FieldSpecGap {
                field: (*field).to_string(),
                kind: *kind,
                expected_kind: None,
                gap: FieldSpecGapKind::UnmatchedName,
            });
        }
    }

    audit.ok(format!(
        "{covered} SysML field spec(s) match Pilot feature names"
    ));
    audit_field_overlay(
        audit,
        &gaps,
        overlay,
        &pilot_feature_qualified_names,
        sysml_field_specs()
            .iter()
            .map(|(field, _)| (*field).to_string())
            .collect(),
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldSpecGapKind {
    KindOverride,
    AmbiguousDirect,
    UnmatchedName,
}

#[derive(Clone, Debug)]
struct FieldSpecGap {
    field: String,
    kind: KirFieldKind,
    expected_kind: Option<KirFieldKind>,
    gap: FieldSpecGapKind,
}

fn audit_field_overlay(
    audit: &mut Audit,
    gaps: &[FieldSpecGap],
    overlay: Option<&FieldSpecOverlay>,
    pilot_feature_qualified_names: &BTreeSet<String>,
    field_specs: BTreeSet<String>,
) {
    if gaps.is_empty() {
        audit.ok("all SysML field specs map directly to Pilot metamodel feature shapes");
        if overlay.is_some() {
            audit.warn("field spec overlay exists but no field gaps remain");
        }
        return;
    }

    let Some(overlay) = overlay else {
        audit.warn(format!(
            "{} SysML field spec gap(s) require field_specs.overlay.json classification: {}",
            gaps.len(),
            summarize_gaps(gaps)
        ));
        return;
    };

    if overlay.schema == "dev.mercurio.field-spec-overlay.v1" {
        audit.ok("field spec overlay schema is current");
    } else {
        audit.error(format!(
            "unexpected field spec overlay schema `{}`",
            overlay.schema
        ));
    }
    if overlay.source.is_some() {
        audit.ok("field spec overlay declares a source block");
    } else {
        audit.error("field spec overlay is missing a source block");
    }

    let mut seen = BTreeSet::new();
    let mut entries_by_field = std::collections::BTreeMap::new();
    for entry in &overlay.entries {
        if !seen.insert(entry.field.clone()) {
            audit.error(format!(
                "duplicate field spec overlay entry `{}`",
                entry.field
            ));
        }
        if !field_specs.contains(&entry.field) {
            audit.error(format!(
                "field spec overlay entry `{}` is not present in sysml_field_specs()",
                entry.field
            ));
        }
        if entry.reason.trim().is_empty() {
            audit.error(format!(
                "field spec overlay entry `{}` has an empty reason",
                entry.field
            ));
        }
        if !matches!(
            entry.classification.as_str(),
            "pilot-alias" | "mercurio-extension" | "kir-shape-override"
        ) {
            audit.error(format!(
                "field spec overlay entry `{}` has unknown classification `{}`",
                entry.field, entry.classification
            ));
        }
        if parse_field_kind(&entry.kind).is_none() {
            audit.error(format!(
                "field spec overlay entry `{}` has unknown kind `{}`",
                entry.field, entry.kind
            ));
        }
        for pilot_feature in &entry.pilot_features {
            if !pilot_feature_qualified_names.contains(pilot_feature) {
                audit.error(format!(
                    "field spec overlay entry `{}` references unknown Pilot feature `{pilot_feature}`",
                    entry.field
                ));
            }
        }
        entries_by_field.insert(entry.field.as_str(), entry);
    }

    let mut missing = Vec::new();
    let mut wrong_classification = Vec::new();
    for gap in gaps {
        let Some(entry) = entries_by_field.get(gap.field.as_str()) else {
            missing.push(gap.clone());
            continue;
        };
        if parse_field_kind(&entry.kind) != Some(gap.kind) {
            audit.error(format!(
                "field spec overlay entry `{}` kind `{}` does not match sysml_field_specs() kind {:?}",
                entry.field, entry.kind, gap.kind
            ));
        }
        match gap.gap {
            FieldSpecGapKind::KindOverride | FieldSpecGapKind::AmbiguousDirect => {
                if entry.classification != "kir-shape-override" {
                    wrong_classification.push(format!(
                        "`{}` should use `kir-shape-override` for metamodel kind {:?}",
                        gap.field, gap.expected_kind
                    ));
                }
                if entry.pilot_features.is_empty() {
                    audit.error(format!(
                        "field spec overlay entry `{}` must cite the Pilot feature it overrides",
                        entry.field
                    ));
                }
            }
            FieldSpecGapKind::UnmatchedName => {
                if entry.classification == "kir-shape-override" {
                    wrong_classification.push(format!(
                        "`{}` has no direct Pilot feature-name match and should be `pilot-alias` or `mercurio-extension`",
                        gap.field
                    ));
                }
                if entry.classification == "pilot-alias" && entry.pilot_features.is_empty() {
                    audit.error(format!(
                        "field spec overlay entry `{}` is a pilot-alias but cites no Pilot feature",
                        entry.field
                    ));
                }
            }
        }
    }

    if missing.is_empty() {
        audit.ok(format!(
            "field spec overlay classifies all {} field gap(s)",
            gaps.len()
        ));
    } else {
        audit.warn(format!(
            "{} field spec gap(s) lack overlay entries: {}",
            missing.len(),
            summarize_gaps(&missing)
        ));
    }

    if !wrong_classification.is_empty() {
        audit.error(format!(
            "field spec overlay classification issue(s): {}",
            wrong_classification.join(", ")
        ));
    }
}

fn summarize_gaps(gaps: &[FieldSpecGap]) -> String {
    gaps.iter()
        .map(|gap| match gap.expected_kind {
            Some(expected) => format!("`{}` ({:?}, metamodel {:?})", gap.field, gap.kind, expected),
            None => format!("`{}` ({:?})", gap.field, gap.kind),
        })
        .collect::<Vec<_>>()
        .join(", ")
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

fn expected_field_kind(field: &str, extract: &MetamodelExtract) -> Option<KirFieldKind> {
    let candidates = extract
        .structural_features
        .iter()
        .filter(|feature| feature.name == field || to_snake_case(&feature.name) == field)
        .map(|feature| {
            if feature.kind == "reference" {
                if feature.upper_bound == 1 {
                    KirFieldKind::Reference
                } else {
                    KirFieldKind::ReferenceList
                }
            } else {
                KirFieldKind::Scalar
            }
        })
        .collect::<Vec<_>>();
    if candidates
        .first()
        .is_some_and(|first| candidates.iter().all(|candidate| candidate == first))
    {
        candidates.first().copied()
    } else {
        None
    }
}

fn is_pilot_class_name(value: &str) -> bool {
    value.starts_with("SysML::") || value.starts_with("KerML::")
}

fn is_pilot_feature_name(value: &str) -> bool {
    is_pilot_class_name(value) && value.matches("::").count() == 2
}

fn to_snake_case(value: &str) -> String {
    let mut out = String::new();
    let chars = value.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase()
            && index > 0
            && (chars[index - 1].is_ascii_lowercase()
                || chars
                    .get(index + 1)
                    .is_some_and(|next| next.is_ascii_lowercase()))
        {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_cases_camel_case_feature_names() {
        assert_eq!(to_snake_case("ownedFeature"), "owned_feature");
        assert_eq!(to_snake_case("isID"), "is_id");
        assert_eq!(to_snake_case("URIValue"), "uri_value");
    }
}
