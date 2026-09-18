//! Bounded executable calculation/constraint calls share the KIR evaluator.
use mercurio_sysml::foundation::{
    ExpressionEvaluationContext, ExpressionEvaluationError, ExpressionIr, ExpressionPathSegment,
};
use mercurio_sysml::{compile_sysml_text, load_sysml_baseline};
use serde_json::{Value, json};

#[derive(Default)]
struct Captures { offset_id: Option<String> }
impl ExpressionEvaluationContext for Captures {
    fn owner_id(&self) -> &str {
        "test"
    }
    fn resolve_path(
        &mut self,
        segments: &[ExpressionPathSegment],
    ) -> Result<Vec<Value>, ExpressionEvaluationError> {
        match segments.first() {
            Some(ExpressionPathSegment::Resolved {
                feature: Some(id), ..
            }) if self.offset_id.as_ref() == Some(id) => Ok(vec![json!(10)]),
            _ => Err(ExpressionEvaluationError::MissingBinding(format!(
                "{segments:?}"
            ))),
        }
    }
}
fn result(source: &str) -> Result<Value, String> {
    let stdlib = load_sysml_baseline().unwrap();
    let document = compile_sysml_text(source, "calls.sysml", &stdlib).map_err(|e| e.to_string())?;
    let value = document
        .elements
        .iter()
        .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("result"))
        .and_then(|e| e.properties.get("expression_ir"))
        .ok_or("missing result IR")?;
    let offset_id = document.elements.iter().find(|element| element.id.ends_with(".Library.offset")).map(|element| element.id.clone());
    ExpressionIr::from_value(value)
        .map_err(|e| e.to_string())?
        .evaluate(&mut Captures { offset_id })
        .map_err(|e| e.to_string())
}

#[test]
fn calculations_bind_positional_inputs_defaults_locals_and_return_contracts() {
    let source = r#"package Library { import ScalarValues::*;
        calc def Scale {
            in x: Integer[1];
            in factor: Integer[1] default = 2;
            attribute doubled: Integer[1] = x * factor;
            return answer: Integer[1] = doubled + 1;
        }
        attribute result = (Scale(3), Scale(3, 4), Scale(Scale(2)));
    }"#;
    assert_eq!(result(source).unwrap(), json!([7, 13, 11]));
}

#[test]
fn calculation_bare_result_and_declared_return_preserve_type_and_cardinality() {
    let source = r#"package Library { import ScalarValues::*;
        calc def Values { in x: Integer[1]; return values: Integer[2]; (x, x + 1) }
        attribute result = Values(4);
    }"#;
    assert_eq!(result(source).unwrap(), json!([4, 5]));
    assert!(
        result(&source.replace("Integer[2]", "Integer[1]"))
            .unwrap_err()
            .contains("multiplicity")
    );
    assert!(
        result(&source.replace("Values(4)", "Values(\"bad\")"))
            .unwrap_err()
            .contains("Integer")
    );
}

#[test]
fn call_defaults_are_ordered_by_dependencies_and_cycles_rejected() {
    let source = r#"package Library { import ScalarValues::*;
        calc def Values { in x: Integer default = y + 1; in y: Integer default = 3; x + y }
        attribute result = Values();
    }"#;
    assert_eq!(result(source).unwrap(), json!(7));
    // Definition defaults retain their lexical origin; supplied arguments run
    // in the caller's context even after topological binding ordering.
    let stdlib = load_sysml_baseline().unwrap();
    for (source, expected_lexical) in [(source.to_string(), 2), (source.replace("Values()", "Values(1)"), 1)] {
        let document = compile_sysml_text(&source, "origins.sysml", &stdlib).unwrap();
        let ir = document.elements.iter().find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("result"))
            .and_then(|e| e.properties.get("expression_ir")).unwrap();
        let bindings = ir["bindings"].as_array().unwrap();
        assert_eq!(bindings.iter().filter(|binding| binding["lexical"] == true).count(), expected_lexical);
    }

    assert!(
        result(&source.replace("default = 3", "default = x"))
            .unwrap_err()
            .contains("cyclic")
    );
    assert!(
        result(&source.replace("Values()", "Values(1,2,3)"))
            .unwrap_err()
            .contains("too many arguments")
    );
    assert!(
        result(&source.replace("in x: Integer default = y + 1", "in x: Integer"))
            .unwrap_err()
            .contains("missing argument")
    );
}

