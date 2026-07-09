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

const DEFAULT_SCOPE: &str = "VehicleRequirementExample";
const DEFAULT_OUTPUT_DIR: &str = "artifacts/views/requirement/latest";
const SAMPLE_SOURCE_NAME: &str = "requirement-view-demo.sysml";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse()?;
    fs::create_dir_all(&args.output_dir)?;

    let stdlib = load_sysml_baseline()?;
    let document = compile_sysml_text(SAMPLE_SYSML, SAMPLE_SOURCE_NAME, &stdlib)?;
    let root_id = resolve_requirement_scope(&document, &args.scope)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidInput, message))?;
    let graph = Graph::from_document(document.clone())?;
    let registry = MetamodelAttributeRegistry::build(&graph);
    let spec = DiagramSpecDto {
        version: 1,
        kind: DiagramKindDto::Requirement,
        title: format!("Requirements: {}", args.scope),
        description: Some(
            "Compiled SysML requirement diagram rendered through the shared Mercurio view DTO path."
                .to_string(),
        ),
        root: Some(root_id.clone()),
        query: DiagramQueryOptionsDto {
            relations: Vec::new(),
            direction: DiagramDirectionDto::Children,
            depth: 4,
            include_libraries: true,
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
            show_affordances: false,
        },
    };
    let view_document = ViewDocumentDto::diagram(spec.clone());
    validate_view_document(&view_document).map_err(format_view_validation_errors)?;
    let view = render_diagram(&graph, &registry, spec)?;

    fs::write(args.output_dir.join(SAMPLE_SOURCE_NAME), SAMPLE_SYSML)?;
    document.write_pretty_to_path(&args.output_dir.join("requirement.kir.json"))?;
    fs::write(
        args.output_dir.join("requirement.view.json"),
        format!("{}\n", serde_json::to_string_pretty(&view_document)?),
    )?;
    fs::write(
        args.output_dir.join("requirement.render.json"),
        format!("{}\n", serde_json::to_string_pretty(&view)?),
    )?;
    fs::write(
        args.output_dir.join("requirement.svg"),
        render_diagram_svg(&view),
    )?;

    println!(
        "source: {}",
        args.output_dir.join(SAMPLE_SOURCE_NAME).display()
    );
    println!(
        "requirement scope: {} -> {}",
        args.scope,
        view.spec.root.as_deref().unwrap_or(&root_id)
    );
    println!("nodes: {}", view.nodes.len());
    println!("edges: {}", view.edges.len());
    println!("warnings: {}", view.warnings.len());
    println!("svg: {}", args.output_dir.join("requirement.svg").display());

    Ok(())
}

struct Args {
    scope: String,
    output_dir: PathBuf,
    direction: String,
}

impl Args {
    fn parse() -> Result<Self, Box<dyn std::error::Error>> {
        let mut scope = DEFAULT_SCOPE.to_string();
        let mut output_dir = PathBuf::from(DEFAULT_OUTPUT_DIR);
        let mut direction = "LR".to_string();
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--requirement" | "--scope" | "--qualified-name" | "--root" => {
                    scope = args.next().ok_or_else(|| {
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
                        "Usage: render_requirement_view [--scope QUALIFIED_NAME] [--out DIR] [--direction LR|RL|TB|BT]"
                    );
                    std::process::exit(0);
                }
                _ if !arg.starts_with('-') => {
                    scope = arg;
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
            scope,
            output_dir,
            direction,
        })
    }
}

fn resolve_requirement_scope(document: &KirDocument, root: &str) -> Result<String, String> {
    let trimmed = root.trim();
    if trimmed.is_empty() {
        return Err("requirement diagram scope must not be empty".to_string());
    }

    if let Some(element) = document
        .elements
        .iter()
        .find(|element| element.id == trimmed && is_requirement_scope_element(element))
    {
        return Ok(element.id.clone());
    }

    let matches = document
        .elements
        .iter()
        .filter(|element| is_requirement_scope_element(element))
        .filter(|element| {
            element_qualified_name(element).is_some_and(|qualified_name| {
                qualified_name.eq_ignore_ascii_case(trimmed)
                    || qualified_name.ends_with(&format!(".{trimmed}"))
            })
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [element] => Ok(element.id.clone()),
        [] => Err(format!(
            "requirement scope `{trimmed}` was not found. Available requirement scopes: {}",
            available_requirement_scopes(document).join(", ")
        )),
        _ => Err(format!(
            "requirement scope `{trimmed}` is ambiguous. Matches: {}",
            matches
                .iter()
                .filter_map(|element| element_qualified_name(element))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn available_requirement_scopes(document: &KirDocument) -> Vec<String> {
    let mut roots = document
        .elements
        .iter()
        .filter(|element| is_requirement_scope_element(element))
        .filter_map(element_qualified_name)
        .collect::<Vec<_>>();
    roots.sort();
    roots
}

fn is_requirement_scope_element(element: &KirElement) -> bool {
    let semantic = semantic_text(element);
    semantic.contains("package")
        || semantic.contains("requirementdefinition")
        || semantic.contains("requirementusage")
}

fn semantic_text(element: &KirElement) -> String {
    let mut text = element.kind.to_ascii_lowercase();
    for value in [
        string_property(element, "metatype"),
        element
            .properties
            .get("metadata")
            .and_then(|metadata| metadata.get("lowering"))
            .and_then(|lowering| lowering.get("construct"))
            .and_then(Value::as_str),
        element
            .properties
            .get("metadata")
            .and_then(|metadata| metadata.get("lowering"))
            .and_then(|lowering| lowering.get("metaclass"))
            .and_then(Value::as_str),
    ]
    .into_iter()
    .flatten()
    {
        text.push(' ');
        text.push_str(&value.to_ascii_lowercase());
    }
    text
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

const SAMPLE_SYSML: &str = r#"package VehicleRequirementExample {
  part def VehicleController;

  requirement <'REQ-START-001'> safeStart {
    doc /* Vehicle shall prevent unsafe starts. */
  }

  part controller : VehicleController {
    satisfy VehicleRequirementExample.safeStart;
  }
}
"#;
