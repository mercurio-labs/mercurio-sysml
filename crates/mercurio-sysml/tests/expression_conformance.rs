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

#[test]
fn constraint_definition_results_survive_parsing_lowering_and_evaluation() {
    let stdlib = load_sysml_baseline().unwrap();
    for (expression, expected) in [
        ("x > 0", true),
        ("x > 10", false),
        ("if x > 0 ? 1 + 1 == 2 else 1 / 0 > 0", true),
        ("SequenceFunctions::size((x, null, (1,2))) == 3", true),
    ] {
        let source = format!(
            "package Audit {{ import ScalarValues::*; constraint def C {{ doc /* Keep this predicate */ in x: Real; {expression} }} }}"
        );
        let parsed = mercurio_sysml::parse_sysml(&source).unwrap();
        let project = mercurio_sysml::foundation::authoring::AuthoringProject::from_parsed_modules(
            std::collections::BTreeMap::from([("definition.sysml".to_string(), parsed)]),
            std::collections::BTreeMap::new(),
        )
        .unwrap();
        let rendered = project.render_new_file("definition.sysml").unwrap();
        let roundtrip = compile_sysml_text(&rendered, "definition.sysml", &stdlib).unwrap();
        let document = compile_sysml_text(&source, "definition.sysml", &stdlib).unwrap();
        let definition = document
            .elements
            .iter()
            .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("C"))
            .unwrap();
        assert!(definition.kind.contains("ConstraintDefinition"));
        let value = definition
            .properties
            .get("expression_ir")
            .expect("definition result must not disappear");
        let restored = roundtrip
            .elements
            .iter()
            .find(|e| e.id == definition.id)
            .unwrap();
        let original_parameter = document
            .elements
            .iter()
            .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("x"))
            .unwrap();
        let restored_parameter = roundtrip
            .elements
            .iter()
            .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("x"))
            .unwrap();
        // Parameter IDs include source positions, which change during formatting.
        // All other IR structure must remain equal, and both IDs must name real elements.
        let expected_ir: Value = serde_json::from_str(
            &value
                .to_string()
                .replace(&original_parameter.id, &restored_parameter.id),
        )
        .unwrap();
        assert_eq!(
            restored.properties.get("expression_ir"),
            Some(&expected_ir),
            "{rendered}"
        );
        let ir = ExpressionIr::from_value(value).unwrap();
        assert_eq!(
            ir.evaluate(&mut Bindings).unwrap(),
            json!(expected),
            "{expression}"
        );
        let serialized = ir.to_value().unwrap().to_string();
        let parameter = document
            .elements
            .iter()
            .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("x"))
            .expect("definition input parameter");
        assert!(
            serialized.contains(&parameter.id),
            "predicate must bind the definition parameter: {serialized}"
        );
    }
}

#[test]
fn constraint_definition_results_reject_lossy_syntax_and_unbound_parameters() {
    let stdlib = load_sysml_baseline().unwrap();
    for expression in [
        "x[1] > 0",
        "x unexpected",
        "min(y=2,x=1)",
        "x > 0; x < 10",
        "missing > 0",
    ] {
        let source = format!(
            "package Audit {{ import ScalarValues::*; constraint def C {{ doc /* Comment */ in x: Real; {expression} }} }}"
        );
        assert!(
            compile_sysml_text(&source, "definition.sysml", &stdlib).is_err(),
            "{expression}"
        );
    }
}

#[test]
fn constraint_definition_result_binds_inherited_parameter_identity() {
    let stdlib = load_sysml_baseline().unwrap();
    let source = "package Audit { import ScalarValues::*; constraint def Base { in x: Real; } constraint def C :> Base { x > 0 } }";
    let document = compile_sysml_text(source, "inherited.sysml", &stdlib).unwrap();
    let definition = document
        .elements
        .iter()
        .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("C"))
        .unwrap();
    let parameter = document
        .elements
        .iter()
        .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("x"))
        .unwrap();
    let value = definition.properties.get("expression_ir").unwrap();
    assert_eq!(value["left"]["segments"][0]["feature"], parameter.id);
    assert_eq!(
        ExpressionIr::from_value(value)
            .unwrap()
            .evaluate(&mut Bindings)
            .unwrap(),
        json!(true)
    );
}

