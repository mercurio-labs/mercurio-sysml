use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mercurio_core::graph::{Element, ElementProperties, GraphArtifact};
use mercurio_core::{
    Edge, Graph, KirDocument, KirElement, MetamodelAttributeRegistry, default_stdlib_path,
};
use mercurio_sysml::compile_sysml_text;
use mercurio_views::{
    DiagramDirectionDto, DiagramKindDto, DiagramLayoutOptionsDto, DiagramQueryOptionsDto,
    DiagramRenderRequestDto, DiagramSpecDto, DiagramStyleOptionsDto, DiagramSymbolDto,
    DiagramViewDto, TableColumnSpecDto, TableKindDto, TableSpecDto, TableViewDto, ViewDocumentDto,
    render_diagram, render_table, validate_view_document,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("artifacts/views/latest"));
    fs::create_dir_all(&output_dir)?;

    let graph = sample_graph()?;
    let registry = MetamodelAttributeRegistry::build(&graph);

    let views = [
        (
            "structure",
            DiagramSpecDto {
                version: 1,
                kind: DiagramKindDto::Structure,
                title: "Vehicle Structure".to_string(),
                description: Some("Sample package containment rendered through the current structure renderer.".to_string()),
                root: Some("pkg.Vehicle".to_string()),
                query: query(
                    vec!["owner", "part"],
                    DiagramDirectionDto::Children,
                    3,
                    false,
                    true,
                    350,
                    900,
                ),
                layout: layout("LR"),
                style: DiagramStyleOptionsDto::default(),
            },
        ),
        (
            "specialization",
            DiagramSpecDto {
                version: 1,
                kind: DiagramKindDto::Structure,
                title: "Vehicle Specialization".to_string(),
                description: Some("Sample specialization view rooted at PartDefinition.".to_string()),
                root: Some("SysML::Systems::PartDefinition".to_string()),
                query: query(vec!["specializes"], DiagramDirectionDto::Children, 2, true, true, 350, 900),
                layout: layout("TB"),
                style: DiagramStyleOptionsDto::default(),
            },
        ),
        (
            "state-machine-outline",
            DiagramSpecDto {
                version: 1,
                kind: DiagramKindDto::Structure,
                title: "DriveMode State Machine Outline".to_string(),
                description: Some("Temporary state-machine outline until a dedicated state_machine view kind exists.".to_string()),
                root: Some("state.Vehicle.DriveMode".to_string()),
                query: query(vec!["owner", "source", "target"], DiagramDirectionDto::Children, 2, false, true, 350, 900),
                layout: layout("LR"),
                style: DiagramStyleOptionsDto::default(),
            },
        ),
        (
            "activity-provide-power",
            DiagramSpecDto {
                version: 1,
                kind: DiagramKindDto::Activity,
                title: "Provide Power Activity".to_string(),
                description: Some("Sample activity diagram family with action, object node, and flow symbols.".to_string()),
                root: Some("activity.Vehicle.ProvidePower".to_string()),
                query: query(Vec::new(), DiagramDirectionDto::Children, 3, false, true, 350, 900),
                layout: layout("LR"),
                style: DiagramStyleOptionsDto::default(),
            },
        ),
    ];

    for (slug, spec) in views {
        let view = render_diagram(&graph, &registry, spec.clone())?;
        write_view_spec(&output_dir, slug, &spec)?;
        write_render(&output_dir, slug, &view)?;
        write_svg(&output_dir, slug, &view)?;
        write_svg_variant(&output_dir, slug, "arrows", &view)?;
        println!(
            "{}: nodes={} edges={} warnings={}",
            slug,
            view.nodes.len(),
            view.edges.len(),
            view.warnings.len()
        );
    }
    let requirements_table_spec = TableSpecDto {
        version: 1,
        kind: TableKindDto::Requirements,
        title: "Vehicle Requirements".to_string(),
        description: Some("Requirements table rendered from KIR graph properties.".to_string()),
        root: Some("pkg.Vehicle".to_string()),
        target_type: None,
        query: query(
            vec!["owner"],
            DiagramDirectionDto::Children,
            2,
            false,
            true,
            350,
            900,
        ),
        columns: vec![
            table_column("requirement_id", "ID", None),
            table_column("name", "Name", None),
            table_column("text", "Text", None),
            table_column(
                "status",
                "Status",
                Some("metadata[RequirementLifecycle].status"),
            ),
            table_column(
                "lifecycle_owner",
                "Lifecycle Owner",
                Some("metadata[RequirementLifecycle].owner"),
            ),
            table_column(
                "review_date",
                "Review Date",
                Some("metadata[RequirementLifecycle].reviewDate"),
            ),
            table_column("owner_name", "Model Owner", Some("owner.declared_name")),
        ],
    };
    let requirements_table = render_table(&graph, &registry, requirements_table_spec.clone())?;
    write_table_spec(
        &output_dir,
        "requirements",
        &requirements_table_spec,
        &requirements_table,
    )?;
    write_table_html(&output_dir, "requirements", &requirements_table)?;
    write_table_svg(&output_dir, "requirements", &requirements_table)?;
    write_requirement_metadata_demo(&output_dir)?;
    println!(
        "requirements-table: rows={} columns={} warnings={}",
        requirements_table.rows.len(),
        requirements_table.columns.len(),
        requirements_table.warnings.len()
    );
    write_activity_swimlane_demo(&output_dir)?;

    Ok(())
}

