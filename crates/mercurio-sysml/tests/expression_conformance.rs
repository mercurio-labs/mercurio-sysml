//! Specification-informed scalar/sequence profile regressions, not full conformance.
//! Keep expected values in the fixture independent of evaluator output.
use mercurio_sysml::foundation::{
    ExpressionEvaluationContext, ExpressionEvaluationError, ExpressionIr, ExpressionPathSegment,
};
use mercurio_sysml::{compile_sysml_text, load_sysml_baseline};
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

#[test]
fn scalar_sequence_profile_agrees_across_attribute_guard_and_assignment_producers() {
    let stdlib = load_sysml_baseline().unwrap();
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/expression-conformance.json")).unwrap();
    for case in cases {
        let expression = case["expression"].as_str().unwrap();
        for producer in ["attribute", "guard", "assignment"] {
            let body = match producer {
                "attribute" => format!("attribute result = {expression};"),
                "guard" => format!(
                    "state s {{ state A; state B; transition t first A accept when {expression} then B; }}"
                ),
                _ => format!(
                    "state s {{ state A; state B; transition t first A accept start do assign x := {expression} then B; }}"
                ),
            };
            let source = format!(
                "package Audit {{ import ScalarValues::*; part def P {{ attribute x : Real = 8.0; attribute items : Real[0..*] = (1,2); {body} }} }}"
            );
            let compiled = compile_sysml_text(&source, "conformance.sysml", &stdlib);
            let label = format!("{} / {producer}: {expression}", case["id"]);
            if case["reject"] == true {
                assert!(compiled.is_err(), "unsupported syntax admitted: {label}");
                continue;
            }
            if case["error"] == true && compiled.is_err() {
                continue;
            }
            let document = compiled.unwrap_or_else(|error| panic!("{label}: {error}"));
            let ir = if producer == "assignment" {
                document
                    .elements
                    .iter()
                    .filter_map(|element| element.properties.get("effects"))
                    .filter_map(Value::as_array)
                    .flatten()
                    .find(|effect| effect["kind"] == "assign_expression")
                    .and_then(|effect| effect.get("expression"))
            } else {
                let name = if producer == "attribute" {
                    "result"
                } else {
                    "t"
                };
                document
                    .elements
                    .iter()
                    .find(|element| {
                        element
                            .properties
                            .get("declared_name")
                            .and_then(Value::as_str)
                            == Some(name)
                    })
                    .and_then(|element| element.properties.get("expression_ir"))
            }
            .unwrap_or_else(|| panic!("missing IR: {label}"));
            let ir =
                ExpressionIr::from_value(ir).unwrap_or_else(|error| panic!("{label}: {error}"));
            let evaluated = ir.evaluate(&mut Bindings);
            if case["error"] == true {
                assert!(evaluated.is_err(), "expected diagnostic: {label}");
            } else {
                let value = evaluated.unwrap_or_else(|error| panic!("{label}: {error}"));
                // JSON number tags differ for Real and Integer; compare numeric samples by value.
                if case["value"].as_u64().is_some_and(|n| n > (1_u64 << 53)) {
                    assert_eq!(
                        value.as_u64(),
                        case["value"].as_u64(),
                        "exact integer result: {label}"
                    );
                } else if value.is_number() && case["value"].is_number() {
                    assert_eq!(value.as_f64(), case["value"].as_f64(), "{label}");
                } else {
                    assert_eq!(value, case["value"], "{label}");
                }
            }
        }
    }
}