#[test]
fn typed_constraint_usage_binds_explicit_parameter_values() {
    let stdlib = load_sysml_baseline().unwrap();
    let source = "package Audit { import ScalarValues::*;
        constraint def Positive { in x: Real; x - 1 > 0 }
        constraint yes: Positive { :>> x = 3; }
        constraint no: Positive { :>> x = 0; }
        constraint missing: Positive;
    }";
    let document = compile_sysml_text(source, "bindings.sysml", &stdlib).unwrap();
    for (name, expected) in [("yes", Some(true)), ("no", Some(false)), ("missing", None)] {
        let usage = document.elements.iter().find(|e|
            e.properties.get("declared_name").and_then(Value::as_str) == Some(name)).unwrap();
        let value = usage.properties.get("expression_ir");
        if let Some(expected) = expected {
            let ir = ExpressionIr::from_value(value.expect(name)).unwrap();
            assert_eq!(ir.evaluate(&mut Bindings).unwrap(), json!(expected), "{name}");
        } else { assert!(value.is_none(), "unbound template must not become executable"); }
    }
}

#[test]
fn typed_constraint_binding_preserves_expressions_and_rejects_incomplete_applications() {
    let stdlib = load_sysml_baseline().unwrap();
    let source = "package Audit { import ScalarValues::*;
        attribute x: Real = 8;
        constraint def Range { in lower: Real; in upper: Real;
            if lower > 0 ? SequenceFunctions::size((lower, null, upper)) == 2 and lower < upper else false }
        constraint dynamic: Range { :>> lower = x - 2; :>> upper = x + 2; }
        constraint partial: Range { :>> lower = 2; }
        constraint unrelated: Range { attribute lower: Real = 2; attribute upper: Real = 3; }
        constraint def Fixed { in n: Real = 5; n > 0 }
        constraint fixed: Fixed { :>> n = -1; }
        constraint def Captured { x > 0 }
        constraint captured: Captured;
        constraint def Child :> Range;
        constraint inherited: Child { :>> lower = 2; :>> upper = 3; }
    }";
    let document = compile_sysml_text(source, "bindings.sysml", &stdlib).unwrap();
    for name in ["dynamic", "partial", "unrelated", "fixed", "inherited", "captured"] {
        let usage = document.elements.iter().find(|e|
            e.properties.get("declared_name").and_then(Value::as_str) == Some(name)).unwrap();
        let value = usage.properties.get("expression_ir");
        if name == "dynamic" || name == "inherited" {
            let ir = ExpressionIr::from_value(value.expect(name)).unwrap();
            assert_eq!(ir.evaluate(&mut Bindings).unwrap(), json!(true));
            assert!(!value.unwrap().to_string().contains("feature.Audit.Range."));
        } else { assert!(value.is_none(), "{name} must remain unevaluated"); }
    }
}

#[test]
fn typed_constraint_duplicate_bindings_are_rejected() {
    let stdlib = load_sysml_baseline().unwrap();
    let source = "package Audit { import ScalarValues::*;
        constraint def Positive { in x: Real; x > 0 }
        constraint duplicate: Positive { ref a redefines x = 2; ref b redefines x = -1; }
    }";
    let error = compile_sysml_text(source, "duplicate.sysml", &stdlib).unwrap_err();
    assert!(error.to_string().contains("duplicate emitted KIR id"), "{error}");
}

#[test]
fn typed_constraint_inherits_unchanged_predicate_through_a_chain() {
    let stdlib = load_sysml_baseline().unwrap();
    let source = "package Audit { import ScalarValues::*;
        constraint def Positive { in x: Real; x - 1 > 0 }
        constraint def Middle :> Positive;
        constraint def Leaf :> Middle;
        constraint yes: Leaf { :>> x = 8; }
        constraint no: Leaf { :>> x = 0; }
        constraint missing: Leaf;
        constraint def Changed :> Positive { x < 10 }
        constraint changed: Changed { :>> x = 8; }
        constraint def Extended :> Positive { in extra: Real; }
        constraint extended: Extended { :>> x = 8; :>> extra = 1; }
    }";
    let document = compile_sysml_text(source, "inherited-bindings.sysml", &stdlib).unwrap();
    for (name, expected) in [("yes", Some(true)), ("no", Some(false)), ("missing", None), ("changed", None), ("extended", None)] {
        let usage = document.elements.iter().find(|e|
            e.properties.get("declared_name").and_then(Value::as_str) == Some(name)).unwrap();
        let value = usage.properties.get("expression_ir");
        if let Some(expected) = expected {
            let ir = ExpressionIr::from_value(value.expect(name)).unwrap();
            assert_eq!(ir.evaluate(&mut Bindings).unwrap(), json!(expected), "{name}");
            assert!(!value.unwrap().to_string().contains("feature.Audit.Positive.x"));
        } else { assert!(value.is_none(), "{name} must remain unevaluated"); }
    }
}

