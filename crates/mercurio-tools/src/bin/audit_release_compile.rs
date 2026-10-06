//! Strict native release audit: partial KIR is not successful compilation.
use mercurio_core::{KirDocument, KirElement};
use mercurio_core::frontend::diagnostics::Diagnostic;
use mercurio_sysml::kerml::{
    compile_kerml_module_strict_with_context, load_kernel_baseline, parse_kerml,
};
use mercurio_sysml::{compile_sysml_module_with_context, load_sysml_baseline, parse_sysml};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[path = "audit_release_compile/profile.rs"]
mod definition_profile;

#[path = "audit_release_compile/core_preview.rs"]
mod core_preview;

#[derive(Deserialize)]
struct Spec {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    relative_path: String,
    input_files: Vec<String>,
    #[serde(default)]
    inspection_queries: Vec<StructureQuery>,
    #[serde(default)]
    literal_value_binding_sources: Vec<String>,
}
#[derive(Deserialize)]
struct StructureQuery {
    #[serde(default)]
    owner_path: Vec<String>,
    #[serde(default)]
    owner_id: Option<String>,
    field: String,
}

#[derive(Deserialize)]
struct DependencySpec { cases: Vec<DependencyCase> }
#[derive(Deserialize)]
struct DependencyCase {
    #[serde(flatten)]
    case: Case,
    dependency_roots: Vec<DependencyRead>,
}
#[derive(Deserialize)]
struct DependencyRead { owner_id: String, field: String }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "--definition-core-preview") {
        if args.len() != 3 { return Err("--definition-core-preview <spec.json> <models.jsonl>".into()); }
        return core_preview::export(&args[1], &args[2]);
    }
    if args.first().is_some_and(|arg| arg == "--definition-plan-records") {
        if args.len() != 4 { return Err("--definition-plan-records <inspection.jsonl> <plan.json> <output.jsonl>".into()); }
        return plan_definition_records(&args[1], &args[2], &args[3]);
    }
    if args.first().is_some_and(|arg| arg == "--definition-query-records") {
        if args.len() != 4 { return Err("--definition-query-records <inspection.jsonl> <queries.json> <output.json>".into()); }
        return query_definition_records(&args[1], &args[2], &args[3]);
    }
    if args.first().is_some_and(|arg| arg == "--definition-dependencies") {
        if args.len() != 3 { return Err("--definition-dependencies <spec.json> <output.jsonl>".into()); }
        return inspect_definition_dependencies(&args[1], &args[2]);
    }
    if args.first().is_some_and(|arg| arg == "--definition-structure") {
        if args.len() != 3 { return Err("--definition-structure <spec.json> <output.jsonl>".into()); }
        return inspect_definition_structure(&args[1], &args[2]);
    }
    if args.first().is_some_and(|arg|arg=="--definition-literal-value-bindings") {
        if args.len()!=3 {return Err("--definition-literal-value-bindings <spec.json> <output.jsonl>".into());}
        return assess_definition_candidate(&args[1],&args[2],false,false,true);
    }
    if args.first().is_some_and(|arg| arg == "--definition-candidate" || arg == "--definition-type-defaults" || arg == "--definition-defaults") {
        if args.len()!=3 {return Err("--definition-candidate <spec.json> <output.jsonl>".into());}
        return assess_definition_candidate(&args[1],&args[2],args[0] != "--definition-candidate",args[0] == "--definition-defaults",false);
    }
    if args.first().is_some_and(|arg| arg == "--assessment") {
        if args.len() != 3 {
            return Err("--assessment <spec.json> <output.json>".into());
        }
        return assess_group(&args[1], &args[2]);
    }
    if args.len() != 2 {
        return Err("usage: audit_release_compile <spec.json> <output.jsonl>".into());
    }
    let spec: Spec = serde_json::from_str(&std::fs::read_to_string(&args[0])?)?;
    if spec.cases.is_empty() {
        return Err("empty audit corpus".into());
    }
    let sysml = load_sysml_baseline()?;
    let kerml = load_kernel_baseline()?;
    let mut output = std::io::BufWriter::with_capacity(128 * 1024, std::fs::File::create(&args[1])?);
    let mut failed = false;
    for case in spec.cases {
        let start = Instant::now();
        let inputs = case
            .input_files
            .iter()
            .map(|path| Ok((path.clone(), std::fs::read_to_string(path)?)))
            .collect::<Result<Vec<_>, std::io::Error>>();
        let mut result = match inputs {
            Ok(inputs) => compile_case(&case.relative_path, &inputs, &sysml, &kerml),
            Err(error) => json!({"status":"infrastructure_error", "error":error.to_string()}),
        };
        result["relative_path"] = json!(case.relative_path);
        result["elapsed_ms"] = json!(start.elapsed().as_millis());
        failed |= result["status"] != "ok";
        serde_json::to_writer(&mut output, &result)?;
        writeln!(output)?;
        output.flush()?;
        eprintln!("{}: {}", case.relative_path, result["status"]);
    }
    if failed {
        std::process::exit(1);
    }
    Ok(())
}

/// Diagnostic-only construction view. Deliberately separate from candidate
/// assessment: an unlinked inspection never records successful compilation.
fn inspect_definition_structure(spec_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let spec: Spec = serde_json::from_str(&std::fs::read_to_string(spec_path)?)?;
    if spec.cases.is_empty() { return Err("empty inspection corpus".into()); }
    let mut output = std::io::BufWriter::with_capacity(128 * 1024, std::fs::File::create(output_path)?);
    let mut failed = false;
    for case in spec.cases {
        let start = Instant::now();
        let inputs = case.input_files.iter().map(|path| Ok((path.clone(), std::fs::read_to_string(path)?)))
            .collect::<Result<Vec<_>, std::io::Error>>();
        let mut row = match inputs {
            Ok(inputs) => definition_structure_case(&inputs, &case.inspection_queries),
            Err(e) => json!({"status":"infrastructure_error", "error":e.to_string()}),
        };
        failed |= row["status"] != "unlinked_inspection";
        row["relative_path"] = json!(case.relative_path);
        row["input_files"] = json!(case.input_files);
        row["elapsed_ms"] = json!(start.elapsed().as_millis());
        serde_json::to_writer(&mut output, &row)?; writeln!(output)?; output.flush()?;
        eprintln!("{}: {}", case.relative_path, row["status"]);
    }
    if failed { return Err("definition structure inspection failed; no candidate publication attempted".into()); }
    Ok(())
}


#[derive(Deserialize)]
struct ConstructedQuerySpec { inspection_queries: Vec<StructureQuery> }

#[derive(Deserialize)]
struct ConstructedDependencySpec {
    #[serde(default)]
    requirements: Vec<mercurio_sysml::definition_document::DefinitionQueryPrerequisite>,
    #[serde(default)]
    batches: Vec<ConstructedDependencyBatch>,
}
#[derive(Deserialize)]
struct ConstructedDependencyBatch {
    name: String,
    requirements: Vec<mercurio_sysml::definition_document::DefinitionQueryPrerequisite>,
    /// Exact original identities. Selection never rewrites a property or target.
    #[serde(default)]
    resource_uris: Option<Vec<String>>,
}

/// Build-time cache driver; native code supplies all targets and semantic edits.
/// Failed waves produce an error record without a partially updated model.
fn plan_definition_records(records_path: &str, plan_path: &str, output_path: &str)
    -> Result<(), Box<dyn std::error::Error>> {
    let input = std::fs::read_to_string(records_path)?;
    let rows = input.lines().filter(|line| !line.trim().is_empty()).collect::<Vec<_>>();
    if rows.len() != 1 { return Err("constructed plan cache requires exactly one native inspection row".into()); }
    let record: Value = serde_json::from_str(rows[0])?;
    let spec: ConstructedDependencySpec = serde_json::from_str(&std::fs::read_to_string(plan_path)?)?;
    if spec.requirements.is_empty() == spec.batches.is_empty() {
        return Err("cached plan requires one wave or named independent batches".into());
    }
    let batches = if spec.batches.is_empty() {vec![ConstructedDependencyBatch {name:"complete-wave".into(),requirements:spec.requirements,resource_uris:None}]}
        else {spec.batches};
    let mut names=std::collections::BTreeSet::new();
    if batches.iter().any(|b|b.name.trim().is_empty() || b.requirements.is_empty() || !names.insert(b.name.as_str())) {
        return Err("cached plan batches require distinct nonempty names and waves".into());
    }
    let mut output = std::io::BufWriter::with_capacity(128 * 1024, std::fs::File::create(output_path)?);
    let mut failed=false;
    for batch in batches {
        let mut result=plan_definition_record_batch(record.clone(), &batch);
        result["batch_name"]=json!(batch.name);
        failed |= result["status"] != "dependency_inspection";
        eprintln!("cached batch {}: {}",batch.name,result["status"]);
        serde_json::to_writer(&mut output, &result)?; writeln!(output)?; output.flush()?;
    }
    if failed { return Err("cached native dependency execution failed; no partial model returned".into()); }
    Ok(())
}

