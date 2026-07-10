use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use mercurio_tools::sha256_file;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const DEFAULT_PROFILE_ID: &str = "sysml-2.0-pilot-2026-04";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    let seed = generate_seed(&args)?;
    let serialized = serde_json::to_string_pretty(&seed)?;

    if args.check {
        let existing = normalize_for_check(read_json(&args.out)?);
        let fresh = normalize_for_check(serde_json::from_str::<Value>(&serialized)?);
        if existing != fresh {
            return Err(format!(
                "construct seed drift: {} does not match grammar extract + overlay",
                args.out.display()
            )
            .into());
        }
        println!("construct seed is current: {}", args.out.display());
        return Ok(());
    }

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&args.out, format!("{serialized}\n"))?;
    println!("wrote construct seed: {}", args.out.display());
    Ok(())
}

#[derive(Debug)]
struct Args {
    grammar_extract: PathBuf,
    overlay: PathBuf,
    out: PathBuf,
    check: bool,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut profile_id = DEFAULT_PROFILE_ID.to_string();
        let mut grammar_extract = None;
        let mut overlay = None;
        let mut out = None;
        let mut check = false;

        let raw = env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        while index < raw.len() {
            match raw[index].as_str() {
                "--profile-id" | "--profile" => profile_id = next_string(&raw, &mut index)?,
                "--grammar-extract" => grammar_extract = Some(next_path(&raw, &mut index)?),
                "--overlay" => overlay = Some(next_path(&raw, &mut index)?),
                "--out" => out = Some(next_path(&raw, &mut index)?),
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
        Ok(Self {
            grammar_extract: grammar_extract
                .unwrap_or_else(|| profile_root.join("grammar.extract.json")),
            overlay: overlay
                .unwrap_or_else(|| profile_root.join("mappings/constructs.overlay.json")),
            out: out
                .unwrap_or_else(|| profile_root.join("mappings/metamodel_constructs.seed.json")),
            check,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --features legacy-pilot-tools --bin generate_pilot_constructs_seed -- [--profile-id ID] [--grammar-extract PATH] [--overlay PATH] [--out PATH] [--check]"
    );
}

fn next_string(args: &[String], index: &mut usize) -> Result<String, Box<dyn std::error::Error>> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| "missing argument value".into())
}

fn next_path(args: &[String], index: &mut usize) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(PathBuf::from(next_string(args, index)?))
}

#[derive(Deserialize)]
struct GrammarExtract {
    source: Value,
    constructs: Vec<GrammarConstruct>,
}

#[derive(Clone, Deserialize)]
struct GrammarConstruct {
    construct: String,
    metaclass: String,
    source_file: String,
    line: usize,
}

#[derive(Deserialize)]
struct ConstructsOverlay {
    construct_selections: Vec<ConstructSelection>,
    explicit_constructs: Vec<ExplicitConstruct>,
    keyword_registry: OverlayKeywordRegistry,
    curated_sections: CuratedSections,
}

#[derive(Deserialize)]
struct ConstructSelection {
    construct: String,
    source_file: String,
    reason: String,
}

#[derive(Deserialize)]
struct ExplicitConstruct {
    construct: String,
    metaclass: String,
    reason: String,
}

