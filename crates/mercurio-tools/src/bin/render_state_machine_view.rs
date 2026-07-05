use std::fs;
use std::path::PathBuf;

use mercurio_core::{Graph, KirDocument, KirElement, MetamodelAttributeRegistry};
use mercurio_sysml::{compile_sysml_text, load_sysml_baseline};
use mercurio_views::{
    DiagramDirectionDto, DiagramKindDto, DiagramLayoutOptionsDto, DiagramQueryOptionsDto,
    DiagramSpecDto, DiagramStyleOptionsDto, ViewDocumentDto, render_diagram, render_diagram_svg,
    validate_view_document,
};
use serde_json::Value;

const DEFAULT_STATE_MACHINE: &str = "VehicleControlExample.VehicleController.operatingMode";
const DEFAULT_OUTPUT_DIR: &str = "artifacts/views/state-machine/latest";
const SAMPLE_SOURCE_NAME: &str = "state-machine-view-demo.sysml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    fs::create_dir_all(&args.output_dir)?;

    let stdlib = load_sysml_baseline()?;
    let document = compile_sysml_text(SAMPLE_SYSML, SAMPLE_SOURCE_NAME, &stdlib)?;
    let root_id = resolve_state_machine_root(&document, &args.state_machine)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    let graph = Graph::from_document(document.clone())?;
    let registry = MetamodelAttributeRegistry::build(&graph);
    let spec = DiagramSpecDto {
        version: 1,
        kind: DiagramKindDto::StateMachine,
        title: format!("State Machine: {}", args.state_machine),
        description: Some(
            "Compiled SysML state machine rendered through the shared Mercurio view DTO path."
                .to_string(),
        ),
        root: Some(root_id.clone()),
        query: DiagramQueryOptionsDto {
            relations: Vec::new(),
            direction: DiagramDirectionDto::Children,
            depth: 4,
            include_libraries: false,
            include_user_model: true,
            max_nodes: 350,
            max_edges: 900,
        },
        layout: DiagramLayoutOptionsDto {
            engine: "dagre".to_string(),
            direction: args.direction,
        },
        style: DiagramStyleOptionsDto {
            show_attributes: false,
            show_edge_labels: true,
            group_by_layer: false,
        },
    };
    let view_document = ViewDocumentDto::diagram(spec.clone());
    validate_view_document(&view_document).map_err(format_view_validation_errors)?;
    let view = render_diagram(&graph, &registry, spec)?;

    fs::write(args.output_dir.join(SAMPLE_SOURCE_NAME), SAMPLE_SYSML)?;
    document.write_pretty_to_path(&args.output_dir.join("state-machine.kir.json"))?;
    fs::write(
        args.output_dir.join("state-machine.view.json"),
        format!("{}\n", serde_json::to_string_pretty(&view_document)?),
    )?;
    fs::write(
        args.output_dir.join("state-machine.render.json"),
        format!("{}\n", serde_json::to_string_pretty(&view)?),
    )?;
    fs::write(
        args.output_dir.join("state-machine.svg"),
        render_diagram_svg(&view),
    )?;

    println!(
        "source: {}",
        args.output_dir.join(SAMPLE_SOURCE_NAME).display()
    );
    println!(
        "state_machine: {} -> {}",
        args.state_machine,
        view.spec.root.as_deref().unwrap_or(&root_id)
    );
    println!("nodes: {}", view.nodes.len());
    println!("edges: {}", view.edges.len());
    println!("warnings: {}", view.warnings.len());
    println!(
        "svg: {}",
        args.output_dir.join("state-machine.svg").display()
    );

    Ok(())
}

struct Args {
    state_machine: String,
    output_dir: PathBuf,
    direction: String,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut state_machine = DEFAULT_STATE_MACHINE.to_string();
        let mut output_dir = PathBuf::from(DEFAULT_OUTPUT_DIR);
        let mut direction = "LR".to_string();
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--state-machine" | "--qualified-name" | "--root" => {
                    state_machine = args.next().ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("missing value for {arg}"),
                        )
                    })?;
                }
                "--out" | "--output-dir" => {
                    output_dir = PathBuf::from(args.next().ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("missing value for {arg}"),
                        )
                    })?);
                }
                "--direction" => {
                    direction = args.next().ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "missing value for --direction",
                        )
                    })?;
                    let normalized = direction.to_ascii_uppercase();
                    if !matches!(normalized.as_str(), "LR" | "RL" | "TB" | "BT") {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "--direction must be one of LR, RL, TB, BT",
                        )
                        .into());
                    }
                    direction = normalized;
                }
                "--help" | "-h" => {
                    println!(
                        "Usage: render_state_machine_view [--state-machine QUALIFIED_NAME] [--out DIR] [--direction LR|RL|TB|BT]"
                    );
                    std::process::exit(0);
                }
                _ if !arg.starts_with('-') => {
                    state_machine = arg;
                }
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("unknown argument {arg}"),
                    )
                    .into());
                }
            }
        }

        Ok(Self {
            state_machine,
            output_dir,
            direction,
        })
    }
}