#[test]
fn defaults_dependent_bindings_and_cycles_keep_their_semantics() {
    let stdlib = load_sysml_baseline().unwrap();
    let source = "package Audit { import ScalarValues::*;
        constraint def C { in x: Real default = 3; in y: Real default = x + 2; y == x + 2 }
        constraint defaults: C;
        constraint overridden: C { :>> x = 7; }
        constraint explicit: C { :>> x = 4; :>> y = x + 2; }
        constraint wrong: C { :>> y = 0; }
        constraint cycle: C { :>> x = y; :>> y = x; }
        constraint def Fixed { in x: Real = 3; x > 1 }
        constraint fixed: Fixed;
        constraint illegalOverride: Fixed { :>> x = 0; }
    }";
    for rendered in [false, true] {
        let source = if rendered {
            let parsed = mercurio_sysml::parse_sysml(source).unwrap();
            let project = mercurio_sysml::foundation::authoring::AuthoringProject::from_parsed_modules(
                std::collections::BTreeMap::from([("defaults.sysml".into(), parsed)]),
                std::collections::BTreeMap::new()).unwrap();
            let text = project.render_new_file("defaults.sysml").unwrap();
            assert!(text.contains("default ="), "{text}");
            text
        } else { source.to_string() };
        let document = compile_sysml_text(&source, "defaults.sysml", &stdlib).unwrap();
        for (name, expected) in [("defaults", Some(true)), ("overridden", Some(true)), ("explicit", Some(true)),
            ("wrong", Some(false)), ("cycle", None), ("fixed", Some(true)), ("illegalOverride", None)] {
            let usage = document.elements.iter().find(|e|
                e.properties.get("declared_name").and_then(Value::as_str) == Some(name)).unwrap();
            let ir = usage.properties.get("expression_ir");
            if let Some(expected) = expected {
                assert_eq!(ExpressionIr::from_value(ir.unwrap_or_else(|| panic!("{name}: {source}"))).unwrap().evaluate(&mut Bindings).unwrap(), json!(expected), "{name}");
            } else { assert!(ir.is_none(), "{name}"); }
        }
    }
}

#[test]
fn constraint_templates_cross_file_boundaries_and_keep_import_scope() {
    let stdlib = load_sysml_baseline().unwrap();
    let library = mercurio_sysml::parse_sysml("package Library { import ScalarValues::*;
        constraint def C { in x: Real default = 3; in y: Real default = x + 2; y > x }
        constraint def Derived :> C; }").unwrap();
    let source = "package Audit { import Library::*; constraint result: Derived { :>> x = 8; } }";
    let document = mercurio_sysml::compile_sysml_text_with_context(source, "client.sysml", &[library], &stdlib).unwrap();
    let usage = document.elements.iter().find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("result")).unwrap();
    let ir = usage.properties.get("expression_ir").unwrap();
    assert_eq!(ExpressionIr::from_value(ir).unwrap().evaluate(&mut Bindings).unwrap(), json!(true));
    assert!(!ir.to_string().contains("feature.Library."));
    assert!(!document.elements.iter().any(|e| e.id == "type.Library.C"), "context definitions must not be emitted into the client file");
}

#[test]
fn initializing_defaults_are_not_misread_as_binding_defaults() {
    let stdlib = load_sysml_baseline().unwrap();
    assert!(compile_sysml_text("package Audit { attribute x default := 3; }", "defaults.sysml", &stdlib).is_err());
}

#[test]
fn dependent_binding_expansion_has_a_finite_budget() {
    let stdlib = load_sysml_baseline().unwrap();
    let mut parameters = String::from("in x0: Real default 1;");
    for i in 1..20 { parameters.push_str(&format!(" in x{i}: Real default = x{} + x{};", i - 1, i - 1)); }
    let source = format!("package Audit {{ import ScalarValues::*; constraint def C {{ {parameters} x19 > 0 }} constraint result: C; }}");
    let document = compile_sysml_text(&source, "budget.sysml", &stdlib).unwrap();
    let usage = document.elements.iter().find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("result")).unwrap();
    assert!(!usage.properties.contains_key("expression_ir"));
}
