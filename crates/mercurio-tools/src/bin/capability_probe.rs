use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use mercurio_core::{
    CapabilityRegistry, CapabilityRunRequest, CapabilityTarget, KirDocument,
    SemanticWorkspaceSnapshot,
};
use mercurio_requirements::register_requirement_analysis_capability;
use mercurio_sysml::register_sysml_behavior_capability;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug)]
struct Args {
    model: Option<PathBuf>,
    command: Command,
}

#[derive(Debug)]
enum Command {
    List,
    Readiness {
        capability: String,
        target: CapabilityTarget,
    },
    Run {
        capability: String,
        target: CapabilityTarget,
        parameters: BTreeMap<String, Value>,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ListOutput<T> {
    capabilities: Vec<T>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("capability_probe: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args(env::args().skip(1).collect())?;
    let registry = capability_registry()?;

    match args.command {
        Command::List => {
            print_json(&ListOutput {
                capabilities: registry.list(),
            })?;
        }
        Command::Readiness { capability, target } => {
            let workspace = load_workspace(args.model.as_ref())?;
            let report = registry.readiness(&workspace, &capability, &target)?;
            print_json(&report)?;
        }
        Command::Run {
            capability,
            target,
            parameters,
        } => {
            let workspace = load_workspace(args.model.as_ref())?;
            let request = CapabilityRunRequest {
                run_id: format!("probe.{capability}"),
                capability_id: capability,
                target,
                parameters,
                input_artifacts: Vec::new(),
            };
            let report = registry.run(&workspace, request)?;
            print_json(&report)?;
        }
    }

    Ok(())
}

fn capability_registry() -> Result<CapabilityRegistry, Box<dyn std::error::Error>> {
    let mut registry = CapabilityRegistry::with_foundation_builtins();
    register_sysml_behavior_capability(&mut registry)?;
    register_requirement_analysis_capability(&mut registry)?;
    Ok(registry)
}

fn load_workspace(
    model: Option<&PathBuf>,
) -> Result<SemanticWorkspaceSnapshot, Box<dyn std::error::Error>> {
    let model = model.ok_or("--model is required for readiness and run")?;
    let document = KirDocument::from_path_lenient(model)?;
    Ok(SemanticWorkspaceSnapshot::from_document_with_profile(
        document,
        Some("sysml".to_string()),
    )?)
}

fn print_json(value: &impl Serialize) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn parse_args(mut raw: Vec<String>) -> Result<Args, String> {
    if raw.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Err(usage());
    }

    let mut model = None;
    let mut index = 0;
    while index < raw.len() {
        match raw[index].as_str() {
            "--model" => {
                let value = raw
                    .get(index + 1)
                    .ok_or_else(|| "--model requires a path".to_string())?;
                model = Some(PathBuf::from(value));
                raw.drain(index..=index + 1);
            }
            _ => index += 1,
        }
    }

    let command = raw
        .first()
        .ok_or_else(usage)
        .and_then(|command| match command.as_str() {
            "list" => Ok(Command::List),
            "readiness" => {
                let options = parse_options(&raw[1..])?;
                Ok(Command::Readiness {
                    capability: required_option(&options, "capability")?,
                    target: target_from_options(&options)?,
                })
            }
            "run" => {
                let options = parse_options(&raw[1..])?;
                Ok(Command::Run {
                    capability: required_option(&options, "capability")?,
                    target: target_from_options(&options)?,
                    parameters: parameters_from_options(&options)?,
                })
            }
            _ => Err(usage()),
        })?;

    Ok(Args { model, command })
}

fn parse_options(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut options = BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let key = args[index].strip_prefix("--").ok_or_else(|| {
            format!(
                "expected option starting with `--`, got `{}`\n{}",
                args[index],
                usage()
            )
        })?;
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("--{key} requires a value"))?;
        options.insert(key.to_string(), value.clone());
        index += 2;
    }
    Ok(options)
}

fn required_option(options: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    options
        .get(key)
        .cloned()
        .ok_or_else(|| format!("--{key} is required\n{}", usage()))
}

fn target_from_options(options: &BTreeMap<String, String>) -> Result<CapabilityTarget, String> {
    match (options.get("target"), options.get("scope")) {
        (Some(_), Some(_)) => Err("--target and --scope are mutually exclusive".to_string()),
        (Some(element_id), None) => Ok(CapabilityTarget::Element {
            element_id: element_id.clone(),
        }),
        (None, Some(scope_id)) => Ok(CapabilityTarget::Scope {
            scope_id: scope_id.clone(),
        }),
        (None, None) => Ok(CapabilityTarget::Workspace),
    }
}

fn parameters_from_options(
    options: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Value>, String> {
    let Some(input) = options.get("parameters") else {
        return Ok(BTreeMap::new());
    };
    let value: Value =
        serde_json::from_str(input).map_err(|err| format!("invalid --parameters JSON: {err}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "--parameters must be a JSON object".to_string())?;
    Ok(object
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect())
}

fn usage() -> String {
    [
        "usage:",
        "  capability_probe --model <kir.json> list",
        "  capability_probe --model <kir.json> readiness --capability <id> [--target <element-id> | --scope <scope-id>]",
        "  capability_probe --model <kir.json> run --capability <id> [--target <element-id> | --scope <scope-id>] [--parameters <json-object>]",
    ]
    .join("\n")
}