fn sample_graph() -> Result<Graph, Box<dyn std::error::Error>> {
    Ok(Graph::from_artifact(GraphArtifact {
        elements: vec![
            element(0, "SysML::Systems::PartDefinition", "SysML::Metaclass", 1),
            element(1, "pkg.Vehicle", "SysML::Package", 2),
            element(2, "type.Vehicle.Car", "SysML::Systems::PartDefinition", 2),
            element(
                3,
                "type.Vehicle.Engine",
                "SysML::Systems::PartDefinition",
                2,
            ),
            element(
                4,
                "req.Vehicle.SafeStart",
                "SysML::Requirements::RequirementUsage",
                2,
            ),
            element(
                5,
                "state.Vehicle.DriveMode",
                "SysML::States::StateDefinition",
                2,
            ),
            element(
                6,
                "state.Vehicle.DriveMode.Off",
                "SysML::States::StateUsage",
                2,
            ),
            element(
                7,
                "state.Vehicle.DriveMode.Starting",
                "SysML::States::StateUsage",
                2,
            ),
            element(
                8,
                "state.Vehicle.DriveMode.Running",
                "SysML::States::StateUsage",
                2,
            ),
            element(
                9,
                "state.Vehicle.DriveMode.Fault",
                "SysML::States::StateUsage",
                2,
            ),
            element(
                10,
                "transition.Vehicle.DriveMode.start",
                "SysML::States::TransitionUsage",
                2,
            ),
            element(
                11,
                "transition.Vehicle.DriveMode.ready",
                "SysML::States::TransitionUsage",
                2,
            ),
            element(
                12,
                "transition.Vehicle.DriveMode.fail",
                "SysML::States::TransitionUsage",
                2,
            ),
            element(
                13,
                "activity.Vehicle.ProvidePower",
                "SysML::Actions::ActivityDefinition",
                2,
            ),
            element(
                14,
                "action.Vehicle.ProvidePower.ProportionPower",
                "SysML::Actions::ActionUsage",
                2,
            ),
            element(
                15,
                "action.Vehicle.ProvidePower.ProvideGasPower",
                "SysML::Actions::ActionUsage",
                2,
            ),
            element(
                16,
                "object.Vehicle.ProvidePower.battCond",
                "SysML::Actions::ObjectNode",
                2,
            ),
            element(
                17,
                "object.Vehicle.ProvidePower.gThrottle",
                "SysML::Actions::ObjectNode",
                2,
            ),
        ],
        edges: vec![
            edge(2, 0, "specializes"),
            edge(3, 0, "specializes"),
            edge(2, 3, "part"),
            edge(2, 1, "owner"),
            edge(3, 1, "owner"),
            edge(4, 1, "owner"),
            edge(5, 1, "owner"),
            edge(6, 5, "owner"),
            edge(7, 5, "owner"),
            edge(8, 5, "owner"),
            edge(9, 5, "owner"),
            edge(10, 5, "owner"),
            edge(11, 5, "owner"),
            edge(12, 5, "owner"),
            edge(10, 6, "source"),
            edge(10, 7, "target"),
            edge(11, 7, "source"),
            edge(11, 8, "target"),
            edge(12, 8, "source"),
            edge(12, 9, "target"),
            edge(13, 1, "owner"),
            edge(14, 13, "owner"),
            edge(15, 13, "owner"),
            edge(16, 13, "owner"),
            edge(17, 13, "owner"),
            edge(16, 14, "object_flow"),
            edge(14, 17, "object_flow"),
            edge(17, 15, "object_flow"),
            edge(14, 15, "control_flow"),
        ],
    })?)
}

fn element(id: u32, element_id: &str, kind: &str, layer: u8) -> Element {
    let mut properties = BTreeMap::new();
    properties.insert("declared_name".to_string(), json!(label_for_id(element_id)));
    if kind.contains("Requirement") {
        properties.insert("requirement_id".to_string(), json!("REQ-001"));
        properties.insert(
            "text".to_string(),
            json!("Vehicle shall prevent unsafe starts."),
        );
        properties.insert(
            "metadata".to_string(),
            json!({
                "RequirementLifecycle": {
                    "properties": {
                        "status": "approved",
                        "owner": "Safety Team",
                        "reviewDate": "2026-05-27"
                    }
                }
            }),
        );
    }
    Element {
        id,
        element_id: element_id.to_string(),
        kind: Arc::from(kind),
        layer,
        properties: ElementProperties::from_declared_arc_for_artifact(
            element_id.to_string(),
            properties
                .into_iter()
                .map(|(key, value)| (Arc::from(key), value))
                .collect(),
        ),
    }
}

fn edge(source: u32, target: u32, relation: &str) -> Edge {
    Edge {
        source,
        target,
        relation: Arc::from(relation),
    }
}

fn query(
    relations: Vec<&str>,
    direction: DiagramDirectionDto,
    depth: usize,
    include_libraries: bool,
    include_user_model: bool,
    max_nodes: usize,
    max_edges: usize,
) -> DiagramQueryOptionsDto {
    DiagramQueryOptionsDto {
        relations: relations.into_iter().map(str::to_string).collect(),
        direction,
        depth,
        include_libraries,
        include_user_model,
        max_nodes,
        max_edges,
    }
}

fn layout(direction: &str) -> DiagramLayoutOptionsDto {
    DiagramLayoutOptionsDto {
        engine: "dagre".to_string(),
        direction: direction.to_string(),
    }
}

fn table_column(key: &str, label: &str, path: Option<&str>) -> TableColumnSpecDto {
    TableColumnSpecDto {
        key: key.to_string(),
        label: label.to_string(),
        path: path.map(str::to_string),
    }
}

