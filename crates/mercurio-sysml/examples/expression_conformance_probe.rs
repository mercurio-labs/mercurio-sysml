//! Read-only audit: record parser, compiler, and shared-evaluator behavior.
//! Usage: cargo run -p mercurio-sysml --example expression_conformance_probe -- probes.json results.json
use mercurio_sysml::foundation::{
    ExpressionEvaluationContext, ExpressionEvaluationError, ExpressionIr, ExpressionPathSegment,
};
use mercurio_sysml::{compile_sysml_text, load_sysml_baseline, parse_sysml};
use serde_json::{Value, json};
struct Bindings;
impl ExpressionEvaluationContext for Bindings {
    fn owner_id(&self) -> &str {
        "Audit"
    }
    fn resolve_path(
        &mut self,
        segments: &[ExpressionPathSegment],
    ) -> Result<Vec<Value>, ExpressionEvaluationError> {
        let path = segments
            .iter()
            .map(ExpressionPathSegment::name)
            .collect::<Vec<_>>()
            .join(".");
        match path.as_str() {
            "x" => Ok(vec![json!(8.0)]),
            "items" => Ok(vec![json!(1), json!(2)]),
            _ => Err(ExpressionEvaluationError::MissingBinding(path)),
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let probes: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(
        args.get(1).ok_or("probes path required")?,
    )?)?;
    let stdlib = load_sysml_baseline()?;
    let mut results = Vec::new();
    for probe in probes {
        let expression = probe["expression"].as_str().ok_or("expression required")?;
        let source = format!(
            "package Audit {{ import ScalarValues::*; attribute x : Real = 8.0; attribute items : Real[0..*] = (1, 2); attribute result = {expression}; }}"
        );
        let mut row = json!({"id":probe["id"],"expression":expression});
        match parse_sysml(&source) {
            Ok(_) => row["parse"] = json!("accepted"),
            Err(error) => {
                row["parse"] = json!("rejected");
                row["parse_error"] = json!(error.to_string());
            }
        }
        match compile_sysml_text(&source, "expression-conformance.sysml", &stdlib) {
            Ok(document) => {
                row["compile"] = json!("accepted");
                let element = document.elements.iter().find(|e| {
                    e.properties.get("declared_name").and_then(Value::as_str) == Some("result")
                });
                if let Some(element) = element {
                    row["properties"] = json!(element.properties);
                    if let Some(value) = element.properties.get("expression_ir") {
                        row["ir"] = value.clone();
                        match ExpressionIr::from_value(value) {
                            Ok(ir) => match ir.evaluate(&mut Bindings) {
                                Ok(value) => row["evaluation"] = json!({"value":value}),
                                Err(error) => {
                                    row["evaluation"] = json!({"error":error.to_string()})
                                }
                            },
                            Err(error) => {
                                row["evaluation"] = json!({"decode_error":error.to_string()})
                            }
                        }
                    }
                }
            }
            Err(error) => {
                row["compile"] = json!("rejected");
                row["compile_error"] = json!(error.to_string());
            }
        }
        let dynamic_source = format!(
            "package Audit {{ import ScalarValues::*; part def P {{ attribute x : Real = 8.0; attribute items : Real[0..*] = (1, 2); state s {{ state A; state B; transition t first A accept when {expression} then B; }} }} }}"
        );
        match compile_sysml_text(
            &dynamic_source,
            "expression-conformance-guard.sysml",
            &stdlib,
        ) {
            Ok(document) => {
                row["guard_compile"] = json!("accepted");
                let ir = document
                    .elements
                    .iter()
                    .filter(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("t"))
                    .find_map(|e| e.properties.get("expression_ir"));
                row["guard_ir"] = ir.cloned().unwrap_or(Value::Null);
            }
            Err(error) => {
                row["guard_compile"] = json!("rejected");
                row["guard_compile_error"] = json!(error.to_string());
            }
        }
        results.push(row);
    }
    std::fs::write(
        args.get(2).ok_or("results path required")?,
        serde_json::to_vec_pretty(&results)?,
    )?;
    println!(
        "Recorded {} expression probes; this is observed behavior, not a conformance pass count.",
        results.len()
    );
    Ok(())
}
