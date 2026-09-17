//! Attribute native runtime construction for the same source/library as mission_trace.
//! Diagnostic timings only: debug/native results are not WASM performance estimates.
use mercurio_foundation::{KirDocument, Runtime};
use mercurio_sysml::{compile_sysml_text, load_sysml_baseline};
use std::{env, fs, path::PathBuf, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let source = PathBuf::from(args.next().ok_or("usage: mission_runtime_profile MODEL.sysml|MERGED.json OUTPUT")?);
    let output = PathBuf::from(args.next().ok_or("missing output path")?);
    let text = fs::read_to_string(&source)?;
    let name = source.file_name().and_then(|s| s.to_str()).ok_or("invalid source filename")?;
    let started = Instant::now();
    let baseline = load_sysml_baseline()?;
    // Browser session exports are runtime inputs, not interchange documents.
    // Match Runtime::from_document's existing deserialization boundary here;
    // this diagnostic does not change the validated KIR import API.
    let document: KirDocument = if source.extension().and_then(|s| s.to_str()) == Some("json") {
        serde_json::from_str(&text)?
    } else {
        compile_sysml_text(&text, name, &baseline)?
    };
    let compile_millis = started.elapsed().as_secs_f64() * 1000.0;
    let profile = Runtime::profile_from_document(document)?;
    let report = serde_json::json!({ "compile_millis": compile_millis, "runtime": profile,
        "scope": "Native diagnostic phase attribution; not browser timings or performance acceptance" });
    fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