fn resolve_state_machine_root(document: &KirDocument, root: &str) -> Result<String, String> {
    let trimmed = root.trim();
    if trimmed.is_empty() {
        return Err("state machine qualified name must not be empty".to_string());
    }

    if let Some(element) = document
        .elements
        .iter()
        .find(|element| element.id == trimmed && is_state_machine_element(element))
    {
        return Ok(
            resolve_state_definition_for_usage(document, element).unwrap_or(element.id.clone())
        );
    }

    let matches = document
        .elements
        .iter()
        .filter(|element| is_state_machine_element(element))
        .filter(|element| {
            element_qualified_name(element).is_some_and(|qualified_name| {
                qualified_name.eq_ignore_ascii_case(trimmed)
                    || qualified_name.ends_with(&format!(".{trimmed}"))
            })
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [element] => {
            Ok(resolve_state_definition_for_usage(document, element).unwrap_or(element.id.clone()))
        }
        [] => Err(format!(
            "state machine `{trimmed}` was not found. Available state-machine roots: {}",
            available_state_machine_roots(document).join(", ")
        )),
        _ => Err(format!(
            "state machine `{trimmed}` is ambiguous. Matches: {}",
            matches
                .iter()
                .filter_map(|element| element_qualified_name(element))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn available_state_machine_roots(document: &KirDocument) -> Vec<String> {
    let mut roots = document
        .elements
        .iter()
        .filter(|element| is_state_machine_element(element))
        .filter(|element| {
            is_state_definition(element)
                || document.elements.iter().any(|candidate| {
                    is_state_usage(candidate)
                        && string_property(candidate, "owner")
                            .is_some_and(|owner| owner == element.id)
                })
        })
        .filter_map(element_qualified_name)
        .collect::<Vec<_>>();
    roots.sort();
    roots
}

fn is_state_machine_element(element: &KirElement) -> bool {
    is_state_definition(element) || is_state_usage(element)
}

fn is_state_definition(element: &KirElement) -> bool {
    string_property(element, "metatype")
        .is_some_and(|metatype| metatype.contains("StateDefinition"))
        || element.kind.contains("StateDefinition")
}

fn is_state_usage(element: &KirElement) -> bool {
    string_property(element, "metatype").is_some_and(|metatype| metatype.contains("StateUsage"))
        || element.id.starts_with("state.")
}

fn resolve_state_definition_for_usage(
    document: &KirDocument,
    element: &KirElement,
) -> Option<String> {
    if is_state_definition(element) {
        return Some(element.id.clone());
    }

    let definition_id = string_property(element, "definition")?;
    document
        .elements
        .iter()
        .find(|candidate| candidate.id == definition_id && is_state_definition(candidate))
        .map(|candidate| candidate.id.clone())
}

fn element_qualified_name(element: &KirElement) -> Option<String> {
    string_property(element, "qualified_name")
        .or_else(|| string_property(element, "qualifiedName"))
        .map(ToOwned::to_owned)
        .or_else(|| {
            element
                .id
                .split_once('.')
                .map(|(_, qualified_name)| qualified_name.to_string())
        })
}

fn string_property<'a>(element: &'a KirElement, key: &str) -> Option<&'a str> {
    element.properties.get(key).and_then(Value::as_str)
}

fn format_view_validation_errors(
    diagnostics: Vec<mercurio_views::ViewValidationDiagnostic>,
) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                format!(
                    "{} {}: {}",
                    diagnostic.code, diagnostic.path, diagnostic.message
                )
            })
            .collect::<Vec<_>>()
            .join("; "),
    )
}

const SAMPLE_SYSML: &str = r#"package VehicleControlExample {
  state def OperatingMode {
    state off;
    state standby;
    state active;
    state faulted;

    transition power_on first off accept powerOn then standby;
    transition enable_drive from standby accept enableDrive then active;
    transition fault from active accept faultDetected then faulted;
    transition reset from faulted accept reset then standby;
    transition shutdown from standby accept shutdown then off;
  }

  part def VehicleControlSystem {
    part controller : VehicleController;
  }

  part def VehicleController {
    state operatingMode : OperatingMode;
  }
}
"#;