/// A diagnostic cache can contain independent positive and negative sources.
/// Project whole resources without rewriting identities, stored edges or ports.
/// The native consumer still enforces Closed structure and endpoint contracts.
fn project_definition_record(mut record: Value, selected: &[String]) -> Result<Value,String> {
    validate_definition_record_phase(&record)?;
    let paths: Vec<String> = serde_json::from_value(record["input_files"].clone()).map_err(|e|e.to_string())?;
    let original_paths = paths.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>();
    let selection = selected.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>();
    if paths.is_empty() || original_paths.len()!=paths.len()
        || paths.iter().any(|p|p.trim().is_empty() || p.contains('\0'))
        || selection.is_empty() || selection.len()!=selected.len()
        || !selection.is_subset(&original_paths) {
        return Err("context selection requires distinct exact original resource identities".into());
    }
    let root = |uri:&str| format!("definition.resource.{}",
        uri.bytes().map(|b|format!("{b:02x}")).collect::<String>());
    let original_roots = paths.iter().map(|p|root(p)).collect::<std::collections::BTreeSet<_>>();
    let selected_roots = selected.iter().map(|p|root(p)).collect::<std::collections::BTreeSet<_>>();
    let identity_root = |id:&str| id.strip_prefix("definition.resource.")
        .and_then(|tail|tail.split('.').next())
        .map(|encoded|format!("definition.resource.{encoded}"));
    let nodes = record["inspection"]["constructed_elements"].as_array()
        .ok_or("cached construction graph absent")?;
    let pending = record["inspection"]["pending_references"].as_array()
        .ok_or("cached pending registry absent")?;
    if record["element_count"].as_u64()!=Some(nodes.len() as u64)
        || record["pending_reference_count"].as_u64()!=Some(pending.len() as u64) {
        return Err("cached native construction counts differ".into());
    }
    let mut identities=std::collections::BTreeSet::new();
    for node in nodes {
        let id=node["id"].as_str().ok_or("cached node identity absent")?;
        if !identities.insert(id) || !identity_root(id).is_some_and(|r|original_roots.contains(&r)) {
            return Err("cached projection contains duplicate or foreign node identities".into());
        }
    }
    for resource in &original_roots {
        if !nodes.iter().any(|node|node["id"]==*resource && node["kind"]=="SysML::Namespace") {
            return Err("cached projection lacks an original canonical resource root".into());
        }
    }
    if pending.iter().any(|port|port["owner_id"].as_str().is_none_or(|id|!identities.contains(id))) {
        return Err("cached projection contains an absent pending owner".into());
    }
    let keep = |id:&str| identity_root(id).is_some_and(|r|selected_roots.contains(&r));
    // Preserve cache order even when selection order differs.
    let nodes = nodes.iter().filter(|node|keep(node["id"].as_str().unwrap())).cloned().collect::<Vec<_>>();
    let pending = pending.iter().filter(|port|keep(port["owner_id"].as_str().unwrap())).cloned().collect::<Vec<_>>();
    let paths = paths.iter().filter(|p|selection.contains(p.as_str())).cloned().collect::<Vec<_>>();
    let projection=json!({"status":"exact_resource_subset","original_resource_count":original_paths.len(),
        "selected_resource_count":paths.len(),"excluded_resource_count":original_paths.len()-paths.len(),
        "selected_input_files":paths,"properties_rewritten":false,"identities_rewritten":false});
    record["element_count"]=json!(nodes.len());
    record["pending_reference_count"]=json!(pending.len());
    record["inspection"]["constructed_elements"]=json!(nodes);
    record["inspection"]["pending_references"]=json!(pending);
    record["input_files"]=json!(paths);
    record["resource_projection"]=projection;
    Ok(record)
}

fn validate_definition_record_phase(record:&Value) -> Result<(),String> {
    if !matches!(record["status"].as_str(),Some("dependency_inspection" | "unlinked_inspection"))
        || record["publication"] != "not_attempted" || record["semantic_qualification"] != "not_assessed"
        || record["inspection"]["schema"] != "dev.mercurio.definition-structure-inspection.v1"
        || record["inspection"]["semantic_validation"] != "not_assessed"
        || record["inspection"]["transformation_completion"] != "not_assessed" {
        return Err("cached execution requires unqualified native construction records".into());
    }
    Ok(())
}

fn plan_definition_record_batch(record:Value,batch:&ConstructedDependencyBatch) -> Value {
    let record = match &batch.resource_uris {
        Some(selected) => match project_definition_record(record,selected) {
            Ok(projected)=>projected,
            Err(error)=>return json!({"status":"blocked","qualification_certificate":false,
                "publication":"not_attempted","semantic_qualification":"not_assessed","error":error,
                "failure_stage":"cache_projection","attempted_reference_commits_discarded":0,
                "requested_requirements":batch.requirements}),
        },
        None=>record,
    };
    let projection=record.get("resource_projection").cloned();
    let mut outcome=plan_definition_record_row(record,&batch.requirements);
    if let Some(projection)=projection {outcome["resource_projection"]=projection;}
    outcome
}

fn plan_definition_record_row(mut record: Value,
    requirements: &[mercurio_sysml::definition_document::DefinitionQueryPrerequisite]) -> Value {
    use mercurio_sysml::{SourceLanguage, definition_document::{DefinitionConstructedResource,
        DefinitionPendingReference, DefinitionConstructionPhase, inspect_constructed_dependencies_traced}};
    let start = Instant::now();
    let committed = std::cell::RefCell::new(Vec::new());
    let failed_reference_reads = std::cell::RefCell::new(Vec::new());
    let progress = std::env::var_os("MERCURIO_DEFINITION_PROGRESS").is_some();
    let result = (|| -> Result<Value, String> {
        validate_definition_record_phase(&record)?;
        let graph: Vec<KirElement> = serde_json::from_value(record["inspection"]["constructed_elements"].take()).map_err(|e|e.to_string())?;
        let pending: Vec<DefinitionPendingReference> = serde_json::from_value(record["inspection"]["pending_references"].take()).map_err(|e|e.to_string())?;
        if record["element_count"].as_u64()!=Some(graph.len() as u64)
            || record["pending_reference_count"].as_u64()!=Some(pending.len() as u64) {
            return Err("cached native construction counts differ".into());
        }
        let paths: Vec<String> = serde_json::from_value(record["input_files"].clone()).map_err(|e|e.to_string())?;
        let resources = paths.iter().map(|path| {
            let language=match Path::new(path).extension().and_then(|s|s.to_str()) {
                Some("kerml")=>SourceLanguage::Kerml,Some("sysml")=>SourceLanguage::Sysml,
                _=>return Err("cached source requires its original language suffix".to_owned()),
            };
            Ok(DefinitionConstructedResource {uri:path,language})
        }).collect::<Result<Vec<_>,String>>()?;
        let inspection = inspect_constructed_dependencies_traced(&resources,&graph,&pending,requirements,|event| {
            if event.phase==DefinitionConstructionPhase::LinkFieldCommitted {
                committed.borrow_mut().push(json!({"owner_id":event.owner_id,"field":event.field}));
            }
            if event.phase==DefinitionConstructionPhase::ReferenceFinished && event.succeeded==Some(false) {
                failed_reference_reads.borrow_mut().push(json!({"owner_id":event.owner_id,"field":event.field,
                    "pending_descriptors":pending.iter().filter(|p|p.owner_id==event.owner_id
                        && Some(p.field.as_str())==event.field).collect::<Vec<_>>()}));
            }
            if progress { eprintln!("cached dependency: elapsed_ms={} phase={:?} elements={} succeeded={:?}",
                start.elapsed().as_millis(),event.phase,event.element_count,event.succeeded); }
        }).map_err(|e|e.to_string())?;
        Ok(json!({"status":"dependency_inspection","qualification_certificate":false,
            "semantic_qualification":"not_assessed","publication":"not_attempted",
            "element_count":inspection.elements().len(),"pending_reference_count":inspection.pending_references().len(),
            "input_files":paths,"inspection":inspection,"committed_reference_fields":committed.borrow().clone(),
            "boundary":"Retained native construction and current native dependency execution have separate provenance. This wave does not qualify a context or family."}))
    })();
    let mut row=result.unwrap_or_else(|error|json!({"status":"blocked","qualification_certificate":false,
        "publication":"not_attempted","semantic_qualification":"not_assessed","error":error,
        "attempted_reference_commits_discarded":committed.borrow().len(),
        "failed_native_reference_reads":failed_reference_reads.borrow().clone()}));
    row["requested_requirements"]=json!(requirements);
    row["elapsed_ms"]=json!(start.elapsed().as_millis());
    row
}

/// Read-only native getters on retained diagnostic records. File access and
/// provenance belong to maintainer tooling; no partial KirDocument is created.
fn query_definition_records(records_path: &str, queries_path: &str, output_path: &str)
    -> Result<(), Box<dyn std::error::Error>> {
    let input = std::fs::read_to_string(records_path)?;
    let rows = input.lines().filter(|line| !line.trim().is_empty()).collect::<Vec<_>>();
    if rows.len() != 1 { return Err("constructed query cache requires exactly one complete native inspection row".into()); }
    let record: Value = serde_json::from_str(rows[0])?;
    let spec: ConstructedQuerySpec = serde_json::from_str(&std::fs::read_to_string(queries_path)?)?;
    let result = query_definition_record_rows(record, &spec.inspection_queries)?;
    let mut output = std::io::BufWriter::with_capacity(128 * 1024, std::fs::File::create(output_path)?);
    serde_json::to_writer(&mut output, &result)?; writeln!(output)?; output.flush()?;
    Ok(())
}