fn write_requirement_metadata_demo(output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        output_dir.join("requirements-metadata-demo.sysml"),
        REQUIREMENTS_METADATA_SYSML,
    )?;
    Ok(())
}

fn write_view_spec(
    output_dir: &Path,
    slug: &str,
    spec: &DiagramSpecDto,
) -> Result<(), Box<dyn std::error::Error>> {
    let wrapper = ViewDocumentDto::diagram(spec.clone());
    validate_view_document(&wrapper).map_err(format_view_validation_errors)?;
    let request = DiagramRenderRequestDto { spec: spec.clone() };
    fs::write(
        output_dir.join(format!("{slug}.view.json")),
        format!("{}\n", serde_json::to_string_pretty(&wrapper)?),
    )?;
    fs::write(
        output_dir.join(format!("{slug}.diagram-request.json")),
        format!("{}\n", serde_json::to_string_pretty(&request)?),
    )?;
    Ok(())
}

fn write_render(
    output_dir: &Path,
    slug: &str,
    view: &DiagramViewDto,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        output_dir.join(format!("{slug}.render.json")),
        format!("{}\n", serde_json::to_string_pretty(view)?),
    )?;
    Ok(())
}

fn write_table_spec(
    output_dir: &Path,
    slug: &str,
    spec: &TableSpecDto,
    view: &TableViewDto,
) -> Result<(), Box<dyn std::error::Error>> {
    let wrapper = ViewDocumentDto::table(spec.clone());
    validate_view_document(&wrapper).map_err(format_view_validation_errors)?;
    fs::write(
        output_dir.join(format!("{slug}.table.view.json")),
        format!("{}\n", serde_json::to_string_pretty(&wrapper)?),
    )?;
    fs::write(
        output_dir.join(format!("{slug}.table.render.json")),
        format!("{}\n", serde_json::to_string_pretty(view)?),
    )?;
    Ok(())
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

fn write_table_html(
    output_dir: &Path,
    slug: &str,
    view: &TableViewDto,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        output_dir.join(format!("{slug}.table.html")),
        render_table_html(view),
    )?;
    Ok(())
}

fn write_table_svg(
    output_dir: &Path,
    slug: &str,
    view: &TableViewDto,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        output_dir.join(format!("{slug}.table.svg")),
        render_table_svg(view),
    )?;
    Ok(())
}

fn write_svg(
    output_dir: &Path,
    slug: &str,
    view: &DiagramViewDto,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(output_dir.join(format!("{slug}.svg")), render_svg(view))?;
    Ok(())
}

fn write_svg_variant(
    output_dir: &Path,
    slug: &str,
    variant: &str,
    view: &DiagramViewDto,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        output_dir.join(format!("{slug}-{variant}.svg")),
        render_svg(view),
    )?;
    Ok(())
}

