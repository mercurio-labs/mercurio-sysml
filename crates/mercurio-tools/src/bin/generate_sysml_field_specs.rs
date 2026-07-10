use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};

use mercurio_core::KirFieldKind;
use mercurio_sysml::sysml_field_specs;
use mercurio_tools::sha256_file;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_PROFILE_ID: &str = "sysml-2.0-pilot-2026-04";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let registry = build_registry(&args)?;
    let serialized = serde_json::to_string_pretty(&registry)?;

    if args.check {
        let existing_text = std::fs::read_to_string(&args.out)?;
        let existing = normalize_for_check(serde_json::from_str::<Value>(&existing_text)?);
        let fresh = normalize_for_check(serde_json::from_str::<Value>(&serialized)?);
        if existing != fresh {
            return Err(format!(
                "field specs drift: {} does not match fresh generation",
                args.out.display()
            )
            .into());
        }
        println!("field specs are current: {}", args.out.display());
        return Ok(());
    }

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, format!("{serialized}\n"))?;
    println!("wrote field specs: {}", args.out.display());
    Ok(())
}

#[derive(Debug)]
struct Args {
    profile_id: String,
    metamodel_extract: PathBuf,
    field_overlay: PathBuf,
    out: PathBuf,
    check: bool,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut profile_id = DEFAULT_PROFILE_ID.to_string();
        let mut metamodel_extract = None;
        let mut field_overlay = None;
        let mut out = None;
        let mut check = false;

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
                "--out" => out = Some(PathBuf::from(next_string(&raw, &mut index)?)),
                "--check" => check = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
            index += 1;
        }

        let profile_root = PathBuf::from("resources")
            .join("metamodels")
            .join(&profile_id);
        let metamodel_extract =
            metamodel_extract.unwrap_or_else(|| profile_root.join("metamodel.extract.json"));
        let field_overlay = field_overlay.unwrap_or_else(|| {
            profile_root
                .join("mappings")
                .join("field_specs.overlay.json")
        });
        let out = out.unwrap_or_else(|| {
            profile_root
                .join("mappings")
                .join("field_specs.generated.json")
        });

        Ok(Self {
            profile_id,
            metamodel_extract,
            field_overlay,
            out,
            check,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --features legacy-pilot-tools --bin generate_sysml_field_specs -- [--profile-id ID] [--metamodel-extract PATH] [--field-overlay PATH] [--out PATH] [--check]"
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
    structural_features: Vec<StructuralFeatureExtract>,
}

#[derive(Debug, Deserialize)]
struct StructuralFeatureExtract {
    name: String,
    qualified_name: String,
    kind: String,
    upper_bound: i32,
}

#[derive(Debug, Deserialize)]
struct FieldSpecOverlay {
    entries: Vec<FieldSpecOverlayEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct FieldSpecOverlayEntry {
    field: String,
    kind: String,
    classification: String,
    #[serde(default)]
    pilot_features: Vec<String>,
    reason: String,
}

#[derive(Serialize)]
struct FieldSpecRegistry {
    schema: &'static str,
    source: FieldSpecRegistrySource,
    fields: Vec<FieldSpecEntry>,
}

#[derive(Serialize)]
struct FieldSpecRegistrySource {
    schema: &'static str,
    profile_id: String,
    source_files: Vec<SourceFile>,
    generator: GeneratorSource,
    generated_at_utc: String,
}

#[derive(Serialize)]
struct SourceFile {
    path: String,
    sha256: String,
}

#[derive(Serialize)]
struct GeneratorSource {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct FieldSpecEntry {
    field: String,
    kind: String,
    source: FieldSpecEntrySource,
}

#[derive(Serialize)]
struct FieldSpecEntrySource {
    classification: String,
    pilot_features: Vec<String>,
    reason: String,
}

fn build_registry(args: &Args) -> Result<FieldSpecRegistry, Box<dyn std::error::Error>> {
    let extract: MetamodelExtract =
        serde_json::from_str(&std::fs::read_to_string(&args.metamodel_extract)?)?;
    let overlay: FieldSpecOverlay =
        serde_json::from_str(&std::fs::read_to_string(&args.field_overlay)?)?;

    let overlay_by_field = overlay
        .entries
        .into_iter()
        .map(|entry| (entry.field.clone(), entry))
        .collect::<BTreeMap<_, _>>();
    let features_by_name = features_by_field_name(&extract.structural_features);
    let mut fields = Vec::new();

    for (field, kind) in sysml_field_specs() {
        let kind_name = field_kind_name(*kind).to_string();
        let entry = if let Some(overlay_entry) = overlay_by_field.get(*field) {
            if overlay_entry.kind != kind_name {
                return Err(format!(
                    "overlay kind `{}` for `{field}` does not match current table kind `{kind_name}`",
                    overlay_entry.kind
                )
                .into());
            }
            FieldSpecEntry {
                field: (*field).to_string(),
                kind: kind_name,
                source: FieldSpecEntrySource {
                    classification: overlay_entry.classification.clone(),
                    pilot_features: overlay_entry.pilot_features.clone(),
                    reason: overlay_entry.reason.clone(),
                },
            }
        } else {
            let Some(pilot_features) = features_by_name.get(*field) else {
                return Err(format!(
                    "field `{field}` has no direct Pilot feature match and no overlay entry"
                )
                .into());
            };
            let Some(expected_kind) = expected_field_kind(field, &extract.structural_features)
            else {
                return Err(format!(
                    "field `{field}` has ambiguous Pilot feature shapes and no overlay entry"
                )
                .into());
            };
            if expected_kind != *kind {
                return Err(format!(
                    "field `{field}` metamodel-derived kind {:?} does not match current table kind {:?}; add overlay",
                    expected_kind, kind
                )
                .into());
            }
            FieldSpecEntry {
                field: (*field).to_string(),
                kind: kind_name,
                source: FieldSpecEntrySource {
                    classification: "pilot-feature".to_string(),
                    pilot_features: pilot_features.clone(),
                    reason: "Directly derived from Pilot metamodel structural feature name and multiplicity."
                        .to_string(),
                },
            }
        };
        fields.push(entry);
    }

    Ok(FieldSpecRegistry {
        schema: "dev.mercurio.field-specs.v1",
        source: FieldSpecRegistrySource {
            schema: "dev.mercurio.generated-source.v1",
            profile_id: args.profile_id.clone(),
            source_files: vec![
                SourceFile {
                    path: path_to_slash(&args.metamodel_extract),
                    sha256: sha256_file(&args.metamodel_extract)?,
                },
                SourceFile {
                    path: path_to_slash(&args.field_overlay),
                    sha256: sha256_file(&args.field_overlay)?,
                },
            ],
            generator: GeneratorSource {
                name: "generate_sysml_field_specs",
                version: env!("CARGO_PKG_VERSION"),
            },
            generated_at_utc: now_utc_rfc3339()?,
        },
        fields,
    })
}

fn features_by_field_name(features: &[StructuralFeatureExtract]) -> BTreeMap<String, Vec<String>> {
    let mut by_name = BTreeMap::<String, BTreeSet<String>>::new();
    for feature in features {
        by_name
            .entry(feature.name.clone())
            .or_default()
            .insert(feature.qualified_name.clone());
        by_name
            .entry(to_snake_case(&feature.name))
            .or_default()
            .insert(feature.qualified_name.clone());
    }
    by_name
        .into_iter()
        .map(|(field, values)| (field, values.into_iter().collect()))
        .collect()
}

fn expected_field_kind(field: &str, features: &[StructuralFeatureExtract]) -> Option<KirFieldKind> {
    let candidates = features
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

fn field_kind_name(kind: KirFieldKind) -> &'static str {
    match kind {
        KirFieldKind::Scalar => "Scalar",
        KirFieldKind::Reference => "Reference",
        KirFieldKind::ReferenceList => "ReferenceList",
        KirFieldKind::Expression => "Expression",
        KirFieldKind::Metadata => "Metadata",
    }
}

fn now_utc_rfc3339() -> Result<String, Box<dyn std::error::Error>> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}

fn normalize_for_check(mut value: Value) -> Value {
    if let Some(source) = value.get_mut("source").and_then(Value::as_object_mut) {
        source.remove("generated_at_utc");
    }
    value
}

fn path_to_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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
    fn groups_feature_names_by_original_and_snake_case() {
        let features = vec![StructuralFeatureExtract {
            name: "ownedFeature".to_string(),
            qualified_name: "SysML::Type::ownedFeature".to_string(),
            kind: "reference".to_string(),
            upper_bound: -1,
        }];

        let by_name = features_by_field_name(&features);

        assert_eq!(
            by_name.get("ownedFeature").cloned().unwrap_or_default(),
            vec!["SysML::Type::ownedFeature"]
        );
        assert_eq!(
            by_name.get("owned_feature").cloned().unwrap_or_default(),
            vec!["SysML::Type::ownedFeature"]
        );
    }
}
