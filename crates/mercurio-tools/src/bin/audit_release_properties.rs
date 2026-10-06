//! Focused native assertions against immutable Pilot source/line/name/property observations.
//! Missing fields, ambiguous anchors and compilation errors never count as matches.
use mercurio_sysml::kerml::{
    compile_kerml_module_strict_with_context, load_kernel_baseline, parse_kerml,
};
use mercurio_sysml::{compile_sysml_module_with_context, load_sysml_baseline, parse_sysml};
use serde::Deserialize;
use mercurio_core::{KirDocument, KirElement};
use serde_json::{Value, json};
use std::{fs, io::Write};

#[derive(Deserialize)]
struct Case {
    relative_path: String,
    input_files: Vec<String>,
}
#[derive(Deserialize)]
struct Expectation {
    source: String,
    line: u64,
    name: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    property: String,
    pilot: Value,
}
#[derive(Deserialize)]
struct Spec {
    cases: Vec<Case>,
    expectations: Vec<Expectation>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: audit_release_properties <spec.json> <output.jsonl>".into());
    }
    let spec: Spec = serde_json::from_str(&fs::read_to_string(&args[0])?)?;
    if spec.cases.is_empty() || spec.expectations.is_empty() {
        return Err("empty property audit".into());
    }
    if spec.expectations.iter().any(|e| e.name.is_none() && e.kind.is_none()) {
        return Err("anonymous anchors require an explicit metaclass".into());
    }
    let paths = spec.cases.iter().map(|c| c.relative_path.as_str()).collect::<std::collections::BTreeSet<_>>();
    if paths.len() != spec.cases.len() || spec.expectations.iter().any(|e| !paths.contains(e.source.as_str())) {
        return Err("every expectation must have exactly one input case".into());
    }
    let sysml = load_sysml_baseline()?;
    let kernel = load_kernel_baseline()?;
    let mut output = fs::File::create(&args[1])?;
    let mut failed = false;
    let mut tested = 0;
    for case in &spec.cases {
        let inputs = case
            .input_files
            .iter()
            .map(|path| {
                let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
                (if path.ends_with(".kerml") {
                    parse_kerml(&text)
                } else {
                    parse_sysml(&text)
                })
                .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>();
        let document = inputs.and_then(|inputs| {
            let target = inputs.last().ok_or("empty input set")?;
            (if case.relative_path.ends_with(".kerml") {
                compile_kerml_module_strict_with_context(
                    target,
                    &case.relative_path,
                    &inputs,
                    &kernel,
                )
            } else {
                compile_sysml_module_with_context(target, &case.relative_path, &inputs, &sysml)
            })
            .map_err(|e| e.to_string())
        });
        for expected in spec
            .expectations
            .iter()
            .filter(|e| e.source == case.relative_path)
        {
            tested += 1;
            let mut row = json!({"source":expected.source,"line":expected.line,"name":expected.name,"property":expected.property,"expected":expected.pilot});
            match &document {
                Err(error) => {
                    row["status"] = json!("compile_error");
                    row["error"] = json!(error);
                }
                Ok(document) => {
                    let matches = document
                        .elements
                        .iter()
                        .filter(|e| {
                            let line = e
                                .properties
                                .get("metadata")
                                .and_then(|m| m.get("source_span"))
                                .or_else(|| e.properties.get("source_span"))
                                .and_then(|s| s.get("start_line"))
                                .and_then(Value::as_u64);
                            e.properties.get("declared_name").and_then(Value::as_str)
                                == expected.name.as_deref()
                                && expected.kind.as_ref().is_none_or(|kind| &e.kind == kind)
                                && line == Some(expected.line)
                        })
                        .collect::<Vec<_>>();
                    row["anchor_count"] = json!(matches.len());
                    row["status"] = json!("unpaired_or_ambiguous");
                    if let [element] = matches.as_slice() {
                        row["native_id"] = json!(element.id);
                        row["native_kind"] = json!(element.kind);
                        row["native_owner"] = element
                            .properties
                            .get("owner")
                            .cloned()
                            .unwrap_or(Value::Null);
                        let value = if expected.property == "@membership_graph" {
                            membership_graph(document, element)
                        } else if expected.property == "@owned_annotation_graph" {
                            owned_annotation_graph(document, element)
                        } else if expected.property == "@comment_annotation_graph" {
                            comment_annotation_graph(document, element)
                        } else { element.properties.get(&expected.property).cloned() };
                        if let Some(value) = value {
                            row["actual"] = value.clone();
                            row["status"] = json!(if value == expected.pilot {
                                "match"
                            } else {
                                "different"
                            });
                        } else {
                            row["status"] = json!("missing_property");
                        }
                    }
                }
            }
            failed |= row["status"] != "match";
            serde_json::to_writer(&mut output, &row)?;
            writeln!(output)?;
        }
        output.flush()?;
        eprintln!("{}: checked", case.relative_path);
    }
    if tested != spec.expectations.len() {
        return Err("not every expected row has exactly one input case".into());
    }
    if failed {
        std::process::exit(1);
    }
    Ok(())
}

// A bounded graph projection for the G05 comment controls. Resolve every edge
// to a source anchor; absent/ambiguous nodes never produce a passing projection.
fn comment_annotation_graph(document: &KirDocument, comment: &KirElement) -> Option<Value> {
    fn lookup<'a>(document: &'a KirDocument, id: &str) -> Option<&'a KirElement> {
        let mut nodes = document.elements.iter().filter(|e| e.id == id);
        let node = nodes.next()?;
        if nodes.next().is_some() { None } else { Some(node) }
    }
    fn anchor(node: &KirElement) -> Option<Value> {
        Some(json!({"kind":node.kind.rsplit("::").next()?, "name":node.properties.get("declared_name"), "line":node.properties.get("metadata")?.get("source_span")?.get("start_line")?.as_u64()?}))
    }
    fn refs(document: &KirDocument, value: &Value) -> Option<Vec<Value>> {
        let ids = if let Some(id) = value.as_str() { vec![id] } else { value.as_array()?.iter().map(Value::as_str).collect::<Option<Vec<_>>>()? };
        ids.into_iter().map(|id| anchor(lookup(document,id)?)).collect()
    }
    let owner = comment.properties.get("owner")?.as_str()?;
    let membership = lookup(document, comment.properties.get("owning_membership")?.as_str()?)?;
    let mut annotations = Vec::new();
    let ids = comment.properties.get("annotation")?.as_array()?;
    let owned = comment.properties.get("owned_relationship").and_then(Value::as_array);
    for id in ids {
        let node = lookup(document,id.as_str()?)?;
        let p = &node.properties;
        if !owned?.contains(id) || p.get("owning_annotating_element")?.as_str()? != comment.id || p.get("annotating_element")?.as_str()? != comment.id || p.contains_key("owner") || p.get("owning_related_element")?.as_str()? != comment.id { return None; }
        if p.get("owned_annotating_element").is_some_and(|v| !v.is_null()) || p.get("owning_annotated_element").is_some_and(|v| !v.is_null()) { return None; }
        annotations.push(json!({"kind":node.kind.rsplit("::").next()?, "line":p.get("metadata")?.get("source_span")?.get("start_line")?, "source":refs(document,p.get("source")?)?, "target":refs(document,p.get("target")?)?, "annotated_element":refs(document,p.get("annotated_element")?)?, "owned_related_element":refs(document,p.get("owned_related_element")?)?}));
    }
    let actual_count = document.elements.iter().filter(|e| e.kind.ends_with("::Annotation") && e.properties.get("owning_related_element").and_then(Value::as_str) == Some(comment.id.as_str())).count();
    if actual_count != ids.len() { return None; }
    Some(json!({"owner":anchor(lookup(document,owner)?)?, "body":comment.properties.get("body")?, "membership":{"kind":membership.kind.rsplit("::").next()?, "owner":refs(document,membership.properties.get("membership_owning_namespace")?)?, "member":refs(document,membership.properties.get("member_element")?)?}, "annotated_elements":refs(document,comment.properties.get("annotated_element")?)?, "annotations":annotations}))
}