fn render_table_html(view: &TableViewDto) -> String {
    let mut html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8"/>
<title>{}</title>
<style>
body {{ margin: 0; padding: 32px; background: #f8fafc; color: #0f172a; font-family: Segoe UI, Arial, sans-serif; }}
h1 {{ margin: 0 0 18px; font-size: 22px; }}
table {{ border-collapse: collapse; width: 100%; background: #fff; border: 1px solid #cbd5e1; }}
th, td {{ border-bottom: 1px solid #e2e8f0; border-right: 1px solid #e2e8f0; padding: 10px 12px; text-align: left; vertical-align: top; font-size: 13px; }}
th {{ background: #e0f2fe; font-weight: 700; }}
tr:last-child td {{ border-bottom: 0; }}
td:first-child {{ font-weight: 700; white-space: nowrap; }}
.empty {{ padding: 16px; background: #fff; border: 1px solid #cbd5e1; }}
</style>
</head>
<body>
<h1>{}</h1>
"#,
        escape_xml(&view.spec.title),
        escape_xml(&view.spec.title)
    );

    if view.rows.is_empty() {
        html.push_str(r#"<div class="empty">No requirements matched.</div>"#);
    } else {
        html.push_str("<table><thead><tr>");
        for column in &view.columns {
            html.push_str(&format!("<th>{}</th>", escape_xml(&column.label)));
        }
        html.push_str("</tr></thead><tbody>");
        for row in &view.rows {
            html.push_str("<tr>");
            for cell in &row.cells {
                html.push_str(&format!("<td>{}</td>", escape_xml(&cell.value)));
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody></table>");
    }

    html.push_str("</body></html>\n");
    html
}

fn render_table_svg(view: &TableViewDto) -> String {
    let column_widths = [120usize, 180, 430, 120, 180];
    let row_height = 58usize;
    let header_height = 42usize;
    let title_height = 54usize;
    let margin = 28usize;
    let table_width = column_widths.iter().sum::<usize>();
    let table_height = header_height + view.rows.len().max(1) * row_height;
    let width = table_width + margin * 2;
    let height = title_height + table_height + margin;
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-label="{}">
<rect width="100%" height="100%" fill="#f8fafc"/>
<text x="{margin}" y="34" font-family="Segoe UI, Arial, sans-serif" font-size="22" font-weight="700" fill="#0f172a">{}</text>
<rect x="{margin}" y="{title_height}" width="{table_width}" height="{table_height}" fill="#ffffff" stroke="#cbd5e1" stroke-width="1"/>
"##,
        escape_xml(&view.spec.title),
        escape_xml(&view.spec.title)
    );

    let mut x = margin;
    for (index, column) in view.columns.iter().enumerate() {
        let column_width = column_widths.get(index).copied().unwrap_or(160);
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{title_height}" width="{column_width}" height="{header_height}" fill="#e0f2fe" stroke="#cbd5e1" stroke-width="1"/>
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="13" font-weight="700" fill="#0f172a">{}</text>
"##,
            x + 10,
            title_height + 26,
            escape_xml(&column.label)
        ));
        x += column_width;
    }

    if view.rows.is_empty() {
        svg.push_str(&format!(
            r##"<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="13" fill="#475569">No requirements matched.</text>
"##,
            margin + 12,
            title_height + header_height + 34
        ));
    } else {
        for (row_index, row) in view.rows.iter().enumerate() {
            let y = title_height + header_height + row_index * row_height;
            let mut x = margin;
            for (cell_index, cell) in row.cells.iter().enumerate() {
                let column_width = column_widths.get(cell_index).copied().unwrap_or(160);
                svg.push_str(&format!(
                    r##"<rect x="{x}" y="{y}" width="{column_width}" height="{row_height}" fill="#ffffff" stroke="#e2e8f0" stroke-width="1"/>
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="13" fill="#0f172a">{}</text>
"##,
                    x + 10,
                    y + 34,
                    truncate(&cell.value, column_width / 8)
                ));
                x += column_width;
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn render_svg(view: &DiagramViewDto) -> String {
    let node_width = 220usize;
    let node_height = 70usize;
    let gap_x = 70usize;
    let gap_y = 55usize;
    let margin = 32usize;
    let title_height = 48usize;
    let auto_layout = auto_layout_positions(
        view,
        node_width,
        node_height,
        gap_x,
        gap_y,
        margin,
        title_height,
    );
    let width = auto_layout.width;
    let height = auto_layout.height;
    let positions = auto_layout.positions;
    let symbols_by_id = view
        .symbols
        .iter()
        .map(|symbol| (symbol.id.as_str(), symbol))
        .collect::<BTreeMap<_, _>>();

    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-label="{}">
<rect width="100%" height="100%" fill="#f8fafc"/>
<text x="{margin}" y="30" font-family="Segoe UI, Arial, sans-serif" font-size="18" font-weight="700" fill="#0f172a">{}</text>
"##,
        escape_xml(&view.spec.title),
        escape_xml(&view.spec.title)
    );

    let mut rendered_edges = Vec::new();
    for edge in &view.edges {
        let Some((source_x, source_y)) = positions.get(edge.source.as_str()) else {
            continue;
        };
        let Some((target_x, target_y)) = positions.get(edge.target.as_str()) else {
            continue;
        };
        let source_center = (source_x + node_width / 2, source_y + node_height / 2);
        let target_center = (target_x + node_width / 2, target_y + node_height / 2);
        let (x1, y1) = rectangle_boundary_point(
            source_center,
            target_center,
            node_width / 2,
            node_height / 2,
        );
        let (x2, y2) = rectangle_boundary_point(
            target_center,
            source_center,
            node_width / 2,
            node_height / 2,
        );
        let symbol = symbols_by_id.get(edge.symbol.as_str()).copied();
        let route = symbol_property(symbol, "route")
            .unwrap_or_else(|| default_route(edge.relation.as_str()).to_string());
        let path = routed_path(&route, x1, y1, x2, y2);
        rendered_edges.push((edge.relation.clone(), edge.symbol.clone(), x1, y1, x2, y2));
        svg.push_str(&format!(
            r##"<path d="{}" fill="none" stroke="#475569" stroke-width="1.5"/>
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="12" font-weight="600" fill="#334155">{}</text>
"##,
            path,
            (x1 + x2) / 2 + 6,
            (y1 + y2) / 2 - 10,
            escape_xml(&edge.label)
        ));
    }

    for node in &view.nodes {
        let Some((x, y)) = positions.get(node.id.as_str()) else {
            continue;
        };
        let symbol = symbols_by_id.get(node.symbol.as_str()).copied();
        let role = symbol
            .map(|symbol| symbol.role.as_str())
            .unwrap_or("element");
        let shape = symbol_property(symbol, "shape").unwrap_or_else(|| "node".to_string());
        svg.push_str(&node_shape_svg(
            role,
            &shape,
            *x,
            *y,
            node_width,
            node_height,
        ));
        svg.push_str(&format!(
            r##"
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="14" font-weight="700" fill="#0f172a">{}</text>
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="11" fill="#475569">{}</text>
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="10" fill="#64748b">{}</text>
"##,
            x + 12,
            y + 24,
            truncate(&node.label, 24),
            x + 12,
            y + 44,
            truncate(&node.kind, 30),
            x + 12,
            y + 60,
            escape_xml(&node.badges.join(" "))
        ));
    }

    for (relation, symbol_id, x1, y1, x2, y2) in rendered_edges {
        let symbol = symbols_by_id.get(symbol_id.as_str()).copied();
        let target_decoration = symbol_property(symbol, "target_decoration")
            .unwrap_or_else(|| default_target_decoration(relation.as_str()).to_string());
        let source_decoration = symbol_property(symbol, "source_decoration")
            .unwrap_or_else(|| default_source_decoration(relation.as_str()).to_string());
        svg.push_str(&target_decoration_svg(&target_decoration, x1, y1, x2, y2));
        svg.push_str(&source_decoration_svg(&source_decoration, x1, y1, x2, y2));
    }

    svg.push_str("</svg>\n");
    svg
}

fn node_shape_svg(
    role: &str,
    shape: &str,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> String {
    if role == "frame" || shape == "activity_frame" {
        return format!(
            r##"<rect x="{x}" y="{y}" width="{width}" height="{height}" rx="0" fill="#ffffff" stroke="#0f172a" stroke-width="1.5"/>
"##
        );
    }

    if role == "action" || shape == "action" {
        return format!(
            r##"<rect x="{x}" y="{y}" width="{width}" height="{height}" rx="10" fill="#fed7aa" stroke="#0f172a" stroke-width="1.5"/>
"##
        );
    }

    if role == "object_node" || shape == "object_node" {
        return format!(
            r##"<rect x="{x}" y="{y}" width="{width}" height="{height}" rx="4" fill="#bae6fd" stroke="#0f172a" stroke-width="1.5"/>
"##
        );
    }

    if role == "decision" || role == "merge" || shape == "decision" {
        let cx = x + width / 2;
        let cy = y + height / 2;
        return format!(
            r##"<path d="M {cx} {y} L {} {cy} L {cx} {} L {x} {cy} Z" fill="#d9f99d" stroke="#0f172a" stroke-width="1.5"/>
"##,
            x + width,
            y + height
        );
    }

    format!(
        r##"<rect x="{x}" y="{y}" width="{width}" height="{height}" rx="8" fill="#ffffff" stroke="#2563eb" stroke-width="1.5"/>
"##
    )
}

fn symbol_property(symbol: Option<&DiagramSymbolDto>, key: &str) -> Option<String> {
    symbol
        .and_then(|symbol| symbol.properties.get(key))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn routed_path(route: &str, x1: isize, y1: isize, x2: isize, y2: isize) -> String {
    if route == "elbow" || route == "orthogonal" {
        let mid_x = (x1 + x2) / 2;
        return format!("M {x1} {y1} L {mid_x} {y1} L {mid_x} {y2} L {x2} {y2}");
    }

    if route == "curve" {
        let mid_x = (x1 + x2) / 2;
        return format!("M {x1} {y1} C {mid_x} {y1}, {mid_x} {y2}, {x2} {y2}");
    }

    format!("M {x1} {y1} L {x2} {y2}")
}

struct AutoLayout {
    positions: BTreeMap<String, (usize, usize)>,
    width: usize,
    height: usize,
}

fn auto_layout_positions(
    view: &DiagramViewDto,
    node_width: usize,
    node_height: usize,
    gap_x: usize,
    gap_y: usize,
    margin: usize,
    title_height: usize,
) -> AutoLayout {
    let levels = layout_levels(view);
    let mut by_level = BTreeMap::<usize, Vec<String>>::new();
    for node in &view.nodes {
        by_level
            .entry(*levels.get(&node.id).unwrap_or(&0))
            .or_default()
            .push(node.id.clone());
    }
    for ids in by_level.values_mut() {
        ids.sort_by_key(|id| {
            view.nodes
                .iter()
                .find(|node| node.id == *id)
                .map(|node| (node.kind.clone(), node.label.clone(), node.id.clone()))
        });
    }

    let direction = view.spec.layout.direction.to_ascii_uppercase();
    let horizontal = direction != "TB" && direction != "BT";
    let max_lanes = by_level.values().map(Vec::len).max().unwrap_or(1);
    let level_count = by_level.len().max(1);
    let mut positions = BTreeMap::new();

    for (level, ids) in &by_level {
        for (lane, id) in ids.iter().enumerate() {
            let logical_level = if direction == "BT" || direction == "RL" {
                level_count.saturating_sub(1).saturating_sub(*level)
            } else {
                *level
            };
            let (x, y) = if horizontal {
                (
                    margin + logical_level * (node_width + gap_x),
                    title_height + margin + lane * (node_height + gap_y),
                )
            } else {
                (
                    margin + lane * (node_width + gap_x),
                    title_height + margin + logical_level * (node_height + gap_y),
                )
            };
            positions.insert(id.clone(), (x, y));
        }
    }

    let width = if horizontal {
        margin * 2 + level_count * node_width + level_count.saturating_sub(1) * gap_x
    } else {
        margin * 2 + max_lanes * node_width + max_lanes.saturating_sub(1) * gap_x
    };
    let height = if horizontal {
        title_height + margin * 2 + max_lanes * node_height + max_lanes.saturating_sub(1) * gap_y
    } else {
        title_height
            + margin * 2
            + level_count * node_height
            + level_count.saturating_sub(1) * gap_y
    };

    AutoLayout {
        positions,
        width,
        height,
    }
}

fn layout_levels(view: &DiagramViewDto) -> BTreeMap<String, usize> {
    let node_ids = view
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    let mut children_by_parent = BTreeMap::<String, Vec<String>>::new();
    let mut child_ids = BTreeSet::new();

    for edge in &view.edges {
        if !node_ids.contains(&edge.source) || !node_ids.contains(&edge.target) {
            continue;
        }
        let (parent, child) = layout_parent_child(
            edge.relation.as_str(),
            edge.source.as_str(),
            edge.target.as_str(),
        );
        children_by_parent
            .entry(parent.to_string())
            .or_default()
            .push(child.to_string());
        child_ids.insert(child.to_string());
    }

    let mut roots = node_ids
        .difference(&child_ids)
        .cloned()
        .collect::<Vec<String>>();
    if roots.is_empty() {
        roots = node_ids.iter().cloned().collect();
    }
    roots.sort();

    let mut levels = BTreeMap::new();
    let mut queue = VecDeque::new();
    for root in roots {
        levels.insert(root.clone(), 0);
        queue.push_back(root);
    }

    while let Some(parent) = queue.pop_front() {
        let parent_level = *levels.get(&parent).unwrap_or(&0);
        for child in children_by_parent.get(&parent).into_iter().flatten() {
            let next_level = parent_level + 1;
            let should_update = levels
                .get(child)
                .is_none_or(|current| next_level > *current);
            if should_update {
                levels.insert(child.clone(), next_level);
                queue.push_back(child.clone());
            }
        }
    }

    for id in node_ids {
        levels.entry(id).or_insert(0);
    }
    levels
}

fn layout_parent_child<'a>(relation: &str, source: &'a str, target: &'a str) -> (&'a str, &'a str) {
    match relation {
        "part" => (source, target),
        _ => (target, source),
    }
}

fn rectangle_boundary_point(
    center: (usize, usize),
    toward: (usize, usize),
    half_width: usize,
    half_height: usize,
) -> (isize, isize) {
    let dx = toward.0 as f64 - center.0 as f64;
    let dy = toward.1 as f64 - center.1 as f64;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return (center.0 as isize, center.1 as isize);
    }

    let scale_x = if dx.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        half_width as f64 / dx.abs()
    };
    let scale_y = if dy.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        half_height as f64 / dy.abs()
    };
    let scale = scale_x.min(scale_y);

    (
        (center.0 as f64 + dx * scale).round() as isize,
        (center.1 as f64 + dy * scale).round() as isize,
    )
}

fn default_route(relation: &str) -> &'static str {
    match relation {
        "part" => "elbow",
        _ => "straight",
    }
}

fn default_source_decoration(relation: &str) -> &'static str {
    match relation {
        "part" => "filled_diamond",
        _ => "none",
    }
}

fn default_target_decoration(relation: &str) -> &'static str {
    match relation {
        "specializes" => "hollow_triangle",
        "part" | "source" | "target" | "transition" => "open_arrow",
        _ => "open_arrow",
    }
}

fn target_decoration_svg(decoration: &str, x1: isize, y1: isize, x2: isize, y2: isize) -> String {
    if decoration == "none" {
        return String::new();
    }

    let dx = x2 as f64 - x1 as f64;
    let dy = y2 as f64 - y1 as f64;
    let length = (dx * dx + dy * dy).sqrt();
    if length < f64::EPSILON {
        return String::new();
    }

    let ux = dx / length;
    let uy = dy / length;
    let px = -uy;
    let py = ux;
    let size = if decoration == "hollow_triangle" {
        20.0
    } else {
        14.0
    };
    let spread = if decoration == "hollow_triangle" {
        12.0
    } else {
        7.0
    };
    let tip_x = x2 as f64;
    let tip_y = y2 as f64;
    let left_x = tip_x - ux * size + px * spread;
    let left_y = tip_y - uy * size + py * spread;
    let right_x = tip_x - ux * size - px * spread;
    let right_y = tip_y - uy * size - py * spread;

    if decoration == "hollow_triangle" {
        return format!(
            r##"<path d="M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} Z" fill="#f8fafc" stroke="#0f172a" stroke-width="3" stroke-linejoin="miter"/>
"##,
            tip_x, tip_y, left_x, left_y, right_x, right_y
        );
    }

    if decoration == "open_arrow" {
        return format!(
            r##"<path d="M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1}" fill="none" stroke="#0f172a" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
"##,
            left_x, left_y, tip_x, tip_y, right_x, right_y
        );
    }

    format!(
        r##"<path d="M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} Z" fill="#dc2626" stroke="#dc2626" stroke-width="2"/>
"##,
        tip_x, tip_y, left_x, left_y, right_x, right_y
    )
}

fn source_decoration_svg(decoration: &str, x1: isize, y1: isize, x2: isize, y2: isize) -> String {
    if decoration != "filled_diamond" {
        return String::new();
    }

    let dx = x2 as f64 - x1 as f64;
    let dy = y2 as f64 - y1 as f64;
    let length = (dx * dx + dy * dy).sqrt();
    if length < f64::EPSILON {
        return String::new();
    }

    let ux = dx / length;
    let uy = dy / length;
    let px = -uy;
    let py = ux;
    let size = 10.0;
    let half_width = 5.5;
    let tip_x = x1 as f64;
    let tip_y = y1 as f64;
    let center_x = tip_x + ux * size;
    let center_y = tip_y + uy * size;
    let tail_x = tip_x + ux * size * 2.0;
    let tail_y = tip_y + uy * size * 2.0;
    let side_a_x = center_x + px * half_width;
    let side_a_y = center_y + py * half_width;
    let side_b_x = center_x - px * half_width;
    let side_b_y = center_y - py * half_width;

    format!(
        r##"<path d="M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1} Z" fill="#0f172a" stroke="#0f172a" stroke-width="1"/>
"##,
        tip_x, tip_y, side_a_x, side_a_y, tail_x, tail_y, side_b_x, side_b_y
    )
}

fn label_for_id(id: &str) -> String {
    id.rsplit("::")
        .next()
        .and_then(|segment| segment.rsplit('.').next())
        .filter(|segment| !segment.is_empty())
        .unwrap_or(id)
        .to_string()
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    escape_xml(&output)
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn write_activity_swimlane_demo(output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let stdlib = KirDocument::from_path(&default_stdlib_path())?;
    let kir = compile_sysml_text(
        ACTIVITY_SWIMLANE_SYSML,
        "activity-swimlane-demo.sysml",
        &stdlib,
    )?;
    fs::write(
        output_dir.join("activity-swimlane-demo.sysml"),
        ACTIVITY_SWIMLANE_SYSML,
    )?;
    fs::write(
        output_dir.join("activity-swimlane-demo.kir.json"),
        format!("{}\n", serde_json::to_string_pretty(&kir)?),
    )?;
    fs::write(
        output_dir.join("activity-swimlane-demo.svg"),
        activity_swimlane_svg(&kir),
    )?;
    Ok(())
}

const ACTIVITY_SWIMLANE_SYSML: &str = r#"package VehiclePowerExample {
  part def PowerControlUnit;
  part def InternalCombustionEngine;
  part def ElectricalPowerController;
  part def ElectricMotorGenerator;

  action def ProportionPower;
  action def ProvideGasPower;
  action def ControlElectricPower;
  action def ProvideElectricPower;

  action def ProvidePower {
    in vehCond;
    in accelPosition;
    out drivePower;

    action a1: ProportionPower;
    action a2: ProvideGasPower;
    action a3: ControlElectricPower;
    action a4: ProvideElectricPower;

    item battCond;
    item gThrottle;
    item eThrottle;
    item driveCurrent;
    item gasDrivePower;
    item elecDrivePower;
  }

  allocation a1_to_pcu allocate ProvidePower::a1 to PowerControlUnit;
  allocation a2_to_ice allocate ProvidePower::a2 to InternalCombustionEngine;
  allocation a3_to_epc allocate ProvidePower::a3 to ElectricalPowerController;
  allocation a4_to_emg allocate ProvidePower::a4 to ElectricMotorGenerator;
}
"#;

const REQUIREMENTS_METADATA_SYSML: &str = r#"package RequirementStatusExample {
  private import Metaobjects::SemanticMetadata;

  enum def RequirementStatusKind {
    enum draft;
    enum reviewed;
    enum approved;
    enum verified;
  }

  requirement requirements[*] nonunique;

  metadata def RequirementLifecycle :> SemanticMetadata {
    :>> baseType = requirements meta SysML::RequirementUsage;

    attribute status : RequirementStatusKind;
    attribute owner : String;
    attribute reviewDate : String;
  }
}

package VehicleRequirements {
  private import RequirementStatusExample::*;

  requirement <'REQ-001'> safeStart {
    doc /* Vehicle shall prevent unsafe starts. */

    @RequirementLifecycle {
      status = RequirementStatusKind::approved;
      owner = "Safety Team";
      reviewDate = "2026-05-27";
    }
  }
}
"#;

fn activity_swimlane_svg(kir: &KirDocument) -> String {
    let width = 1320;
    let height = 520;
    let margin = 28;
    let frame_x = 20;
    let frame_y = 14;
    let frame_w = 1280;
    let frame_h = 480;
    let lane_y = 86;
    let lane_h = 340;
    let lane_w = 250;
    let lane_gap = 0;
    let activity = swimlane_activity_from_kir(kir);
    let lane_x = activity
        .lanes
        .iter()
        .enumerate()
        .map(|(index, lane)| (lane.id.clone(), 110 + (lane_w + lane_gap) * index as i32))
        .collect::<BTreeMap<_, _>>();
    let action_offsets = BTreeMap::from([
        ("a1".to_string(), (35, 184)),
        ("a2".to_string(), (45, 79)),
        ("a3".to_string(), (40, 164)),
        ("a4".to_string(), (50, 164)),
    ]);
    let objects = [
        ("battCond", 64, 210),
        ("gThrottle", 382, 270),
        ("eThrottle", 615, 330),
        ("driveCurrent", 870, 330),
        ("gasDrivePower", 1160, 166),
        ("elecDrivePower", 1160, 330),
    ];
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-label="Provide Power activity swimlane demo">
<rect width="100%" height="100%" fill="#ffffff"/>
<rect x="{frame_x}" y="{frame_y}" width="{frame_w}" height="{frame_h}" fill="#ffffff" stroke="#0f172a" stroke-width="1.5"/>
<path d="M {frame_x} 14 L 380 14 L 360 36 L {frame_x} 36 Z" fill="#f8fafc" stroke="#0f172a" stroke-width="1"/>
<text x="32" y="30" font-family="Segoe UI, Arial, sans-serif" font-size="13" font-weight="700">act [activity] ProvidePower [KIR allocations]</text>
<rect x="90" y="64" width="930" height="385" rx="24" fill="none" stroke="#0f172a" stroke-width="1.2" stroke-dasharray="8 5"/>
"##
    );

    for lane in &activity.lanes {
        let Some(x) = lane_x.get(&lane.id).copied() else {
            continue;
        };
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{lane_y}" width="{lane_w}" height="{lane_h}" fill="#ffffff" stroke="#0f172a" stroke-width="1"/>
<rect x="{x}" y="{lane_y}" width="{lane_w}" height="44" fill="#f8fafc" stroke="#0f172a" stroke-width="1"/>
<text x="{}" y="104" font-family="Segoe UI, Arial, sans-serif" font-size="12" font-weight="700" text-anchor="middle">«performer» {}</text>
"##,
            x + lane_w / 2,
            escape_xml(&lane.label)
        ));
    }

    for (label, x, y) in objects {
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="128" height="46" rx="4" fill="#bae6fd" stroke="#0f172a" stroke-width="1.2"/>
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="12" font-weight="700" text-anchor="middle">«continuous»</text>
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="12" font-weight="700" text-anchor="middle">{}</text>
"##,
            x + 64,
            y + 18,
            x + 64,
            y + 34,
            escape_xml(label)
        ));
    }

    for action in &activity.actions {
        let Some(lane_x) = lane_x.get(&action.lane_id).copied() else {
            continue;
        };
        let short_name = action.short_name.clone();
        let (offset_x, offset_y) = action_offsets
            .get(&short_name)
            .copied()
            .unwrap_or((45, 164));
        let x = lane_x + offset_x;
        let y = lane_y + offset_y;
        svg.push_str(&format!(
            r##"<rect x="{x}" y="{y}" width="138" height="60" rx="10" fill="#fed7aa" stroke="#0f172a" stroke-width="1.2"/>
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="12" font-weight="700" text-anchor="middle">{}:</text>
<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="12" font-weight="700" text-anchor="middle">{}</text>
"##,
            x + 69,
            y + 24,
            escape_xml(&short_name),
            x + 69,
            y + 42,
            escape_xml(&action.label)
        ));
    }

    let flows = [
        (192, 233, 145, 300, "object"),
        (283, 300, 382, 293, "object"),
        (510, 293, 405, 225, "object"),
        (283, 300, 615, 353, "object"),
        (743, 353, 655, 310, "object"),
        (793, 310, 870, 353, "object"),
        (998, 353, 1160, 353, "object"),
        (543, 195, 1160, 189, "object"),
        (283, 300, 405, 195, "control"),
        (543, 195, 655, 280, "control"),
        (793, 280, 910, 280, "control"),
    ];
    for (x1, y1, x2, y2, kind) in flows {
        let label = if kind == "control" { "control" } else { "" };
        svg.push_str(&format!(
            r##"<path d="M {x1} {y1} L {} {y1} L {} {y2} L {x2} {y2}" fill="none" stroke="#0f172a" stroke-width="1.4"/>
{}{}
"##,
            (x1 + x2) / 2,
            (x1 + x2) / 2,
            open_arrow_path(x1, y1, x2, y2),
            if label.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<text x="{}" y="{}" font-family="Segoe UI, Arial, sans-serif" font-size="11">{}</text>"#,
                    (x1 + x2) / 2 + 8,
                    y2 - 8,
                    label
                )
            }
        ));
    }

    svg.push_str("</svg>\n");
    svg = svg.replace(
        "\u{00c2}\u{00ab}performer\u{00c2}\u{00bb}",
        "&lt;&lt;allocate target&gt;&gt;",
    );
    let _ = margin;
    svg
}

