use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::PathBuf;

use mercurio_core::{Graph, KirDocument, RulePack};
use mercurio_sysml::{available_release_bundles, release_bundle, sysml_semantic_legality_rulepack};
use serde_json::{Value, json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let bundles = if args.all {
        available_release_bundles()?
    } else {
        vec![release_bundle(&args.selector)?]
    };

    for bundle in bundles {
        let stdlib = KirDocument::from_path(&bundle.stdlib_path)?;
        let adapter = RulePack::metamodel_adapter_from_graph(&Graph::from_document(stdlib)?);
        let element_count = adapter.metadata.get("elementCount").cloned();
        let mut metadata = BTreeMap::from([
            (
                "description".to_string(),
                Value::String(
                    "Generated SysML metamodel adapter facts and semantic legality diagnostics"
                        .to_string(),
                ),
            ),
            (
                "profileId".to_string(),
                Value::String(bundle.profile_id.clone()),
            ),
            (
                "selector".to_string(),
                Value::String(bundle.selector.clone()),
            ),
            ("release".to_string(), json!(bundle.release)),
            (
                "stdlibPath".to_string(),
                Value::String(relative_path_string(&bundle.root, &bundle.stdlib_path)),
            ),
        ]);
        if let Some(element_count) = element_count {
            metadata.insert("elementCount".to_string(), element_count);
        }
        let rulepack = merge_rulepacks(adapter, sysml_semantic_legality_rulepack(), metadata);
        let output_path = args
            .out
            .clone()
            .unwrap_or_else(|| bundle.rulepack_path.clone());
        rulepack.write_pretty_to_path(&output_path)?;
        println!(
            "wrote {} for {} (facts: {}, diagnostics: {})",
            output_path.display(),
            bundle.profile_id,
            rulepack.facts.len(),
            rulepack.diagnostics.len()
        );
    }

    Ok(())
}

#[derive(Debug)]
struct Args {
    selector: String,
    all: bool,
    out: Option<PathBuf>,
}

fn parse_args() -> Result<Args, Box<dyn std::error::Error>> {
    let mut selector = "latest".to_string();
    let mut all = false;
    let mut out = None;
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--selector" => {
                index += 1;
                selector = args
                    .get(index)
                    .ok_or("missing value for --selector")?
                    .to_string();
            }
            "--all" => {
                all = true;
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(
                    args.get(index).ok_or("missing value for --out")?,
                ));
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}").into()),
        }
        index += 1;
    }

    if all && out.is_some() {
        return Err("--out can only be used when generating a single selector".into());
    }

    Ok(Args { selector, all, out })
}

fn print_usage() {
    println!(
        "Usage: cargo run -p mercurio-tools --bin generate_sysml_legality_rulepack -- [--selector latest|2026-01|2026-04] [--all] [--out PATH]"
    );
}

fn merge_rulepacks(
    adapter: RulePack,
    legality: RulePack,
    metadata: BTreeMap<String, Value>,
) -> RulePack {
    let mut facts = BTreeSet::new();
    facts.extend(adapter.facts);
    facts.extend(legality.facts);

    let mut rules = adapter.rules;
    rules.extend(legality.rules);

    RulePack {
        id: "sysml.semantic_legality".to_string(),
        version: legality.version,
        metadata,
        facts: facts.into_iter().collect(),
        rules,
        diagnostics: legality.diagnostics,
    }
}

fn relative_path_string(root: &std::path::Path, path: &std::path::Path) -> String {
    let stable_path = path.strip_prefix(root).unwrap_or(path);
    stable_path.to_string_lossy().replace('\\', "/")
}