// Exact bounded graph for source annotating elements and every attached Annotation.
fn owned_annotation_graph(document: &KirDocument, element: &KirElement) -> Option<Value> {
    fn lookup<'a>(document: &'a KirDocument, id: &str) -> Option<&'a KirElement> {
        let mut nodes = document.elements.iter().filter(|e| e.id == id);
        let node = nodes.next()?;
        if nodes.next().is_some() { None } else { Some(node) }
    }
    fn anchor(node: &KirElement) -> Option<Value> {
        Some(json!({"kind":node.kind.rsplit("::").next()?, "name":node.properties.get("declared_name"), "line":node.properties.get("metadata")?.get("source_span")?.get("start_line")?}))
    }
    fn node(document: &KirDocument, element: &KirElement) -> Option<Value> {
        let mut fields = serde_json::Map::new();
        for field in ["owner", "owning_relationship", "owning_namespace", "owning_membership", "annotation", "annotated_element", "annotating_element", "owning_annotating_relationship", "owned_annotating_relationship", "owned_annotating_element", "owning_annotating_element", "owning_annotated_element", "owning_related_element", "related_element", "source", "target", "owned_related_element", "owned_relationship"] {
            let values = match element.properties.get(field) {
                None | Some(Value::Null) => vec![],
                Some(Value::String(id)) => vec![id.as_str()],
                Some(Value::Array(ids)) => ids.iter().map(Value::as_str).collect::<Option<Vec<_>>>()?,
                _ => return None,
            };
            let targets = values.into_iter().map(|id| anchor(lookup(document, id)?)).collect::<Option<Vec<_>>>()?;
            fields.insert(field.into(), json!(targets));
        }
        Some(json!({"anchor":anchor(element)?, "body":element.properties.get("body"), "language":element.properties.get("language"), "refs":fields}))
    }
    let annotations = element.properties.get("annotation")?.as_array()?.iter()
        .map(|id| node(document, lookup(document, id.as_str()?)?)).collect::<Option<Vec<_>>>()?;
    Some(json!({"element":node(document, element)?, "annotations":annotations}))
}