fn query_definition_record_rows(mut record: Value, queries: &[StructureQuery]) -> Result<Value, String> {
    let valid = matches!(record["status"].as_str(), Some("dependency_inspection" | "unlinked_inspection"))
        && record["publication"] == "not_attempted"
        && record["semantic_qualification"] == "not_assessed"
        && record["inspection"]["schema"] == "dev.mercurio.definition-structure-inspection.v1";
    if !valid { return Err("cached getters require unqualified native construction-inspection records".into()); }
    let count = record["element_count"].as_u64().ok_or("cached element count absent")?;
    let pending = record["inspection"]["pending_references"].as_array().ok_or("cached pending registry absent")?.len();
    if record["pending_reference_count"].as_u64() != Some(pending as u64) {
        return Err("cached pending registry count differs".into());
    }
    let graph: Vec<KirElement> = serde_json::from_value(record["inspection"]["constructed_elements"].take())
        .map_err(|e| e.to_string())?;
    if graph.len() as u64 != count { return Err("cached graph count differs".into()); }
    if queries.is_empty() || queries.iter().any(|q| q.owner_id.as_ref().is_none_or(|id| id.is_empty())) {
        return Err("cached query requests require explicit nonempty native owner identities".into());
    }
    let requests = queries.iter().filter_map(|q| q.owner_id.as_deref().map(|id| (id, q.field.as_str()))).collect::<Vec<_>>();
    let results = mercurio_sysml::definition_document::query_constructed_references(&graph, &requests)
        .map_err(|e| e.to_string())?;
    let index = graph.iter().map(|e| (e.id.as_str(), e)).collect::<std::collections::BTreeMap<_, _>>();
    let rows = queries.iter().zip(results).map(|(request, result)| {
        let owner = request.owner_id.as_deref().and_then(|id| index.get(id).copied());
        if !request.owner_path.is_empty() && owner.is_some_and(|owner|
            structure_declared_path(&index, &owner.id).ok().as_ref() != Some(&request.owner_path)) {
            return json!({"owner_id":request.owner_id,"field":request.field,"status":"unavailable",
                "error":"cached query owner path differs from stored ownership"});
        }
        match result {
            Ok(targets) => json!({"owner_id":request.owner_id,"field":request.field,"status":"query_evaluated",
                "targets":targets.iter().map(|target| json!({"id":target.id,"kind":target.kind,
                    "declared_path":structure_declared_path(&index,&target.id).ok()})).collect::<Vec<_>>()}),
            Err(e @ mercurio_sysml::definition_document::DefinitionReferenceQueryError::Required {..}) =>
                json!({"owner_id":request.owner_id,"field":request.field,"status":"dependency_required","dependency":e}),
            Err(e) => json!({"owner_id":request.owner_id,"field":request.field,"status":"unavailable","error":e.to_string()}),
        }
    }).collect::<Vec<_>>();
    Ok(json!({"schema":"dev.mercurio.constructed-native-query-batch.v1","qualification_certificate":false,
        "status":"read_only_diagnostic","publication":"not_attempted","semantic_qualification":"not_assessed",
        "constructed_elements_read":graph.len(),"unfinished_reference_descriptors_retained":pending,
        "input_files":record["input_files"],"queries":rows,
        "boundary":"Cached native construction and current native semantic queries have separate provenance; this never qualifies a complete context or family."}))
}

/// Build-time driver for explicit native dependency roots. Targets are resolved
/// by the native language consumers; the spec never supplies endpoint identities.
fn inspect_definition_dependencies(spec_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let spec: DependencySpec = serde_json::from_str(&std::fs::read_to_string(spec_path)?)?;
    if spec.cases.is_empty() { return Err("empty dependency corpus".into()); }
    let mut output = std::io::BufWriter::with_capacity(128 * 1024, std::fs::File::create(output_path)?);
    let mut failed = false;
    for plan in spec.cases {
        let case = plan.case;
        let start = Instant::now();
        let requirements = plan.dependency_roots.iter().map(|port|
            mercurio_sysml::definition_document::DefinitionQueryPrerequisite::ReadField {
                owner_id: port.owner_id.clone(), field: port.field.clone() }).collect::<Vec<_>>();
        let inputs = case.input_files.iter().map(|path| Ok((path.clone(), std::fs::read_to_string(path)?)))
            .collect::<Result<Vec<_>, std::io::Error>>();
        let mut row = match inputs {
            Ok(inputs) => definition_structure_case_planned(&inputs, &case.inspection_queries, Some(&requirements)),
            Err(e) => json!({"status":"infrastructure_error", "error":e.to_string()}),
        };
        failed |= row["status"] != "dependency_inspection";
        row["relative_path"] = json!(case.relative_path);
        row["input_files"] = json!(case.input_files);
        row["requested_requirements"] = json!(requirements);
        row["elapsed_ms"] = json!(start.elapsed().as_millis());
        serde_json::to_writer(&mut output, &row)?; writeln!(output)?; output.flush()?;
        eprintln!("{}: {}", case.relative_path, row["status"]);
    }
    if failed { return Err("native dependency execution failed; no candidate publication attempted".into()); }
    Ok(())
}

/// Select observation receivers by stored declaration ancestry only. This is
/// tooling identity selection, not a substitute for language naming or scoping.
fn structure_declared_path(index: &std::collections::BTreeMap<&str, &KirElement>, id: &str) -> Result<Vec<String>, String> {
    let mut cursor = Some(id); let mut seen = std::collections::BTreeSet::new(); let mut path = Vec::new();
    while let Some(id) = cursor {
        if !seen.insert(id) { return Err("inspection ownership cycle".into()); }
        let element = index.get(id).ok_or("inspection owner absent")?;
        if let Some(name) = element.properties.get("declared_name").and_then(Value::as_str) { path.push(name.to_owned()); }
        cursor = if let Some(relation) = element.properties.get("owning_relationship").and_then(Value::as_str) {
            index.get(relation).ok_or("inspection ownership relation absent")?.properties.get("owning_related_element").and_then(Value::as_str)
        } else { element.properties.get("owning_related_element").and_then(Value::as_str) };
    }
    path.reverse(); Ok(path)
}

fn definition_structure_case(inputs: &[(String, String)], queries: &[StructureQuery]) -> Value {
    definition_structure_case_planned(inputs, queries, None)
}

fn definition_structure_case_planned(inputs: &[(String, String)], queries: &[StructureQuery],
    requirements: Option<&[mercurio_sysml::definition_document::DefinitionQueryPrerequisite]>) -> Value {
    use mercurio_sysml::{SourceLanguage, definition_document::{DefinitionSource, inspect_source_structure_traced, inspect_source_dependencies_traced}};
    if inputs.is_empty() || inputs.iter().any(|(_, text)| text.trim().is_empty()) {
        return json!({"status":"infrastructure_error", "error":"inspection inputs must be present and nonempty"});
    }
    let mut sources = Vec::new();
    for (path, text) in inputs {
        let language = match Path::new(path).extension().and_then(|s| s.to_str()) {
            Some("kerml") => SourceLanguage::Kerml, Some("sysml") => SourceLanguage::Sysml,
            _ => return json!({"status":"infrastructure_error", "error":"inspection source requires a language suffix"}),
        };
        sources.push(DefinitionSource { uri:path, text, language });
    }
    let progress = std::env::var_os("MERCURIO_DEFINITION_PROGRESS").is_some();
    let clock = Instant::now();
    let committed = std::cell::RefCell::new(Vec::new());
    let source_progress = |uri: Option<&str>, phase| {
        if progress { eprintln!("inspection source: elapsed_ms={} phase={phase:?} {}", clock.elapsed().as_millis(), uri.unwrap_or("<source set>")); }
    };
    let construction_progress = |event: mercurio_sysml::definition_document::DefinitionConstructionEvent<'_>| {
        if event.phase == mercurio_sysml::definition_document::DefinitionConstructionPhase::LinkFieldCommitted {
            committed.borrow_mut().push(json!({"owner_id":event.owner_id,"field":event.field}));
        }
        if progress { eprintln!("inspection construction: elapsed_ms={} phase={:?} elements={} succeeded={:?}", clock.elapsed().as_millis(), event.phase, event.element_count, event.succeeded); }
    };
    let outcome = match requirements {
        Some(requirements) => inspect_source_dependencies_traced(&sources, requirements, source_progress, construction_progress),
        None => inspect_source_structure_traced(&sources, source_progress, construction_progress),
    };
    match outcome {
        Ok(inspection) => {
            let index = inspection.elements().iter().map(|e| (e.id.as_str(), e)).collect::<std::collections::BTreeMap<_, _>>();
            let mut declarations = std::collections::BTreeMap::<Vec<String>, Vec<&KirElement>>::new();
            if !queries.is_empty() {
                for element in inspection.elements().iter().filter(|e| e.properties.get("declared_name").and_then(Value::as_str).is_some()) {
                    if let Ok(path) = structure_declared_path(&index, &element.id) { declarations.entry(path).or_default().push(element); }
                }
            }
            let query_rows = queries.iter().map(|query| {
                let candidates = if let Some(id) = &query.owner_id {
                    index.get(id.as_str()).copied().filter(|owner| query.owner_path.is_empty()
                        || structure_declared_path(&index, &owner.id).ok().as_ref() == Some(&query.owner_path)).into_iter().collect::<Vec<_>>()
                } else { declarations.get(&query.owner_path).cloned().unwrap_or_default() };
                if candidates.len() != 1 { return json!({"owner_path":query.owner_path,"field":query.field,"status":"unavailable","error":"inspection selector requires exactly one declared owner","matches":candidates.len()}); }
                let owner = candidates[0];
                match inspection.reference_targets_with_dependencies(&owner.id, &query.field) {
                    Ok(targets) => {
                        let records = targets.iter().map(|target| {
                            let relation = target.properties.get("owning_relationship").and_then(Value::as_str)
                                .and_then(|id| inspection.elements().iter().find(|e| e.id == id));
                            json!({"id":target.id,"kind":target.kind,"declared_path":structure_declared_path(&index, &target.id).ok(),
                                "owning_relationship_kind":relation.map(|e| &e.kind),"stored_direction":target.properties.get("direction"),"stored_properties":target.properties})
                        }).collect::<Vec<_>>();
                        json!({"owner_path":query.owner_path,"owner_id":owner.id,"field":query.field,"status":"query_evaluated","targets":records,"graph_qualification":"not_assessed"})
                    },
                    Err(e @ mercurio_sysml::definition_document::DefinitionReferenceQueryError::Required { .. }) =>
                        json!({"owner_path":query.owner_path,"owner_id":owner.id,"field":query.field,"status":"dependency_required","dependency":e,"graph_qualification":"not_assessed"}),
                    Err(e) => json!({"owner_path":query.owner_path,"owner_id":owner.id,"field":query.field,"status":"unavailable","error":e.to_string(),"graph_qualification":"not_assessed"}),
                }
            }).collect::<Vec<_>>();
            json!({"status":if requirements.is_some() {"dependency_inspection"} else {"unlinked_inspection"}, "semantic_qualification":"not_assessed",
                "publication":"not_attempted", "element_count":inspection.elements().len(),
                "pending_reference_count":inspection.pending_references().len(), "queries":query_rows,"inspection":inspection,
                "committed_reference_fields":committed.into_inner()})
        },
        Err(e) => json!({"status":"blocked", "error":e.to_string(), "publication":"not_attempted"}),
    }
}

