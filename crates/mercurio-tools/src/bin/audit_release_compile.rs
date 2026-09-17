//! Strict native release audit: partial KIR is not successful compilation.
use mercurio_core::KirDocument;
use mercurio_core::frontend::diagnostics::Diagnostic;
use mercurio_sysml::kerml::{compile_kerml_module_with_context, load_kernel_baseline, parse_kerml};
use mercurio_sysml::{compile_sysml_module_with_context, load_sysml_baseline, parse_sysml};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

#[derive(Deserialize)]
struct Spec {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    relative_path: String,
    input_files: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: audit_release_compile <spec.json> <output.jsonl>".into());
    }
    let spec: Spec = serde_json::from_str(&std::fs::read_to_string(&args[0])?)?;
    if spec.cases.is_empty() {
        return Err("empty audit corpus".into());
    }
    let sysml = load_sysml_baseline()?;
    let kerml = load_kernel_baseline()?;
    let mut output = std::fs::File::create(&args[1])?;
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
    let target = if is_kerml {
        parse_kerml(target_text)
    } else {
        parse_sysml(target_text)
    };
    let target = match target {
        Ok(module) => module,
        Err(error) => return diagnostic_result("parse", target_path, error, false),
    };
    let mut context = Vec::new();
    for (path, text) in &inputs[..inputs.len() - 1] {
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
    let compiled = if is_kerml {
        compile_kerml_module_with_context(&target, name, &context, kerml)
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
}