struct SwimlaneActivity {
    lanes: Vec<SwimlaneLane>,
    actions: Vec<SwimlaneAction>,
}

struct SwimlaneLane {
    id: String,
    label: String,
}

struct SwimlaneAction {
    short_name: String,
    label: String,
    lane_id: String,
}

fn swimlane_activity_from_kir(kir: &KirDocument) -> SwimlaneActivity {
    let elements = kir
        .elements
        .iter()
        .map(|element| (element.id.as_str(), element))
        .collect::<BTreeMap<_, _>>();
    let preferred_lane_order = [
        "PowerControlUnit",
        "InternalCombustionEngine",
        "ElectricalPowerController",
        "ElectricMotorGenerator",
    ];
    let mut lane_by_id = BTreeMap::<String, SwimlaneLane>::new();
    let mut actions = Vec::new();

    for allocation in &kir.elements {
        let Some(action_id) = json_str(&allocation.properties, "allocated") else {
            continue;
        };
        let Some(lane_id) = json_str(&allocation.properties, "allocated_to") else {
            continue;
        };
        let lane_label = elements
            .get(lane_id)
            .map(|element| display_name(element))
            .unwrap_or_else(|| label_for_id(lane_id));
        lane_by_id
            .entry(lane_id.to_string())
            .or_insert_with(|| SwimlaneLane {
                id: lane_id.to_string(),
                label: lane_label,
            });

        let action = elements.get(action_id).copied();
        actions.push(SwimlaneAction {
            short_name: action
                .map(display_name)
                .unwrap_or_else(|| label_for_id(action_id)),
            label: action
                .and_then(|element| json_str(&element.properties, "type"))
                .map(label_for_id)
                .unwrap_or_else(|| label_for_id(action_id)),
            lane_id: lane_id.to_string(),
        });
    }

    let mut lanes = lane_by_id.into_values().collect::<Vec<_>>();
    lanes.sort_by_key(|lane| {
        preferred_lane_order
            .iter()
            .position(|name| *name == lane.label)
            .unwrap_or(usize::MAX)
    });
    actions.sort_by_key(|action| action.short_name.clone());
    SwimlaneActivity { lanes, actions }
}