/// This audits the opt-in definition pipeline, without using the legacy baseline
/// or claiming that construction implies complete semantic qualification.
fn assess_definition_candidate(spec_path: &str, output_path: &str, type_defaults: bool, feature_defaults: bool, literal_bindings: bool) -> Result<(), Box<dyn std::error::Error>> {
    let spec:Spec=serde_json::from_str(&std::fs::read_to_string(spec_path)?)?;
    if spec.cases.is_empty() {return Err("empty audit corpus".into());}
    let mut output=std::io::BufWriter::with_capacity(128 * 1024, std::fs::File::create(output_path)?);
    let mut failed=false;
    // Assessment depends on resource identities, grammars and content, not the
    // reporting target. Reuse exact source-set outcomes within this process;
    // changed content is a new key. No persistent cache or semantic claim.
    let mut cache = std::collections::BTreeMap::<(Vec<(String,String)>,Vec<String>),Value>::new();
    for case in spec.cases {
        let inputs=case.input_files.iter().map(|path|Ok((path.clone(),std::fs::read_to_string(path)?)))
            .collect::<Result<Vec<_>,std::io::Error>>();
        let (mut row, cache_hit)=match inputs {
            Ok(inputs) => {
                let scope=if literal_bindings {case.literal_value_binding_sources.as_slice()}else{&[]};
                if literal_bindings && scope.is_empty() {
                    (json!({"status":"infrastructure_error","error":"literal producer audit requires an explicit nonempty source URI selection"}),false)
                } else {
                    definition_candidate_cached(&case.relative_path,inputs,type_defaults,feature_defaults,scope,&mut cache)
                }
            },
            Err(error)=>(json!({"status":"infrastructure_error","error":error.to_string()}),false),
        };
        row["source_set_cache_hit"]=json!(cache_hit);
        row["relative_path"]=json!(case.relative_path);
        row["input_files"]=json!(case.input_files);
        row["semantic_qualification"]=json!("not_assessed");
        failed|=row["status"]!="constructed_unqualified";
        serde_json::to_writer(&mut output,&row)?;writeln!(output)?;output.flush()?;
        eprintln!("{}: {}",case.relative_path,row["status"]);
    }
    if failed {return Err("definition candidate has unresolved qualification dependencies".into());}
    Ok(())
}

// A changed producer scope is a distinct source-set computation, even when all
// resource bytes are identical. Cache entries remain process-local and unqualified.
fn definition_candidate_cached(name:&str,inputs:Vec<(String,String)>,type_defaults:bool,feature_defaults:bool,
    scope:&[String],cache:&mut std::collections::BTreeMap<(Vec<(String,String)>,Vec<String>),Value>) -> (Value,bool) {
    let key=(inputs,scope.to_vec());
    if let Some(row)=cache.get(&key) {return (row.clone(),true);}
    let row=definition_candidate_case_with_stages(name,&key.0,type_defaults,feature_defaults,scope);
    cache.insert(key,row.clone());(row,false)
}

#[cfg(test)]
fn definition_candidate_case(name: &str, inputs: &[(String,String)]) -> Value {
    definition_candidate_case_with_type_defaults(name, inputs, false)
}

#[cfg(test)]
fn definition_candidate_case_with_type_defaults(name: &str, inputs: &[(String,String)], type_defaults: bool) -> Value {
    definition_candidate_case_with_defaults(name,inputs,type_defaults,false)
}

#[cfg(test)]
fn definition_candidate_case_with_defaults(name: &str, inputs: &[(String,String)], type_defaults: bool, feature_defaults: bool) -> Value {
    definition_candidate_case_with_stages(name,inputs,type_defaults,feature_defaults,&[])
}

