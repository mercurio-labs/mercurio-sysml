//! Executable explicit contracts, not complete effective-multiplicity semantics.
use mercurio_sysml::{compile_sysml_text, load_sysml_baseline};
use mercurio_sysml::foundation::{ExecutionContext, Runtime};
use serde_json::{Value, json};

#[test]
fn explicit_scalar_and_multiplicity_contracts_reach_the_runtime() {
    let stdlib = load_sysml_baseline().unwrap();
    for (declaration, expected) in [
        ("Real[2..4] = (10,20,30)", Ok(json!([10,20,30]))),
        ("Real[0..1] = null", Ok(json!([]))),
        ("Real[0] = null", Ok(json!([]))),
        ("Integer[1] = 4", Ok(json!(4))),
        ("Real[*] = (1,2,3)", Ok(json!([1,2,3]))),
        ("Real[1] = null", Err("multiplicity")),
        ("Real[2..4] = (1,2,3,4,5)", Err("multiplicity")),
        ("Integer[1] = 1.5", Err("type")),
        ("Integer[1] = 1.0", Err("type")),
        ("Natural[1] = -1", Err("type")),
        ("Positive[1] = 0", Err("type")),
        ("Boolean[1] = 1", Err("type")),
        ("String[1] = true", Err("type")),
    ] {
        let source = format!("package Contracts {{ import ScalarValues::*; attribute result : {declaration}; }}");
        let document = compile_sysml_text(&source, "contracts.sysml", &stdlib).unwrap();
        let feature = document.elements.iter().find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("result")).unwrap();
        let id = feature.id.clone();
        let owner = feature.properties["owner"].as_str().unwrap().to_string();
        let runtime = Runtime::from_document(document).unwrap();
        let result = runtime.evaluate(&id, &owner, &ExecutionContext::default());
        match expected {
            Ok(value) => assert_eq!(result.unwrap().value, value, "{source}"),
            Err(message) => assert!(result.unwrap_err().to_string().to_lowercase().contains(message), "{source}"),
        }
    }
}

#[test]
fn dynamic_and_inverted_explicit_bounds_do_not_silently_execute() {
    let stdlib = load_sysml_baseline().unwrap();
    for bounds in ["limit", "4..2"] {
        let source = format!("package Contracts {{ import ScalarValues::*; attribute limit: Integer = 3; attribute result: Real[{bounds}] = (1,2); }}");
        assert!(compile_sysml_text(&source, "bounds.sysml", &stdlib).is_err(), "{source}");
    }
}

#[test]
fn lexical_capture_uses_resolved_identity_through_the_real_runtime() {
    let stdlib = load_sysml_baseline().unwrap();
    let source = "package Harness { import ScalarValues::*; attribute offset: Integer = 10; calc def Add { in x: Integer[1]; x + offset } part def Caller { attribute offset: Integer = -100; attribute result = Add(2); } }";
    let document = compile_sysml_text(source, "captures.sysml", &stdlib).unwrap();
    let feature = document.elements.iter().find(|e| e.properties.get("declared_name").and_then(Value::as_str) == Some("result")).unwrap();
    let id = feature.id.clone();
    let owner = feature.properties["owner"].as_str().unwrap().to_string();
    let runtime = Runtime::from_document(document).unwrap();
    assert_eq!(runtime.evaluate(&id, &owner, &ExecutionContext::default()).unwrap().value, json!(12));
}