#[test]
fn calls_preserve_lexical_identity_and_can_invoke_constraints() {
    let source = r#"package Library { import ScalarValues::*;
        attribute offset: Integer = 10;
        calc def Add { in x: Integer[1]; x + offset }
        constraint def Positive { in x: Integer[1]; x > 0 }
        part def Caller { attribute offset: Integer = -100; attribute result = Positive(Add(2)); }
    }"#;
    assert_eq!(result(source).unwrap(), json!(true));
}

#[test]
fn cross_file_calls_use_library_import_scope_and_aliases() {
    let stdlib = load_sysml_baseline().unwrap();
    let library = mercurio_sysml::parse_sysml(
        r#"package Library { import ScalarValues::*;
        calc def Add { in x: Integer[1]; in y: Integer[1] default = x + 1; x + y }
        calc def Derived :> Add;
    }"#,
    )
    .unwrap();
    let source =
        "package Client { import Library::*; alias Sum for Derived; attribute result = Sum(4); }";
    let document = mercurio_sysml::compile_sysml_text_with_context(
        source,
        "client.sysml",
        &[library],
        &stdlib,
    )
    .unwrap();
    let value = document
        .elements
        .iter()
        .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("result"))
        .and_then(|e| e.properties.get("expression_ir"))
        .unwrap();
    assert_eq!(
        ExpressionIr::from_value(value)
            .unwrap()
            .evaluate(&mut Captures::default())
            .unwrap(),
        json!(9)
    );
    assert!(!document.elements.iter().any(|e| e.id == "type.Library.Add"));
}

#[test]
fn recursive_calls_and_fixed_parameter_overrides_report_diagnostics() {
    let source = r#"package Library { import ScalarValues::*;
        calc def Again { in x: Integer; Again(x) }
        attribute result = Again(1);
    }"#;
    let error = result(source).unwrap_err();
    assert!(error.contains("recursive"), "{error}");
    let fixed = r#"package Library { import ScalarValues::*;
        calc def Fixed { in x: Integer = 3; x + 1 }
        attribute result = Fixed();
    }"#;
    assert_eq!(result(fixed).unwrap(), json!(4));
    assert!(
        result(&fixed.replace("Fixed()", "Fixed(9)"))
            .unwrap_err()
            .contains("fixed parameter")
    );
}

#[test]
fn calls_agree_in_query_guard_and_assignment_producers() {
    let stdlib = load_sysml_baseline().unwrap();
    for (producer, body) in [
        ("query", "attribute result = Positive(Double(3));"),
        (
            "guard",
            "state s { state A; state B; transition result first A accept when Positive(Double(3)) then B; }",
        ),
        (
            "assignment",
            "attribute target: Boolean[1]; state s { state A; state B; transition result first A accept go do assign target := Positive(Double(3)) then B; }",
        ),
    ] {
        let source = format!(
            "package Library {{ import ScalarValues::*; calc def Double {{ in x: Integer[1]; x * 2 }} constraint def Positive {{ in x: Integer[1]; x > 0 }} part def P {{ {body} }} }}"
        );
        let document = compile_sysml_text(&source, "producer.sysml", &stdlib).unwrap();
        let element = document
            .elements
            .iter()
            .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("result"))
            .unwrap();
        let value = if producer == "assignment" {
            &element.properties["effects"][0]["expression"]
        } else {
            &element.properties["expression_ir"]
        };
        assert_eq!(
            ExpressionIr::from_value(value)
                .unwrap()
                .evaluate(&mut Captures::default())
                .unwrap(),
            json!(true),
            "{producer}"
        );
    }
}

