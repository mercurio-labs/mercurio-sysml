//! Export a raw analysis trace for cross-host acceptance, without host-side projection.
use std::{env, fs, path::PathBuf};
use mercurio_foundation::Runtime;
use mercurio_sysml::{compile_sysml_text, load_sysml_baseline};
use mercurio_sysml::simulation::{list_analysis_cases, run_analysis_case};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let source = PathBuf::from(args.next().ok_or("usage: mission_trace MODEL OUTPUT [CASE_LABEL]")?);
    let output = PathBuf::from(args.next().ok_or("missing output path")?);
    let label = args.next().unwrap_or_else(|| "HeatProfile".to_string());
    let text = fs::read_to_string(&source)?;
    let name = source.file_name().and_then(|s| s.to_str()).ok_or("invalid source filename")?;
    let document = compile_sysml_text(&text, name, &load_sysml_baseline()?)?;
    let runtime = Runtime::from_document(document)?;
    let case = list_analysis_cases(&runtime).into_iter().find(|case| case.label == label)
        .ok_or("analysis case not found")?;
    let report = run_analysis_case(&runtime, &case.id, "preview.acceptance")?;
    let traces = report.artifacts.iter().filter(|artifact| artifact.kind == "simulation_trace").collect::<Vec<_>>();
    if traces.len() != 1 { return Err("expected exactly one simulation trace".into()); }
    fs::write(output, serde_json::to_vec_pretty(&traces[0].payload)?)?;
    Ok(())
}
