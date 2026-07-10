use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use mercurio_core::{
    CapabilityRegistry, CapabilityRunRequest, CapabilityTarget, ElementRef, KirDocument,
    SemanticLegalityRequest, SemanticNextActionsRequest, SemanticWorkspaceSnapshot,
};
use mercurio_requirements::register_requirement_analysis_capability;
use mercurio_sysml::{
    register_sysml_behavior_capability, sysml_semantic_legality_service_for_release,
    sysml_semantic_next_actions_service_for_release,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug)]
struct Args {
    model: Option<PathBuf>,
    release: String,
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
    Legality {
        request: SemanticLegalityRequest,
    },
    NextActions {
        request: SemanticNextActionsRequest,
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
        Command::Legality { request } => {
            let report = sysml_semantic_legality_service_for_release(&args.release)?.check(request);
            print_json(&report)?;
        }
        Command::NextActions { request } => {
            let report = sysml_semantic_next_actions_service_for_release(&args.release)?
                .next_actions(request);
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
    let document = KirDocument::from_path(model)?;
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
    let mut release = "latest".to_string();
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
            "--release" => {
                let value = raw
                    .get(index + 1)
                    .ok_or_else(|| "--release requires a selector".to_string())?;
                release = value.clone();
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
            "legality" => {
                let options = parse_options(&raw[1..])?;
                Ok(Command::Legality {
                    request: legality_request_from_options(&options)?,
                })
            }
            "next-actions" => {
                let options = parse_options(&raw[1..])?;
                Ok(Command::NextActions {
                    request: next_actions_request_from_options(&options)?,
                })
            }
            _ => Err(usage()),
        })?;

    Ok(Args {
        model,
        release,
        command,
    })
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

fn next_actions_request_from_options(
    options: &BTreeMap<String, String>,
) -> Result<SemanticNextActionsRequest, String> {
    let element_kind = required_option(options, "element-kind")?;
    let max_actions = match options.get("max-actions") {
        Some(value) => Some(
            value
                .parse::<usize>()
                .map_err(|err| format!("invalid --max-actions: {err}"))?,
        ),
        None => None,
    };
    Ok(SemanticNextActionsRequest {
        element: options.get("element").cloned().map(ElementRef::new),
        element_kind,
        candidate_target_kinds: comma_list_option(options, "target-kinds"),
        candidate_targets: Vec::new(),
        candidate_attributes: comma_list_option(options, "attributes"),
        facts: Vec::new(),
        max_actions,
    })
}

fn comma_list_option(options: &BTreeMap<String, String>, key: &str) -> Vec<String> {
    options
        .get(key)
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn legality_request_from_options(
    options: &BTreeMap<String, String>,
) -> Result<SemanticLegalityRequest, String> {
    let operation = required_option(options, "operation")?;
    match operation.as_str() {
        "containment" => Ok(SemanticLegalityRequest::containment(
            required_option(options, "container-kind")?,
            required_option(options, "child-kind")?,
        )),
        "specialization" => Ok(SemanticLegalityRequest::specialization(
            required_option(options, "source-kind")?,
            required_option(options, "target-kind")?,
        )),
        "usage-typing" => Ok(SemanticLegalityRequest::usage_typing(
            required_option(options, "usage-kind")?,
            required_option(options, "definition-kind")?,
        )),
        "relationship" => Ok(SemanticLegalityRequest::relationship(
            required_option(options, "relationship-kind")?,
            required_option(options, "source-kind")?,
            required_option(options, "target-kind")?,
        )),
        "attribute-write" => Ok(SemanticLegalityRequest::attribute_write(
            required_option(options, "element-kind")?,
            required_option(options, "attribute")?,
        )),
        _ => Err(format!(
            "unknown legality operation `{operation}`; expected containment, specialization, usage-typing, relationship, or attribute-write"
        )),
    }
}

fn usage() -> String {
    [
        "usage:",
        "  capability_probe [--release <selector>] list",
        "  capability_probe --model <kir.json> readiness --capability <id> [--target <element-id> | --scope <scope-id>]",
        "  capability_probe --model <kir.json> run --capability <id> [--target <element-id> | --scope <scope-id>] [--parameters <json-object>]",
        "  capability_probe [--release <selector>] legality --operation containment --container-kind <kind> --child-kind <kind>",
        "  capability_probe [--release <selector>] legality --operation specialization --source-kind <kind> --target-kind <kind>",
        "  capability_probe [--release <selector>] legality --operation usage-typing --usage-kind <kind> --definition-kind <kind>",
        "  capability_probe [--release <selector>] legality --operation relationship --relationship-kind <kind> --source-kind <kind> --target-kind <kind>",
        "  capability_probe [--release <selector>] legality --operation attribute-write --element-kind <kind> --attribute <name>",
        "  capability_probe [--release <selector>] next-actions --element-kind <kind> [--element <qualified-name>] [--target-kinds <kind,kind>] [--attributes <name,name>] [--max-actions <n>]",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use mercurio_core::SemanticLegalityOperation;

    use super::*;

    #[test]
    fn parses_legality_relationship_command() {
        let args = parse_args(vec![
            "--release".to_string(),
            "2026-01".to_string(),
            "legality".to_string(),
            "--operation".to_string(),
            "relationship".to_string(),
            "--relationship-kind".to_string(),
            "satisfy".to_string(),
            "--source-kind".to_string(),
            "part".to_string(),
            "--target-kind".to_string(),
            "part".to_string(),
        ])
        .expect("args parse");

        assert_eq!(args.release, "2026-01");
        let Command::Legality { request } = args.command else {
            panic!("expected legality command");
        };
        assert_eq!(
            request.operation,
            SemanticLegalityOperation::Relationship {
                relationship_kind: "satisfy".to_string(),
                source_kind: "part".to_string(),
                target_kind: "part".to_string(),
            }
        );
    }

    #[test]
    fn parses_legality_attribute_write_command() {
        let args = parse_args(vec![
            "legality".to_string(),
            "--operation".to_string(),
            "attribute-write".to_string(),
            "--element-kind".to_string(),
            "requirement".to_string(),
            "--attribute".to_string(),
            "text".to_string(),
        ])
        .expect("args parse");

        assert_eq!(args.release, "latest");
        let Command::Legality { request } = args.command else {
            panic!("expected legality command");
        };
        assert_eq!(
            request.operation,
            SemanticLegalityOperation::AttributeWrite {
                kind: "requirement".to_string(),
                attribute: "text".to_string(),
            }
        );
    }

    #[test]
    fn parses_next_actions_command() {
        let args = parse_args(vec![
            "next-actions".to_string(),
            "--element".to_string(),
            "HybridVehicle.vehicle".to_string(),
            "--element-kind".to_string(),
            "part".to_string(),
            "--target-kinds".to_string(),
            "requirement, part".to_string(),
            "--attributes".to_string(),
            "id,text".to_string(),
            "--max-actions".to_string(),
            "12".to_string(),
        ])
        .expect("args parse");

        let Command::NextActions { request } = args.command else {
            panic!("expected next-actions command");
        };
        assert_eq!(
            request
                .element
                .as_ref()
                .map(|element| element.qualified_name.as_str()),
            Some("HybridVehicle.vehicle")
        );
        assert_eq!(request.element_kind, "part");
        assert_eq!(
            request.candidate_target_kinds,
            vec!["requirement".to_string(), "part".to_string()]
        );
        assert_eq!(
            request.candidate_attributes,
            vec!["id".to_string(), "text".to_string()]
        );
        assert_eq!(request.max_actions, Some(12));
    }
}