#[derive(Deserialize, Serialize)]
struct OverlayKeywordRegistry {
    reason: String,
    definitions: BTreeMap<String, String>,
    usages: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct CuratedSections {
    reason: String,
    default_specialization_anchors: Value,
    semantic_specialization_defaults: Value,
    usage_semantic_specialization_overrides: Value,
    stdlib_aliases: Value,
}

#[derive(Serialize)]
struct GeneratedConstructSeed {
    source: GeneratedSource,
    keyword_registry: GeneratedKeywordRegistry,
    default_specialization_anchors: Value,
    semantic_specialization_defaults: Value,
    usage_semantic_specialization_overrides: Value,
    stdlib_aliases: Value,
    constructs: Vec<GeneratedConstruct>,
}

#[derive(Serialize)]
struct GeneratedSource {
    schema: &'static str,
    kind: &'static str,
    authorship: GeneratedAuthorship,
    pilot: Value,
    source_files: Vec<GeneratedSourceFile>,
    extractor: GeneratedExtractor,
    generated_at_utc: String,
}

#[derive(Serialize)]
struct GeneratedAuthorship {
    mode: &'static str,
    note: &'static str,
}

#[derive(Serialize)]
struct GeneratedSourceFile {
    path: String,
    sha256: String,
}

#[derive(Serialize)]
struct GeneratedExtractor {
    name: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
struct GeneratedKeywordRegistry {
    source: GeneratedRegistrySource,
    definitions: BTreeMap<String, String>,
    usages: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct GeneratedRegistrySource {
    mode: &'static str,
    reason: String,
}

#[derive(Serialize, PartialEq, Eq, Debug)]
struct GeneratedConstruct {
    construct: String,
    metaclass: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    grammar_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    grammar_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    overlay_reason: Option<String>,
}

fn generate_seed(args: &Args) -> Result<GeneratedConstructSeed, Box<dyn std::error::Error>> {
    let grammar: GrammarExtract = read_json(&args.grammar_extract)?;
    let overlay: ConstructsOverlay = read_json(&args.overlay)?;
    validate_overlay_reasons(&overlay)?;

    let mut constructs = Vec::new();
    for selection in &overlay.construct_selections {
        let selected = grammar
            .constructs
            .iter()
            .filter(|construct| {
                construct.construct == selection.construct
                    && construct.source_file == selection.source_file
            })
            .collect::<Vec<_>>();
        let [selected] = selected.as_slice() else {
            return Err(format!(
                "overlay construct selection `{}` from `{}` matched {} grammar extract entries",
                selection.construct,
                selection.source_file,
                selected.len()
            )
            .into());
        };
        constructs.push(GeneratedConstruct {
            construct: selected.construct.clone(),
            metaclass: selected.metaclass.clone(),
            grammar_file: Some(grammar_file_name(&selected.source_file)),
            grammar_line: Some(selected.line),
            overlay_reason: Some(selection.reason.clone()),
        });
    }
    for explicit in &overlay.explicit_constructs {
        constructs.push(GeneratedConstruct {
            construct: explicit.construct.clone(),
            metaclass: explicit.metaclass.clone(),
            grammar_file: None,
            grammar_line: None,
            overlay_reason: Some(explicit.reason.clone()),
        });
    }

    Ok(GeneratedConstructSeed {
        source: GeneratedSource {
            schema: "dev.mercurio.pilot-artifact-source.v1",
            kind: "generated-pilot-construct-seed",
            authorship: GeneratedAuthorship {
                mode: "generated-with-overlay",
                note: "Generated from grammar.extract.json and constructs.overlay.json; overlay carries Mercurio compiler subset and compatibility decisions.",
            },
            pilot: grammar.source.get("pilot").cloned().unwrap_or(Value::Null),
            source_files: vec![
                GeneratedSourceFile {
                    path: path_to_slash(&args.grammar_extract),
                    sha256: sha256_file(&args.grammar_extract)?,
                },
                GeneratedSourceFile {
                    path: path_to_slash(&args.overlay),
                    sha256: sha256_file(&args.overlay)?,
                },
            ],
            extractor: GeneratedExtractor {
                name: "generate_pilot_constructs_seed",
                version: env!("CARGO_PKG_VERSION"),
            },
            generated_at_utc: now_utc_rfc3339()?,
        },
        keyword_registry: GeneratedKeywordRegistry {
            source: GeneratedRegistrySource {
                mode: "overlay",
                reason: overlay.keyword_registry.reason,
            },
            definitions: overlay.keyword_registry.definitions,
            usages: overlay.keyword_registry.usages,
        },
        default_specialization_anchors: overlay.curated_sections.default_specialization_anchors,
        semantic_specialization_defaults: overlay.curated_sections.semantic_specialization_defaults,
        usage_semantic_specialization_overrides: overlay
            .curated_sections
            .usage_semantic_specialization_overrides,
        stdlib_aliases: overlay.curated_sections.stdlib_aliases,
        constructs,
    })
}

fn validate_overlay_reasons(overlay: &ConstructsOverlay) -> Result<(), Box<dyn std::error::Error>> {
    for selection in &overlay.construct_selections {
        if selection.reason.trim().is_empty() {
            return Err(format!(
                "construct selection `{}` is missing reason",
                selection.construct
            )
            .into());
        }
    }
    for explicit in &overlay.explicit_constructs {
        if explicit.reason.trim().is_empty() {
            return Err(format!(
                "explicit construct `{}` is missing reason",
                explicit.construct
            )
            .into());
        }
    }
    if overlay.keyword_registry.reason.trim().is_empty() {
        return Err("keyword_registry overlay is missing reason".into());
    }
    if overlay.curated_sections.reason.trim().is_empty() {
        return Err("curated_sections overlay is missing reason".into());
    }
    Ok(())
}

fn grammar_file_name(source_file: &str) -> String {
    Path::new(source_file)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(source_file)
        .to_string()
}

fn path_to_slash(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
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

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(text.trim_start_matches('\u{feff}'))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_constructs_from_selection_and_explicit_overlay() {
        let grammar = GrammarExtract {
            source: serde_json::json!({
                "pilot": {
                    "commit": "pilot"
                }
            }),
            constructs: vec![GrammarConstruct {
                construct: "PartDefinition".to_string(),
                metaclass: "SysML::PartDefinition".to_string(),
                source_file: "org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext".to_string(),
                line: 937,
            }],
        };
        let overlay = ConstructsOverlay {
            construct_selections: vec![ConstructSelection {
                construct: "PartDefinition".to_string(),
                source_file: "org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext".to_string(),
                reason: "current compiler subset".to_string(),
            }],
            explicit_constructs: vec![ExplicitConstruct {
                construct: "FeatureUsage".to_string(),
                metaclass: "KerML::Feature".to_string(),
                reason: "compatibility alias".to_string(),
            }],
            keyword_registry: OverlayKeywordRegistry {
                reason: "keyword choices".to_string(),
                definitions: BTreeMap::new(),
                usages: BTreeMap::new(),
            },
            curated_sections: CuratedSections {
                reason: "curated profile data".to_string(),
                default_specialization_anchors: serde_json::json!({}),
                semantic_specialization_defaults: serde_json::json!({}),
                usage_semantic_specialization_overrides: serde_json::json!({}),
                stdlib_aliases: serde_json::json!({}),
            },
        };

        validate_overlay_reasons(&overlay).unwrap();
        let selected = grammar
            .constructs
            .iter()
            .find(|construct| construct.construct == "PartDefinition")
            .unwrap();
        let generated = GeneratedConstruct {
            construct: selected.construct.clone(),
            metaclass: selected.metaclass.clone(),
            grammar_file: Some(grammar_file_name(&selected.source_file)),
            grammar_line: Some(selected.line),
            overlay_reason: Some(overlay.construct_selections[0].reason.clone()),
        };

        assert_eq!(
            generated,
            GeneratedConstruct {
                construct: "PartDefinition".to_string(),
                metaclass: "SysML::PartDefinition".to_string(),
                grammar_file: Some("SysML.xtext".to_string()),
                grammar_line: Some(937),
                overlay_reason: Some("current compiler subset".to_string())
            }
        );
    }

    #[test]
    fn overlay_reasons_are_required() {
        let overlay = ConstructsOverlay {
            construct_selections: vec![ConstructSelection {
                construct: "PartDefinition".to_string(),
                source_file: "SysML.xtext".to_string(),
                reason: String::new(),
            }],
            explicit_constructs: Vec::new(),
            keyword_registry: OverlayKeywordRegistry {
                reason: "keywords".to_string(),
                definitions: BTreeMap::new(),
                usages: BTreeMap::new(),
            },
            curated_sections: CuratedSections {
                reason: "sections".to_string(),
                default_specialization_anchors: serde_json::json!({}),
                semantic_specialization_defaults: serde_json::json!({}),
                usage_semantic_specialization_overrides: serde_json::json!({}),
                stdlib_aliases: serde_json::json!({}),
            },
        };

        assert!(validate_overlay_reasons(&overlay).is_err());
    }
}