#[test]
fn assignment_target_multiplicity_is_checked_after_invocation() {
    let stdlib = load_sysml_baseline().unwrap();
    let source = "package Library { import ScalarValues::*; calc def Pair { (1, 2) } part def P { attribute target: Integer[1]; state s { state A; state B; transition result first A accept go do assign target := Pair() then B; } } }";
    let document = compile_sysml_text(source, "assignment.sysml", &stdlib).unwrap();
    let element = document
        .elements
        .iter()
        .find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("result"))
        .unwrap();
    let ir = ExpressionIr::from_value(&element.properties["effects"][0]["expression"]).unwrap();
    assert!(
        ir.evaluate(&mut Captures::default())
            .unwrap_err()
            .to_string()
            .contains("multiplicity")
    );
}

#[test]
fn constraint_invocation_requires_one_boolean_result() {
    for declaration in ["Integer[1]", "Boolean[2]"] {
        let source = format!("package Library {{ import ScalarValues::*; constraint def C {{ return answer: {declaration}; true }} attribute result = C(); }}");
        let error = result(&source).unwrap_err();
        assert!(error.contains("Integer") || error.contains("multiplicity"), "{error}");
    }
    let source = "package Library { constraint def C { (true, false) } attribute result = C(); }";
    assert!(result(source).unwrap_err().contains("multiplicity"));
    assert!(
        result(&source.replace("(true, false)", "1"))
            .unwrap_err()
            .contains("Boolean")
    );
}

#[test]
fn constraint_local_features_do_not_consume_positional_arguments() {
    let source = "package Library { import ScalarValues::*; constraint def C { attribute limit: Integer = 3; in x: Integer; x > limit } attribute result = C(4); }";
    assert_eq!(result(source).unwrap(), json!(true));
}

#[test]
fn callable_compilation_budgets_reject_deep_calls_and_oversized_frames() {
    let mut definitions = String::from("calc def F0 { 1 } ");
    for index in 1..66 { definitions.push_str(&format!("calc def F{index} {{ F{}() }} ", index - 1)); }
    let source = format!("package Library {{ {definitions} attribute result = F65(); }}");
    let error = result(&source).unwrap_err();
    assert!(error.contains("64-call"), "{error}");

    let parameters = (0..129).map(|index| format!("in p{index}: Integer default = 0; ")).collect::<String>();
    let source = format!("package Library {{ import ScalarValues::*; calc def Many {{ {parameters} 1 }} attribute result = Many(); }}");
    let error = result(&source).unwrap_err();
    assert!(error.contains("128-binding"), "{error}");
}

#[test]
fn structural_bindings_preserve_narrowed_actual_contracts_and_defaults() {
    for (formal, actual) in [
        ("in x: Real[*];", ":>> x[1] = (1,2);"),
        ("in x: Real[*] default = (1,2);", ":>> x[1];"),
    ] {
        let source = format!("package Library {{ import ScalarValues::*; constraint def C {{ {formal} size(x) > 0 }} constraint result: C {{ {actual} }} }}");
        let error = result(&source).unwrap_err();
        assert!(error.contains("cardinality 2 violates multiplicity 1..1"), "{error}");
    }
}

#[test]
fn nested_callable_captures_participate_in_default_dependency_order_and_cycles() {
    let source = "package Library { import ScalarValues::*; calc def Outer { in z: Integer default = 2; in a: Integer default = Inner(); calc def Inner { z + 1 } a } attribute result = Outer(); }";
    assert_eq!(result(source).unwrap(), json!(3));
    let error = result(&source.replace("default = 2", "default = a")).unwrap_err();
    assert!(error.contains("cyclic"), "{error}");
}