fn definition_candidate_case_with_stages(name: &str, inputs: &[(String,String)], type_defaults: bool, feature_defaults: bool, literal_binding_sources:&[String]) -> Value {
    use mercurio_sysml::{SourceLanguage,definition_document::{parse_and_link_sources_traced,parse_and_link_sources_with_literal_value_bindings_traced,DefinitionSource,DefinitionDocumentError as E}};
    if inputs.is_empty() || inputs.iter().any(|(_,text)|text.trim().is_empty()) {
        return json!({"status":"infrastructure_error","error":"candidate inputs must be present and nonempty"});
    }
    let source_bytes = inputs.iter().map(|(_,text)|text.len()).sum::<usize>();
    let mut sources = Vec::new();
    for (path,text) in inputs {
        let language = match Path::new(path).extension().and_then(|s|s.to_str()) {
            Some("kerml") => SourceLanguage::Kerml,
            Some("sysml") => SourceLanguage::Sysml,
            _ => return json!({"status":"infrastructure_error","error":"candidate resource requires a KerML or SysML suffix"}),
        };
        sources.push(DefinitionSource {uri:path,text,language});
    }
    let _target_name = name; // Target identity belongs to the audit row, not grammar selection.
    let progress = std::env::var_os("MERCURIO_DEFINITION_PROGRESS").is_some();
    let query_progress = std::env::var_os("MERCURIO_DEFINITION_QUERY_PROGRESS").is_some();
    // Tool-owned bounded observation: avoids emitting every large URI/query.
    let query_summary = std::env::var_os("MERCURIO_DEFINITION_QUERY_SUMMARY").is_some();
    let phase_counts = std::cell::RefCell::new(std::collections::BTreeMap::<String, usize>::new());
    let clock = std::time::Instant::now();
    let profile_enabled = std::env::var_os("MERCURIO_DEFINITION_QUERY_PROFILE").is_some();
    let profile = std::cell::RefCell::new(definition_profile::QueryProfile::default());
    let next_profile_ns = std::cell::Cell::new(10_000_000_000u64);
    let source_observer=|uri:Option<&str>, phase| {
        if progress { eprintln!("definition progress: {phase:?} {}", uri.unwrap_or("<source set>")); }
    };
    let construction_observer=|event:mercurio_sysml::definition_document::DefinitionConstructionEvent<'_>| {
        if query_progress { eprintln!("definition query: elapsed_ms={} phase={:?} owner={} kind={} field={} elements={} succeeded={:?}",clock.elapsed().as_millis(),event.phase,event.owner_id,event.owner_kind.unwrap_or(""),event.field.unwrap_or(""),event.element_count,event.succeeded); }
        if query_summary && matches!(event.phase,
            mercurio_sysml::definition_document::DefinitionConstructionPhase::LiteralProducerStarted |
            mercurio_sysml::definition_document::DefinitionConstructionPhase::LiteralProducerFinished |
            mercurio_sysml::definition_document::DefinitionConstructionPhase::LiteralStageStarted |
            mercurio_sysml::definition_document::DefinitionConstructionPhase::LiteralStageFinished |
            mercurio_sysml::definition_document::DefinitionConstructionPhase::LiteralPrerequisite) {
            eprintln!("definition literal stage: {}",json!({"elapsed_ms":clock.elapsed().as_millis(),
                "phase":format!("{:?}",event.phase),"owner_id":event.owner_id,"kind":event.owner_kind,
                "stage_or_field":event.field,"elements":event.element_count,"succeeded":event.succeeded}));
        }
        if profile_enabled {
            let now = clock.elapsed().as_nanos().min(u64::MAX as u128) as u64;
            profile.borrow_mut().observe(event,now);
            if now >= next_profile_ns.get() {
                eprintln!("definition query profile: {}",profile.borrow().snapshot(now));
                next_profile_ns.set(now.saturating_add(10_000_000_000));
            }
        }
        if query_summary && !profile_enabled {
            let phase = format!("{:?}", event.phase);
            let mut counts = phase_counts.borrow_mut();
            let count = counts.entry(phase).or_default();
            *count += 1;
            // Keep the first dependency chain visible: three reference events
            // hid the next unresolved attempt in the large library workload.
            let initial = if matches!(event.phase, mercurio_sysml::definition_document::DefinitionConstructionPhase::ReferenceStarted | mercurio_sysml::definition_document::DefinitionConstructionPhase::ReferenceFinished) { 8 } else { 3 };
            // Preserve the complete identity for a failed reference: a suffix
            // can match unrelated objects in different library resources.
            if event.succeeded == Some(false) && matches!(event.phase, mercurio_sysml::definition_document::DefinitionConstructionPhase::ReferenceFinished) {
                eprintln!("definition failed reference: {}", json!({"owner_id":event.owner_id,"kind":event.owner_kind,"field":event.field}));
            }
            // Failed reads carry the concrete owner/context even between samples.
            if event.succeeded == Some(false) || *count <= initial || *count % 500 == 0 {
                let owner = event.owner_id.chars().rev().take(160).collect::<String>().chars().rev().collect::<String>();
                eprintln!("definition query summary: elapsed_ms={} phase={:?} count={} owner_suffix={} kind={} field={} elements={} succeeded={:?}",clock.elapsed().as_millis(),event.phase,count,owner,event.owner_kind.unwrap_or(""),event.field.unwrap_or(""),event.element_count,event.succeeded);
            }
        }
    };
    let parsed=if literal_binding_sources.is_empty() {
        parse_and_link_sources_traced(&sources,source_observer,construction_observer)
    } else {
        let selection=literal_binding_sources.iter().map(String::as_str).collect::<Vec<_>>();
        parse_and_link_sources_with_literal_value_bindings_traced(&sources,&selection,source_observer,construction_observer)
    };
    let profile_row = if profile_enabled {
        let row=profile.borrow().snapshot(clock.elapsed().as_nanos().min(u64::MAX as u128) as u64);
        eprintln!("definition query profile: {row}");
        Some(row)
    } else {None};
    let mut outcome = match parsed {
        Ok(mut document)=> {
            let mut row = json!({"status":"constructed_unqualified","element_count":document.elements.len(),"source_bytes":source_bytes,"resource_mode":"separate"});
            if !literal_binding_sources.is_empty() {
                row["literal_value_bindings"]=json!({"status":"selected_receivers_completed_unqualified","sources":literal_binding_sources,
                    "native_stage":document.metadata.get("literal_value_binding_stage"),"full_model_transformation":"not_assessed"});
                row["semantic_qualification"]=json!("not_assessed");
                row["transformation_completion"]=json!("not_assessed");
            }
            if feature_defaults {
                match mercurio_sysml::definition_document::materialize_owned_expression_results(&mut document,"definition.contribution.expression-result") {
                    Ok(inserted) => {
                        row["owned_expression_results"] = json!({"status":"contributed_unqualified","inserted":inserted,"scope":"seven imported owned-result providers; inherited and other providers not assessed"});
                        row["element_count"] = json!(document.elements.len());
                    },
                    Err(error) => {
                        row["status"] = json!("blocked"); row["stage"] = json!("owned_expression_results");
                        row["error"] = json!(error.to_string());
                    },
                }
                row["semantic_qualification"] = json!("not_assessed");
                row["transformation_completion"] = json!("not_assessed");
            }
            if type_defaults && row["status"] == "constructed_unqualified" {
                use mercurio_sysml::definition_document::{type_default_contribution_owners,materialize_type_defaults};
                let owners = type_default_contribution_owners(&document);
                let prefixes: Vec<_> = (0..owners.len()).map(|i| format!("definition.contribution.type-default.{i}")).collect();
                let batch: Vec<_> = owners.iter().zip(&prefixes).map(|(id,prefix)| (id.as_str(),prefix.as_str())).collect();
                row["type_default_owner_count"] = json!(owners.len());
                match materialize_type_defaults(&mut document, &batch) {
                    Ok(inserted) => {
                        row["type_defaults"] = json!({"status":"contributed_unqualified","inserted":inserted});
                        row["element_count"] = json!(document.elements.len());
                    },
                    Err(error) => {
                        row["status"] = json!("blocked"); row["stage"] = json!("type_default_contributions");
                        row["error"] = json!(format!("{error:?}"));
                    },
                }
                // Type defaults are one producer. Neither transformation nor
                // semantic qualification follows from these contributions.
                row["transformation_completion"] = json!("not_assessed");
                row["semantic_qualification"] = json!("not_assessed");
            }
            if feature_defaults && row["status"] == "constructed_unqualified" {
                use mercurio_sysml::definition_document::{feature_default_contribution_owners,materialize_feature_defaults};
                let owners = feature_default_contribution_owners(&document);
                let prefixes: Vec<_> = (0..owners.len()).map(|i|format!("definition.contribution.feature-default.{i}")).collect();
                let batch: Vec<_> = owners.iter().zip(&prefixes).map(|(id,prefix)|(id.as_str(),prefix.as_str())).collect();
                row["feature_default_owner_count"] = json!(owners.len());
                match materialize_feature_defaults(&mut document,&batch) {
                    Ok(inserted) => {
                        row["feature_defaults"] = json!({"status":"contributed_unqualified","inserted":inserted});
                        row["element_count"] = json!(document.elements.len());
                    },
                    Err(error) => {
                        row["status"] = json!("blocked"); row["stage"] = json!("feature_default_contributions");
                        row["error"] = json!(format!("{error:?}"));
                    },
                }
                row["transformation_completion"] = json!("not_assessed");
                row["semantic_qualification"] = json!("not_assessed");
            }
            if feature_defaults && row["status"] == "constructed_unqualified" {
                match verify_default_replay(&document,feature_defaults) {
                    Ok(()) => {
                        row["default_replay"] = json!("idempotent_after_kir_persistence");
                        match mercurio_sysml::definition_document::assess_required_features(&document) {
                            Ok(issues) => {
                                let unverified = issues.iter().filter(|issue| issue.unverified).count();
                                row["required_values"] = json!({
                                    "scope":"required_ecore_values_only",
                                    "invalid_count":issues.len()-unverified,
                                    "unverified_count":unverified,"issues":issues,
                                });
                            },
                            Err(error) => {
                                row["status"] = json!("blocked"); row["stage"] = json!("required_value_structure");
                                row["error"] = json!(format!("{error:?}"));
                            },
                        }
                    },
                    Err(error) => {
                        row["status"] = json!("blocked"); row["stage"] = json!("default_replay");
                        row["error"] = json!(error.to_string());
                    },
                }
            }
            row
        },
        Err(error)=>{
            let stage=match &error {
                E::Syntax(_)=>"syntax",E::Lexical(_)=>"lexical",E::Unsupported(_)=>"unsupported",
                E::ResourceLimit(_)=>"resource_limit",E::Artifact(_)=>"artifact",E::ConstructionOrLinking(_)=>"construction_or_linking",
            };
            json!({"status":"blocked","stage":stage,"error":error.to_string(),"source_bytes":source_bytes,"resource_mode":"separate"})
        }
    };
    if let Some(row)=profile_row {outcome["native_query_diagnostic"]=row;}
    outcome
}

/// Stage-local KIR persistence and replay; complete model transformation and
/// abstract-syntax equivalence are distinct obligations.
fn verify_default_replay(document: &KirDocument, owned_results: bool) -> Result<(),Box<dyn std::error::Error>> {
    use mercurio_sysml::definition_document::{type_default_contribution_owners,feature_default_contribution_owners,materialize_type_defaults,materialize_feature_defaults};
    let snapshot = serde_json::to_value(document)?;
    let mut restored: KirDocument = serde_json::from_value(snapshot.clone())?;
    if owned_results && mercurio_sysml::definition_document::materialize_owned_expression_results(&mut restored,"replay.expression-result")? != 0 {
        return Err("result replay selected additional members".into());
    }
    for type_stage in [true,false] {
        let owners=if type_stage {type_default_contribution_owners(&restored)}else{feature_default_contribution_owners(&restored)};
        let prefixes: Vec<_> = (0..owners.len()).map(|i|format!("replay.default.{}.{}",type_stage,i)).collect();
        let batch: Vec<_> = owners.iter().zip(&prefixes).map(|(id,prefix)|(id.as_str(),prefix.as_str())).collect();
        let inserted=if type_stage {materialize_type_defaults(&mut restored,&batch)?}else{materialize_feature_defaults(&mut restored,&batch)?};
        if inserted!=0 {return Err("default replay selected additional contributions".into());}
    }
    if serde_json::to_value(&restored)?!=snapshot {return Err("default replay changed the persisted model".into());}
    Ok(())
}

#[cfg(test)]
mod definition_candidate_tests {
    use super::*;
    #[test]
    fn literal_producer_audit_scopes_are_independent_and_unqualified() {
        let inputs=vec![("empty.kerml".into(),"package Empty;".into()),("value.kerml".into(),"package Values { feature x = 1; }".into())];
        let mut cache=std::collections::BTreeMap::new();
        let good=vec!["empty.kerml".into()];let missing=vec!["value.kerml".into()];
        let (plain,hit)=definition_candidate_cached("case",inputs.clone(),false,false,&good,&mut cache);assert!(!hit);
        assert_eq!(plain["status"],"constructed_unqualified");
        assert_eq!(plain["literal_value_bindings"]["full_model_transformation"],"not_assessed");
        assert_eq!(plain["semantic_qualification"],"not_assessed");
        let (blocked,hit)=definition_candidate_cached("case",inputs.clone(),false,false,&missing,&mut cache);assert!(!hit);
        assert_eq!(blocked["status"],"blocked");assert!(blocked.get("literal_value_bindings").is_none());
        let (replayed,hit)=definition_candidate_cached("case",inputs,false,false,&good,&mut cache);assert!(hit);assert_eq!(replayed,plain);
    }

    #[test]
    fn definition_structure_rows_never_claim_candidate_publication() {
        let row = definition_structure_case(&[("source.kerml".into(), "package P { function f { return result : Missing::T; } }".into())], &[StructureQuery {owner_id:None,owner_path:vec!["P".into(),"f".into()],field:"result".into()}, StructureQuery {owner_id:None,owner_path:vec!["P".into(),"f".into(),"result".into()],field:"type".into()}, StructureQuery {owner_id:None,owner_path:vec!["P".into(),"missing".into()],field:"result".into()}]);
        assert_eq!(row["status"], "unlinked_inspection");
        assert_eq!(row["publication"], "not_attempted");
        assert_eq!(row["semantic_qualification"], "not_assessed");
        assert!(row["pending_reference_count"].as_u64().unwrap() > 0);
        assert!(row["inspection"].get("elements").is_none());
        assert_eq!(row["queries"][0]["status"], "query_evaluated");
        assert_eq!(row["queries"][0]["targets"][0]["stored_direction"], "out");
        assert_eq!(row["queries"][1]["status"], "dependency_required");
        assert_eq!(row["queries"][1]["dependency"]["prerequisite"]["field"], "type");
        assert_eq!(row["queries"][2]["status"], "unavailable");
        assert_eq!(definition_structure_case(&[], &[])["status"], "infrastructure_error");
        assert_eq!(definition_structure_case(&[("source.txt".into(), "package P;".into())], &[])["status"], "infrastructure_error");
        assert_eq!(definition_structure_case(&[("source.kerml".into(), "package P {".into())], &[])["status"], "blocked");
    }