// Selected concrete membership/import fields; every reference must resolve uniquely.
fn membership_graph(document: &KirDocument, element: &KirElement) -> Option<Value> {
    let mut refs = serde_json::Map::new();
    for field in ["source", "target", "owned_related_element", "owned_relationship", "member_element", "imported_membership", "imported_namespace"] {
        let ids = match element.properties.get(field) {
            None | Some(Value::Null) => vec![],
            Some(Value::String(id)) => vec![id.as_str()],
            Some(Value::Array(ids)) => ids.iter().map(Value::as_str).collect::<Option<Vec<_>>>()?,
            _ => return None,
        };
        let mut targets = Vec::new();
        for id in ids {
            let mut nodes = document.elements.iter().filter(|e| e.id == id);
            let node = nodes.next()?;
            if nodes.next().is_some() { return None; }
            targets.push(json!({"kind":node.kind.rsplit("::").next()?, "name":node.properties.get("declared_name"),
                "line":node.properties.get("metadata")?.get("source_span")?.get("start_line")?}));
        }
        refs.insert(field.into(), json!(targets));
    }
    let scalars = ["member_name", "member_short_name", "owned_member_name", "owned_member_short_name", "visibility", "is_implied", "is_import_all", "is_recursive"]
        .into_iter().map(|field| (field.to_string(), element.properties.get(field).cloned().unwrap_or(Value::Null))).collect::<serde_json::Map<_,_>>();
    Some(json!({"refs":refs,"scalars":scalars}))
}