fn json_str<'a>(properties: &'a BTreeMap<String, serde_json::Value>, key: &str) -> Option<&'a str> {
    properties.get(key).and_then(|value| value.as_str())
}

fn display_name(element: &KirElement) -> String {
    json_str(&element.properties, "declared_name")
        .or_else(|| json_str(&element.properties, "name"))
        .map(str::to_string)
        .unwrap_or_else(|| label_for_id(&element.id))
}

fn open_arrow_path(x1: i32, y1: i32, x2: i32, y2: i32) -> String {
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    let length = (dx * dx + dy * dy).sqrt().max(1.0);
    let ux = dx / length;
    let uy = dy / length;
    let px = -uy;
    let py = ux;
    let size = 13.0;
    let spread = 6.0;
    let tip_x = x2 as f64;
    let tip_y = y2 as f64;
    let left_x = tip_x - ux * size + px * spread;
    let left_y = tip_y - uy * size + py * spread;
    let right_x = tip_x - ux * size - px * spread;
    let right_y = tip_y - uy * size - py * spread;
    format!(
        r##"<path d="M {:.1} {:.1} L {:.1} {:.1} L {:.1} {:.1}" fill="none" stroke="#0f172a" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"##,
        left_x, left_y, tip_x, tip_y, right_x, right_y
    )
}