    #[test]
    fn definition_structure_query_dependencies_remain_structured_and_unqualified() {
        let row = definition_structure_case(&[("typed.kerml".into(), r#"
            standard library package Base { feature things; }
            standard library package Performances { function Evaluation { return inherited; } }
            package P { function F specializes Performances::Evaluation { return result : Missing::T; } }
        "#.into())], &[StructureQuery { owner_id: None, owner_path: vec!["P".into(), "F".into(), "result".into()], field: "type".into() }]);
        assert_eq!(row["status"], "unlinked_inspection");
        assert_eq!(row["publication"], "not_attempted");
        let query = &row["queries"][0];
        assert_eq!(query["status"], "dependency_required");
        assert_eq!(query["graph_qualification"], "not_assessed");
        assert_eq!(query["dependency"]["prerequisite"]["kind"], "read_field");
        assert_eq!(query["dependency"]["prerequisite"]["field"], "type");
        assert!(query.get("targets").is_none(), "a dependency is not an empty successful semantic answer");
        let exact = definition_structure_case(&[("typed.kerml".into(), r#"
            standard library package Base { feature things; }
            standard library package Performances { function Evaluation { return inherited; } }
            package P { function F specializes Performances::Evaluation { return result : Missing::T; } }
        "#.into())], &[StructureQuery { owner_path: vec![], owner_id: Some(query["owner_id"].as_str().unwrap().into()), field: "type".into() }]);
        assert_eq!(exact["queries"][0]["dependency"], query["dependency"], "exact anonymous identities use the same native query");

        assert!(row["inspection"]["pending_references"].as_array().unwrap().iter().any(|r|
            r["owner_id"] == query["dependency"]["prerequisite"]["owner_id"] && r["field"] == query["dependency"]["prerequisite"]["field"]));
    }

    #[test]
    fn owned_expression_result_stage_runs_before_defaults_and_stays_unqualified() {
        let inputs=[("expressions.kerml".into(),"package P { feature a; feature b = a; feature c = 1 + 2; }".into())];
        let row=definition_candidate_case_with_defaults("expressions.kerml",&inputs,true,true);
        assert_eq!(row["owned_expression_results"]["inserted"],2);
        assert_eq!(row["owned_expression_results"]["status"],"contributed_unqualified");
        assert_eq!(row["semantic_qualification"],"not_assessed");
        assert_eq!(row["transformation_completion"],"not_assessed");
        // Real missing library/default dependencies still block downstream stages.
        assert_eq!(row["status"],"blocked");
        let plain=definition_candidate_case("expressions.kerml",&inputs);
        assert!(plain.get("owned_expression_results").is_none());
    }

    #[test]
    fn composed_default_stage_preserves_qualification_boundary() {
        let inputs = [("base.kerml".into(),"standard library package Base { classifier Anything; feature things; } package P { feature x; }".into())];
        let row = definition_candidate_case_with_defaults("base.kerml",&inputs,true,true);
        assert_eq!(row["status"],"constructed_unqualified");
        assert_eq!(row["feature_defaults"]["inserted"],1);
        assert_eq!(row["default_replay"],"idempotent_after_kir_persistence");
        assert_eq!(row["required_values"]["scope"], "required_ecore_values_only");
        assert_eq!(row["required_values"]["unverified_count"], 0);
        assert_eq!(row["semantic_qualification"], "not_assessed");
        assert_eq!(row["transformation_completion"],"not_assessed");
        let missing = definition_candidate_case_with_defaults("missing.kerml",&[("missing.kerml".into(),"feature x;".into())],true,true);
        assert_eq!(missing["stage"],"feature_default_contributions");
    }

    #[test]
    fn type_default_stage_contributes_without_completing_or_qualifying() {
        let inputs = [("base.kerml".into(), "standard library package Base { classifier Anything; classifier T; }".into())];
        let row = definition_candidate_case_with_type_defaults("base.kerml", &inputs, true);
        assert_eq!(row["status"], "constructed_unqualified");
        assert_eq!(row["type_default_owner_count"], 2);
        assert_eq!(row["type_defaults"]["inserted"], 1);
        assert_eq!(row["transformation_completion"], "not_assessed");
        assert_eq!(row["semantic_qualification"], "not_assessed");
    }
    #[test]
    fn missing_type_default_dependency_is_a_separate_stage() {
        let row = definition_candidate_case_with_type_defaults("types.kerml", &[("types.kerml".into(), "class T;".into())], true);
        assert_eq!(row["status"], "blocked");
        assert_eq!(row["stage"], "type_default_contributions");
        assert_eq!(row["transformation_completion"], "not_assessed");
    }

    #[test]
    fn construction_is_not_reported_as_semantic_success() {
        let result=definition_candidate_case("simple.kerml",&[("simple.kerml".into(),"package P;".into())]);
        assert_eq!(result["status"],"constructed_unqualified");
    }
    #[test]
    fn source_set_language_is_selected_per_resource() {
        let result = definition_candidate_case("consumer.sysml", &[
            ("provider.kerml".into(), "package A { classifier T; }".into()),
            ("consumer.sysml".into(), "package B { part x : A::T; }".into()),
        ]);
        assert_eq!(result["status"], "constructed_unqualified");
        assert_eq!(result["resource_mode"], "separate");
        let invalid = definition_candidate_case("consumer.sysml", &[
            ("provider.kerml".into(), "package A { part def T; }".into()),
            ("consumer.sysml".into(), "package B { part x : A::T; }".into()),
        ]);
        assert_eq!(invalid["stage"], "syntax");
        assert!(invalid["error"].as_str().unwrap().contains("provider.kerml"));
    }
    #[test]
    fn duplicate_resource_cannot_silently_supply_two_units() {
        let result=definition_candidate_case("consumer.sysml", &[
            ("same.kerml".into(), "package A;".into()),
            ("same.kerml".into(), "package B;".into()),
        ]);
        assert_eq!(result["stage"], "artifact");
    }
    #[test]
    fn empty_library_cannot_silently_satisfy_a_source_set() {
        let result=definition_candidate_case("simple.kerml",&[("library.kerml".into(),String::new()),("simple.kerml".into(),"package P;".into())]);
        assert_eq!(result["status"],"infrastructure_error");
    }
}

/// Fresh-process project assessment: parse the shared source set once, compile every target.
fn assess_group(spec_path: &str, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let total_start = Instant::now();
    let spec: Spec = serde_json::from_str(&std::fs::read_to_string(spec_path)?)?;
    let first = spec.cases.first().ok_or("empty source set")?;
    let expected = first
        .input_files
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if spec.cases.iter().any(|case| {
        case.input_files
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            != expected
    }) {
        return Err("assessment input sets differ".into());
    }
    let start = Instant::now();
    let sysml = load_sysml_baseline()?;
    let kerml = load_kernel_baseline()?;
    let load_library_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let inputs = first
        .input_files
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path)?;
            let module = if path.ends_with(".kerml") {
                parse_kerml(&text)
            } else {
                parse_sysml(&text)
            }?;
            Ok(module)
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let read_parse_inputs_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let mut cases = Vec::new();
    let mut failed = false;
    for case in &spec.cases {
        let target = case.input_files.last().ok_or("missing target")?;
        let index = first
            .input_files
            .iter()
            .position(|path| path == target)
            .ok_or("target outside input set")?;
        let case_start = Instant::now();
        let compiled = if target.ends_with(".kerml") {
            compile_kerml_module_strict_with_context(
                &inputs[index],
                &case.relative_path,
                &inputs,
                &kerml,
            )
        } else {
            compile_sysml_module_with_context(&inputs[index], &case.relative_path, &inputs, &sysml)
        };
        let mut row = match compiled {
            Ok(document) => {
                json!({"status":"ok", "element_count":document.elements.len(), "diagnostics":[]})
            }
            Err(error) => {
                failed = true;
                diagnostic_result("compile", target, error, true)
            }
        };
        row["relative_path"] = json!(case.relative_path);
        row["compile_validate_ms"] = json!(case_start.elapsed().as_secs_f64() * 1000.0);
        cases.push(row);
    }
    let compile_validate_targets_ms = start.elapsed().as_secs_f64() * 1000.0;
    let result = json!({"engine_total_ms":total_start.elapsed().as_secs_f64()*1000.0,
        "phases":{"load_library_ms":load_library_ms,"read_parse_inputs_ms":read_parse_inputs_ms,
            "compile_validate_targets_ms":compile_validate_targets_ms},"input_files":first.input_files,"cases":cases});
    std::fs::write(output_path, serde_json::to_vec_pretty(&result)?)?;
    if failed {
        return Err("one or more assessment targets failed".into());
    }
    Ok(())
}

fn compile_case(
    name: &str,
    inputs: &[(String, String)],
    sysml: &KirDocument,
    kerml: &KirDocument,
) -> Value {
    // Both engines use the same input set, with the target last.
    let Some((target_path, target_text)) = inputs.last() else {
        return json!({"status":"infrastructure_error", "error":"missing target"});
    };
    let is_kerml = Path::new(name)
        .extension()
        .is_some_and(|ext| ext == "kerml");
    eprintln!("parsing target {target_path}");
    let target = if is_kerml {
        parse_kerml(target_text)
    } else {
        parse_sysml(target_text)
    };
    let target = match target {
        Ok(module) => module,
        Err(error) => return diagnostic_result("parse", target_path, error, false),
    };
    eprintln!("parsed target {target_path}");
    let mut context = Vec::new();
    for (path, text) in &inputs[..inputs.len() - 1] {
        eprintln!("parsing support {path}");
        let parsed = if Path::new(path)
            .extension()
            .is_some_and(|ext| ext == "kerml")
        {
            parse_kerml(text)
        } else {
            parse_sysml(text)
        };
        match parsed {
            Ok(module) => context.push(module),
            Err(error) => return diagnostic_result("support_parse", path, error, true),
        }
    }
    context.push(target.clone());
    eprintln!("resolving {name}");
    let compiled = if is_kerml {
        compile_kerml_module_strict_with_context(&target, name, &context, kerml)
    } else {
        compile_sysml_module_with_context(&target, name, &context, sysml)
    };
    match compiled {
        Ok(document) => {
            json!({"status":"ok", "parse_ok":true, "element_count":document.elements.len(), "diagnostics":[]})
        }
        Err(error) => diagnostic_result("compile", target_path, error, true),
    }
}

fn diagnostic_result(stage: &str, path: &str, diagnostic: Diagnostic, parse_ok: bool) -> Value {
    json!({"status":"error", "stage":stage, "input_path":path, "parse_ok":parse_ok, "diagnostics":[diagnostic]})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_partial_kir_and_uses_cross_file_context() {
        let library = load_sysml_baseline().unwrap();
        let missing = vec![(
            "target.sysml".into(),
            "package Demo { part def Good; part good: Good; part bad: Missing; }".into(),
        )];
        assert_eq!(
            compile_case("target.sysml", &missing, &library, &library)["status"],
            "error"
        );
        let supported = vec![
            (
                "types.sysml".into(),
                "package Types { part def Vehicle; }".into(),
            ),
            (
                "target.sysml".into(),
                "package Demo { part vehicle: Types::Vehicle; }".into(),
            ),
        ];
        assert_eq!(
            compile_case("target.sysml", &supported, &library, &library)["status"],
            "ok"
        );
        assert_eq!(
            compile_case("target.sysml", &supported[1..], &library, &library)["status"],
            "error"
        );
    }
    #[test]
    fn kerml_audit_rejects_unresolved_references() {
        let library = load_kernel_baseline().unwrap();
        let inputs = vec![(
            "missing.kerml".into(),
            "package P { feature value: Missing; }".into(),
        )];
        assert_eq!(
            compile_case("missing.kerml", &inputs, &library, &library)["status"],
            "error"
        );
    }

    #[test]
    fn syntax_failure_is_not_a_semantic_failure() {
        let library = KirDocument {
            metadata: Default::default(),
            elements: Vec::new(),
        };
        let inputs = vec![("bad.sysml".into(), "package Bad { part def".into())];
        let result = compile_case("bad.sysml", &inputs, &library, &library);
        assert_eq!(result["parse_ok"], false);
        assert_eq!(result["stage"], "parse");
    }
    #[test]
    fn definition_dependency_plan_tool_keeps_candidate_and_inspection_separate() {
        let inputs=vec![("library.kerml".into(),"package L { class A; }".into()),
            ("source.kerml".into(),"package P { feature selected : L::A; feature ignored : Missing; }".into())];
        let raw=super::definition_structure_case(&inputs,&[]);
        let nodes=raw["inspection"]["constructed_elements"].as_array().unwrap();
        let selected=nodes.iter().find(|e|e["properties"]["declared_name"]=="selected").unwrap();
        let port=raw["inspection"]["pending_references"].as_array().unwrap().iter().find(|p|
            nodes.iter().find(|e|e["id"]==p["owner_id"]).unwrap()["properties"]["owning_related_element"]==selected["id"]).unwrap();
        let roots=[mercurio_sysml::definition_document::DefinitionQueryPrerequisite::ReadField {
            owner_id:port["owner_id"].as_str().unwrap().into(),field:port["field"].as_str().unwrap().into()}];
        let linked=super::definition_structure_case_planned(&inputs,&[],Some(&roots));
        assert_eq!(linked["status"],"dependency_inspection");
        assert_eq!(linked["publication"],"not_attempted");
        assert_eq!(linked["semantic_qualification"],"not_assessed");
        assert_eq!(linked["inspection"]["transformation_completion"],"not_assessed");
        assert_eq!(linked["element_count"],raw["element_count"]);
        assert_eq!(linked["pending_reference_count"].as_u64().unwrap()+1,raw["pending_reference_count"].as_u64().unwrap());
        assert_eq!(linked["committed_reference_fields"].as_array().unwrap().len(),1);
        assert_eq!(linked["committed_reference_fields"][0]["owner_id"],port["owner_id"]);
        assert_eq!(raw["status"],"unlinked_inspection");
    }

    #[test]
    fn definition_dependency_plan_tool_rejects_bad_wave_without_publishable_payload() {
        let inputs=vec![("source.kerml".into(),"class A; feature selected : A;".into())];
        for roots in [vec![],vec![mercurio_sysml::definition_document::DefinitionQueryPrerequisite::ReadField {
            owner_id:"unregistered".into(),field:"type".into()}]] {
            let row=super::definition_structure_case_planned(&inputs,&[],Some(&roots));
            assert_eq!(row["status"],"blocked");
            assert_eq!(row["publication"],"not_attempted");
            assert!(row.get("inspection").is_none());
            assert!(row.get("model").is_none());
        }
    }

}


#[cfg(test)]
mod cached_definition_query_tests {
    use super::*;
    fn cached() -> Value {
        definition_structure_case(&[("cached.kerml".into(),
            "package P { class A; feature selected : Missing; }".into())], &[])
    }
    fn query(owner_id: &str, field: &str) -> StructureQuery {
        StructureQuery {owner_path: Vec::new(), owner_id: Some(owner_id.into()), field: field.into()}
    }

    #[test]
    fn cached_definition_queries_preserve_unqualified_phase_and_typed_pending_reads() {
        let row = cached();
        let nodes = row["inspection"]["constructed_elements"].as_array().unwrap();
        let selected = nodes.iter().find(|e| e["properties"]["declared_name"] == "selected").unwrap()["id"].as_str().unwrap();
        let result = query_definition_record_rows(row.clone(), &[query(selected,"type"),query("missing","type")]).unwrap();
        assert_eq!(result["status"],"read_only_diagnostic");
        assert_eq!(result["semantic_qualification"],"not_assessed");
        assert_eq!(result["publication"],"not_attempted");
        assert_eq!(result["qualification_certificate"],false);
        assert_eq!(result["constructed_elements_read"],row["element_count"]);
        assert_eq!(result["unfinished_reference_descriptors_retained"],row["pending_reference_count"]);
        assert_eq!(result["queries"][0]["status"],"dependency_required");
        assert_eq!(result["queries"][1]["status"],"unavailable");
        assert!(result.get("model").is_none());
    }

    #[test]
    fn cached_definition_queries_reject_phase_and_count_corruption() {
        let row = cached();
        for (field, value) in [("status",json!("ok")),("publication",json!("published")),
            ("semantic_qualification",json!("verified")),("element_count",json!(0)),
            ("pending_reference_count",json!(0))] {
            let mut corrupted = row.clone();
            corrupted[field] = value;
            assert!(query_definition_record_rows(corrupted,&[query("missing","type")]).is_err(),"{field}");
        }
        let mut corrupted = row;
        corrupted["inspection"]["schema"] = json!("unreviewed.schema");
        assert!(query_definition_record_rows(corrupted,&[query("missing","type")]).is_err());
    }

    #[test]
    fn cached_definition_queries_require_identity_and_check_optional_ownership_path() {
        let row = cached();
        assert!(query_definition_record_rows(row.clone(),&[]).is_err());
        assert!(query_definition_record_rows(row.clone(),&[query("","type")]).is_err());
        let owner = row["inspection"]["constructed_elements"].as_array().unwrap().iter()
            .find(|e|e["properties"]["declared_name"]=="A").unwrap()["id"].as_str().unwrap().to_owned();
        let mut request = query(&owner,"owned_feature_membership");
        request.owner_path = vec!["wrong".into()];
        let result = query_definition_record_rows(row,&[request]).unwrap();
        assert_eq!(result["queries"][0]["status"],"unavailable");
        assert!(result["queries"][0].get("targets").is_none());
    }
}

#[cfg(test)]
mod cached_definition_plan_tests {
    use super::*;
    use mercurio_sysml::definition_document::DefinitionQueryPrerequisite;
    fn fixture() -> (Value,Vec<DefinitionQueryPrerequisite>) {
        let paths=vec!["cached-library.kerml","cached-source.kerml"];
        let inputs=vec![(paths[0].into(),"package L { class A; }".into()),
            (paths[1].into(),"package P { feature selected : L::A; feature ignored : Missing; }".into())];
        let mut record=definition_structure_case(&inputs,&[]);
        record["input_files"]=json!(paths);
        let p=record["inspection"]["pending_references"].as_array().unwrap().iter().find(|p|p["spelling"]=="L::A").unwrap();
        let wave=vec![DefinitionQueryPrerequisite::ReadField {owner_id:p["owner_id"].as_str().unwrap().into(),field:p["field"].as_str().unwrap().into()}];
        (record,wave)
    }
    #[test]
    fn cached_definition_plan_tool_executes_native_ports_without_publication() {
        let (record,wave)=fixture();let output=plan_definition_record_row(record.clone(),&wave);
        assert_eq!(output["status"],"dependency_inspection");
        assert_eq!(output["element_count"],record["element_count"]);
        assert_eq!(output["pending_reference_count"].as_u64().unwrap()+1,record["pending_reference_count"].as_u64().unwrap());
        assert_eq!(output["committed_reference_fields"].as_array().unwrap().len(),1);
        assert_eq!(output["qualification_certificate"],false);
        assert_eq!(output["inspection"]["semantic_validation"],"not_assessed");
        assert!(output.get("model").is_none());
        let queried=query_definition_record_rows(output,&[StructureQuery {owner_path:vec![],owner_id:Some("absent".into()),field:"type".into()}]).unwrap();
        assert_eq!(queried["status"],"read_only_diagnostic");
    }
    #[test]
    fn cached_definition_plan_tool_rejects_phase_counts_registry_and_source_corruption() {
        let (record,wave)=fixture();
        for case in 0..8 {
            let mut bad=record.clone();
            match case {
                0=>bad["publication"]=json!("published"),
                1=>bad["semantic_qualification"]=json!("verified"),
                2=>bad["element_count"]=json!(0),
                3=>bad["pending_reference_count"]=json!(0),
                4=>bad["inspection"]["schema"]=json!("foreign"),
                5=>bad["inspection"]["pending_references"][0]["feature_id"]=json!("foreign"),
                6=>bad["input_files"]=json!(["foreign.kerml"]),
                7=>bad["inspection"]["transformation_completion"]=json!("complete"),
                _=>unreachable!(),
            }
            let output=plan_definition_record_row(bad,&wave);
            assert_eq!(output["status"],"blocked","case{case}");
            assert!(output.get("inspection").is_none());
            assert!(output.get("model").is_none());
            assert_eq!(output["attempted_reference_commits_discarded"],0);
        }
    }
    #[test]
    fn cached_definition_plan_tool_discards_real_commits_on_late_failure() {
        let (record,mut wave)=fixture();
        let p=record["inspection"]["pending_references"].as_array().unwrap().iter().find(|p|p["spelling"]=="Missing").unwrap();
        wave.push(DefinitionQueryPrerequisite::ReadField {owner_id:p["owner_id"].as_str().unwrap().into(),field:p["field"].as_str().unwrap().into()});
        let output=plan_definition_record_row(record,&wave);
        assert_eq!(output["status"],"blocked");
        assert_eq!(output["attempted_reference_commits_discarded"],1);
        assert!(output.get("inspection").is_none());
    }
}

#[cfg(test)]
mod cached_definition_context_tests {
    use super::*;
    use mercurio_sysml::definition_document::DefinitionQueryPrerequisite;

    fn inputs() -> Vec<(String,String)> {
        vec![("scope-library.kerml".into(),"package L { class A; }".into()),
            ("scope-accepted-\u{03b2}.kerml".into(),"package P { feature selected : L::A; }".into()),
            ("scope-rejected.kerml".into(),"package Q { feature rejected : Missing; }".into())]
    }
    fn fixture() -> Value {
        let sources=inputs();
        let mut record=definition_structure_case(&sources,&[]);
        record["input_files"]=json!(sources.iter().map(|p|&p.0).collect::<Vec<_>>());
        record
    }
    fn read(record:&Value,spelling:&str) -> DefinitionQueryPrerequisite {
        let pending=record["inspection"]["pending_references"].as_array().unwrap().iter()
            .find(|p|p["spelling"]==spelling).unwrap();
        DefinitionQueryPrerequisite::ReadField {owner_id:pending["owner_id"].as_str().unwrap().into(),
            field:pending["field"].as_str().unwrap().into()}
    }
    fn batch(record:&Value,spelling:&str,selected:Vec<String>) -> ConstructedDependencyBatch {
        ConstructedDependencyBatch {name:"context".into(),requirements:vec![read(record,spelling)],
            resource_uris:Some(selected)}
    }
    #[test]
    fn cached_definition_context_matches_fresh_native_execution_without_negative_companions() {
        let record=fixture();
        let sources=inputs();
        let wave=vec![read(&record,"L::A")];
        let context=batch(&record,"L::A",sources[..2].iter().map(|p|p.0.clone()).rev().collect());
        let output=plan_definition_record_batch(record.clone(),&context);
        let fresh=definition_structure_case_planned(&sources[..2],&[],Some(&wave));
        assert_eq!(output["status"],"dependency_inspection");
        assert_eq!(output["inspection"],fresh["inspection"]);
        assert_eq!(output["input_files"],json!(sources[..2].iter().map(|p|&p.0).collect::<Vec<_>>()));
        assert_eq!(output["resource_projection"]["excluded_resource_count"],1);
        assert_eq!(output["qualification_certificate"],false);
        assert_eq!(record,fixture());
        let all=plan_definition_record_row(record.clone(),&[read(&record,"L::A"),read(&record,"Missing")]);
        assert_eq!(all["status"],"blocked");
        assert_eq!(all["attempted_reference_commits_discarded"],1);
        assert!(all.get("inspection").is_none());
    }
    #[test]
    fn cached_definition_context_preserves_complete_nodes_ports_order_and_exact_uris() {
        let record=fixture();let sources=inputs();
        let selected=vec![sources[1].0.clone(),sources[0].0.clone()];
        let projection=project_definition_record(record.clone(),&selected).unwrap();
        let before=record["inspection"]["constructed_elements"].as_array().unwrap();
        let after=projection["inspection"]["constructed_elements"].as_array().unwrap();
        let selected_ids=after.iter().map(|n|n["id"].as_str().unwrap()).collect::<std::collections::BTreeSet<_>>();
        assert_eq!(*after,before.iter().filter(|n|selected_ids.contains(n["id"].as_str().unwrap()))
            .cloned().collect::<Vec<_>>());
        let pending=record["inspection"]["pending_references"].as_array().unwrap();
        assert_eq!(projection["inspection"]["pending_references"],
            json!(pending.iter().filter(|p|selected_ids.contains(p["owner_id"].as_str().unwrap())).collect::<Vec<_>>()));
        assert_eq!(projection["resource_projection"]["properties_rewritten"],false);
        assert_eq!(projection["resource_projection"]["identities_rewritten"],false);
    }
    #[test]
    fn cached_definition_context_rejects_real_missing_symbol_without_returning_partial_model() {
        let record=fixture();let sources=inputs();
        let output=plan_definition_record_batch(record.clone(),&batch(&record,"Missing",
            vec![sources[0].0.clone(),sources[2].0.clone()]));
        assert_eq!(output["status"],"blocked");
        assert_eq!(output["attempted_reference_commits_discarded"],0);
        let failures=output["failed_native_reference_reads"].as_array().unwrap();
        assert_eq!(failures.len(),1);
        assert_eq!(failures[0]["pending_descriptors"][0]["spelling"],"Missing");
        assert_eq!(failures[0]["owner_id"],serde_json::to_value(read(&record,"Missing")).unwrap()["owner_id"]);
        assert!(output.get("inspection").is_none());
        assert!(output.get("model").is_none());
        assert_eq!(output["resource_projection"]["selected_resource_count"],2);
    }
    #[test]
    fn cached_definition_context_rejects_unknown_empty_duplicate_or_normalized_selection() {
        let record=fixture();let sources=inputs();
        for selected in [vec![],vec!["foreign.kerml".into()],
            vec![sources[0].0.clone(),sources[0].0.clone()],
            vec![sources[0].0.to_uppercase()]] {
            let output=plan_definition_record_batch(record.clone(),&batch(&record,"L::A",selected));
            assert_eq!(output["status"],"blocked");
            assert_eq!(output["failure_stage"],"cache_projection");
            assert_eq!(output["attempted_reference_commits_discarded"],0);
            assert!(output.get("inspection").is_none());
        }
    }
    #[test]
    fn cached_definition_context_rejects_corrupt_original_registry_before_filtering() {
        let record=fixture();let sources=inputs();
        for case in 0..6 {
            let mut bad=record.clone();
            match case {
                0=>bad["element_count"]=json!(0),
                1=>bad["pending_reference_count"]=json!(0),
                2=>bad["inspection"]["constructed_elements"][0]["id"]=json!("foreign"),
                3=>bad["inspection"]["pending_references"][0]["owner_id"]=json!("foreign"),
                4=>bad["input_files"]=json!([sources[0].0,sources[0].0,sources[1].0]),
                5=>bad["publication"]=json!("published"),
                _=>unreachable!(),
            }
            let output=plan_definition_record_batch(bad,&batch(&record,"L::A",
                sources[..2].iter().map(|p|p.0.clone()).collect()));
            assert_eq!(output["status"],"blocked","case {case}");
            assert_eq!(output["failure_stage"],"cache_projection");
            assert!(output.get("inspection").is_none());
        }
    }
    #[test]
    fn cached_definition_context_does_not_use_unselected_resources_or_requirement_owners() {
        let record=fixture();let sources=inputs();
        let no_library=plan_definition_record_batch(record.clone(),&batch(&record,"L::A",vec![sources[1].0.clone()]));
        assert_eq!(no_library["status"],"blocked");
        assert!(no_library.get("inspection").is_none());
        let mut wrong_wave=batch(&record,"L::A",sources[..2].iter().map(|p|p.0.clone()).collect());
        wrong_wave.requirements.push(read(&record,"Missing"));
        let absent=plan_definition_record_batch(record,&wrong_wave);
        assert_eq!(absent["status"],"blocked");
        assert_eq!(absent["attempted_reference_commits_discarded"],0);
        assert!(absent.get("inspection").is_none());
    }
    #[test]
    fn cached_definition_context_cannot_drop_a_committed_cross_resource_endpoint() {
        let record=fixture();let sources=inputs();
        let linked=plan_definition_record_batch(record.clone(),&batch(&record,"L::A",
            sources[..2].iter().map(|p|p.0.clone()).collect()));
        assert_eq!(linked["status"],"dependency_inspection");
        let output=plan_definition_record_batch(linked,&batch(&record,"L::A",vec![sources[1].0.clone()]));
        assert_eq!(output["status"],"blocked");
        assert!(output["error"].as_str().unwrap().contains("invalid retained construction"));
        assert_eq!(output["attempted_reference_commits_discarded"],0);
        assert!(output.get("inspection").is_none());
    }
}
