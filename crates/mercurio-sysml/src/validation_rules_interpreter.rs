//! Test-only reference interpreter for the translated validation IR.
//! Production execution uses validation_rules_generated.rs.
use super::{RuleDiagnostic, SemanticModel, Value, identity_equal};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Deserialize)]
struct RuleSet {
    schema_version: u32,
    rules: Vec<Rule>,
}

#[derive(Deserialize)]
struct Rule {
    id: String,
    name: String,
    parameter: Parameter,
    dependencies: Vec<String>,
    body: Vec<Statement>,
}

#[derive(Clone, Deserialize)]
struct Parameter {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Statement {
    Discard { value: Expression },
    Block {
        body: Vec<Statement>,
    },
    Let {
        name: String,
        value: Expression,
    },
    If {
        condition: Expression,
        then: Vec<Statement>,
        r#else: Vec<Statement>,
    },
    Diagnostic {
        severity: String,
        message: Expression,
        code: Expression,
        subject: Expression,
        feature: Expression,
        index: Option<i64>,
        data: Vec<Expression>,
    },
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Expression {
    Local {
        name: String,
    },
    Int { value: i32 },
    FeatureLiteral { identity: String, #[serde(rename = "type")] type_name: String },
    Length { operand: Box<Expression> },
    IntEq { left: Box<Expression>, right: Box<Expression> },
    IsInstance { operand: Box<Expression>, class_name: String },
    Exists { collection: Box<Expression>, parameter: Parameter, predicate: Box<Expression> },
    Null,
    Bool {
        value: bool,
    },
    String {
        value: String,
    },
    Enum {
        name: String,
        #[serde(rename = "type")]
        type_name: String,
    },
    Get {
        symbol: String,
        receiver: Box<Expression>,
    },
    Not {
        operand: Box<Expression>,
    },
    And {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Or {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    IdentityEq {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    IdentityNe {
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

type Locals = BTreeMap<String, Value>;

fn expression(
    expr: &Expression,
    locals: &Locals,
    model: &impl SemanticModel,
) -> Result<Value, String> {
    let eval = |expr: &Expression| expression(expr, locals, model);
    match expr {
        Expression::Local { name } => locals
            .get(name)
            .cloned()
            .ok_or_else(|| format!("unbound validation local {name}")),
        Expression::Int { value } => Ok(Value::Int(*value)),
        Expression::FeatureLiteral { identity, type_name } => Ok(Value::object(identity, type_name)),
        Expression::Length { operand } => Ok(Value::Int(i32::try_from(eval(operand)?.list()?.len()).map_err(|_| "collection length exceeds Java int")?)),
        Expression::IntEq { left, right } => Ok(Value::Bool(eval(left)?.integer()? == eval(right)?.integer()?)),
        Expression::IsInstance { operand, class_name } => Ok(Value::Bool(super::instance_of(eval(operand)?, class_name, model)?)),
        Expression::Exists { collection, parameter, predicate } => {
            for item in eval(collection)?.list()? {
                let mut nested = locals.clone();
                if nested.insert(parameter.name.clone(), item).is_some() { return Err("shadowed closure parameter".into()); }
                if expression(predicate, &nested, model)?.boolean()? { return Ok(Value::Bool(true)); }
            }
            Ok(Value::Bool(false))
        }
        Expression::Null => Ok(Value::Null),
        Expression::Bool { value } => Ok(Value::Bool(*value)),
        Expression::String { value } => Ok(Value::String(value.clone())),
        Expression::Enum { name, type_name } => Ok(Value::Enum {
            name: name.clone(),
            type_name: type_name.clone(),
        }),
        Expression::Get { symbol, receiver } => {
            let Value::Object { id, .. } = eval(receiver)? else {
                return Err(format!(
                    "null or non-object receiver for validation getter {symbol}"
                ));
            };
            model.get(&id, symbol)
        }
        Expression::Not { operand } => Ok(Value::Bool(!eval(operand)?.boolean()?)),
        Expression::And { left, right } => Ok(Value::Bool(
            eval(left)?.boolean()? && eval(right)?.boolean()?,
        )),
        Expression::Or { left, right } => Ok(Value::Bool(
            eval(left)?.boolean()? || eval(right)?.boolean()?,
        )),
        Expression::IdentityEq { left, right } => {
            Ok(Value::Bool(identity_equal(eval(left)?, eval(right)?)?))
        }
        Expression::IdentityNe { left, right } => {
            Ok(Value::Bool(!identity_equal(eval(left)?, eval(right)?)?))
        }
    }
}

fn statements(
    body: &[Statement],
    locals: &mut Locals,
    model: &impl SemanticModel,
    diagnostics: &mut Vec<RuleDiagnostic>,
) -> Result<(), String> {
    for statement in body {
        match statement {
            Statement::Discard { value } => { expression(value, locals, model)?; }
            Statement::Block { body } => {
                statements(body, &mut locals.clone(), model, diagnostics)?;
            }
            Statement::Let { name, value } => {
                let value = expression(value, locals, model)?;
                if locals.insert(name.clone(), value).is_some() {
                    return Err(format!("duplicate immutable validation local {name}"));
                }
            }
            Statement::If {
                condition,
                then,
                r#else,
            } => {
                let branch = if expression(condition, locals, model)?.boolean()? {
                    then
                } else {
                    r#else
                };
                statements(branch, &mut locals.clone(), model, diagnostics)?;
            }
            Statement::Diagnostic {
                severity,
                message,
                code,
                subject,
                feature,
                index,
                data,
            } => {
                if !matches!(severity.as_str(), "error" | "warning") {
                    return Err(format!(
                        "unsupported validation diagnostic severity {severity}"
                    ));
                }
                // Preserve the resolved error/warning call's argument order:
                // message, subject, feature, issue code, then varargs data.
                let message = expression(message, locals, model)?.string()?;
                let subject = expression(subject, locals, model)?.optional_identity()?;
                let feature = expression(feature, locals, model)?.optional_identity()?;
                let code = expression(code, locals, model)?.string()?;
                let data = data
                    .iter()
                    .map(|item| expression(item, locals, model)?.string())
                    .collect::<Result<_, _>>()?;
                diagnostics.push(RuleDiagnostic {
                    severity: severity.clone(),
                    message,
                    code,
                    subject,
                    feature,
                    index: *index,
                    data,
                });
            }
        }
    }
    Ok(())
}

fn execute(
    rule: &Rule,
    subject: &str,
    model: &impl SemanticModel,
) -> Result<Vec<RuleDiagnostic>, String> {
    for dependency in &rule.dependencies {
        if !model.supports_getter(dependency) {
            return Err(format!(
                "{} requires unimplemented semantic getter {dependency}",
                rule.id
            ));
        }
    }
    let mut locals = BTreeMap::from([(
        rule.parameter.name.clone(),
        Value::object(subject, &rule.parameter.type_name),
    )]);
    let mut diagnostics = Vec::new();
    statements(&rule.body, &mut locals, model, &mut diagnostics)?;
    Ok(diagnostics)
}

pub(super) fn evaluate(
    name: &str,
    subject: &str,
    model: &impl SemanticModel,
) -> Result<Vec<RuleDiagnostic>, String> {
    static RULES: OnceLock<Result<RuleSet, String>> = OnceLock::new();
    let rules = RULES
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../resources/metamodels/sysml-2.0-pilot-2026-08/validation-rules.extract.json"
            ))
            .map_err(|e| format!("invalid generated validation IR: {e}"))
        })
        .as_ref()
        .map_err(Clone::clone)?;
    if rules.schema_version != 1 {
        return Err("unsupported validation IR schema".into());
    }
    let mut candidates = rules.rules.iter().filter(|rule| rule.name == name);
    let rule = candidates
        .next()
        .ok_or_else(|| format!("missing translated validation rule {name}"))?;
    if candidates.next().is_some() {
        return Err(format!("ambiguous translated validation rule {name}"));
    }
    execute(rule, subject, model)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct NoReads;
    impl SemanticModel for NoReads {
        fn supports_getter(&self, _: &str) -> bool {
            false
        }
        fn get(&self, _: &str, _: &str) -> Result<Value, String> {
            panic!("short-circuit must avoid getter")
        }
    }
    #[test]
    fn short_circuit_does_not_read_null_receiver() {
        let bad = Expression::Get {
            symbol: "unsupported.getValue()".into(),
            receiver: Box::new(Expression::Null),
        };
        let no_read_and = Expression::And {
            left: Box::new(Expression::Bool { value: false }),
            right: Box::new(bad.clone()),
        };
        let no_read_or = Expression::Or {
            left: Box::new(Expression::Bool { value: true }),
            right: Box::new(bad),
        };
        assert_eq!(
            expression(&no_read_and, &Locals::new(), &NoReads).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            expression(&no_read_or, &Locals::new(), &NoReads).unwrap(),
            Value::Bool(true)
        );
    }
    #[test]
    fn missing_dependency_is_not_hidden_by_short_circuit() {
        let rule = Rule {
            id: "test#check".into(),
            name: "check".into(),
            parameter: Parameter {
                name: "element".into(),
                type_name: "Element".into(),
            },
            dependencies: vec!["Element.getMissing()".into()],
            body: Vec::new(),
        };
        assert!(
            execute(&rule, "subject", &NoReads)
                .unwrap_err()
                .contains("unimplemented semantic getter")
        );
    }
    #[test]
    fn identity_comparison_preserves_object_identity() {
        assert!(identity_equal(Value::object("one", "A"), Value::object("one", "B")).unwrap());
        assert!(!identity_equal(Value::object("one", "A"), Value::object("two", "A")).unwrap());
        assert!(identity_equal(Value::Bool(false), Value::Null).is_err());
        assert!(
            identity_equal(Value::String("same".into()), Value::String("same".into())).is_err()
        );
    }

    struct PilotInputs<'a>(&'a serde_json::Value);
    impl SemanticModel for PilotInputs<'_> {
        fn supports_getter(&self, symbol: &str) -> bool {
            matches!(
                symbol,
                "org.omg.sysml.lang.sysml.Import.getImportOwningNamespace()"
                    | "org.omg.sysml.lang.sysml.Element.getOwner()"
                    | "org.omg.sysml.lang.sysml.Import.getVisibility()"
                    | "org.omg.sysml.util.FeatureUtil.getAllTypesOf(org.omg.sysml.lang.sysml.Feature)"
                    | "java.lang.Class.isInstance(java.lang.Object)"
                    | "org.omg.sysml.lang.sysml.Usage.isReference()"
                    | "org.omg.sysml.lang.sysml.Definition.isVariation()"
                    | "org.omg.sysml.lang.sysml.Annotation.getOwnedAnnotatingElement()"
                    | "org.omg.sysml.lang.sysml.Annotation.getOwningAnnotatingElement()"
                    | "org.omg.sysml.lang.sysml.Annotation.getOwningAnnotatedElement()"
            )
        }
        fn get(&self, receiver: &str, symbol: &str) -> Result<Value, String> {
            let present = |key: &str, id: &str, class: &str| match self.0[key].as_bool() {
                Some(true) => Ok(Value::object(
                    id,
                    format!("org.omg.sysml.lang.sysml.{class}"),
                )),
                Some(false) => Ok(Value::Null),
                None => Err(format!("missing controlled input {key}")),
            };
            match (receiver, symbol) {
                ("typed-usage", "org.omg.sysml.util.FeatureUtil.getAllTypesOf(org.omg.sysml.lang.sysml.Feature)") => {
                    let kinds = self.0["type_kinds"].as_array().ok_or("missing controlled type list")?;
                    Ok(Value::List(kinds.iter().enumerate().map(|(i, kind)|
                        Value::object(format!("type-{i}"), format!("org.omg.sysml.lang.sysml.{}", kind.as_str().unwrap()))).collect()))
                }
                ("import", "org.omg.sysml.lang.sysml.Import.getImportOwningNamespace()") => {
                    present("namespace_present", "namespace", "Namespace")
                }
                ("namespace", "org.omg.sysml.lang.sysml.Element.getOwner()") => {
                    present("namespace_owner_present", "namespace-owner", "Element")
                }
                ("import", "org.omg.sysml.lang.sysml.Import.getVisibility()") => {
                    let name = self.0["visibility"].as_str().ok_or("missing visibility")?;
                    if !matches!(name, "private" | "public" | "protected") {
                        return Err(format!("unknown controlled visibility {name}"));
                    }
                    Ok(Value::Enum {
                        type_name: "org.omg.sysml.lang.sysml.VisibilityKind".into(),
                        name: name.to_uppercase(),
                    })
                }
                (
                    "annotation",
                    "org.omg.sysml.lang.sysml.Annotation.getOwnedAnnotatingElement()",
                ) => present(
                    "owned_annotating_element_present",
                    "owned-annotating",
                    "AnnotatingElement",
                ),
                (
                    "annotation",
                    "org.omg.sysml.lang.sysml.Annotation.getOwningAnnotatingElement()",
                ) => present(
                    "owning_annotating_element_present",
                    "owning-annotating",
                    "AnnotatingElement",
                ),
                (
                    "annotation",
                    "org.omg.sysml.lang.sysml.Annotation.getOwningAnnotatedElement()",
                ) => present(
                    "owning_annotated_element_present",
                    "owning-annotated",
                    "Element",
                ),
                ("reference", "org.omg.sysml.lang.sysml.Usage.isReference()") =>
                    self.0["is_reference"].as_bool().map(Value::Bool).ok_or("missing is_reference".into()),
                ("enumeration", "org.omg.sysml.lang.sysml.Definition.isVariation()") =>
                    self.0["is_variation"].as_bool().map(Value::Bool).ok_or("missing is_variation".into()),
                _ => Err(format!("unexpected controlled getter {receiver}.{symbol}")),
            }
        }
        fn is_instance(&self, receiver: &str, required: &str) -> Result<bool, String> {
            let index: usize = receiver.strip_prefix("type-").ok_or("invalid type identity")?.parse().map_err(|_| "invalid type index")?;
            let kind = self.0["type_kinds"][index].as_str().ok_or("missing controlled type")?;
            Ok(required == format!("org.omg.sysml.lang.sysml.{kind}"))
        }
    }

    #[test]
    fn translated_rules_match_actual_pilot_predicate_observations() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/conformance/2026-08-support/translated-validator-pilot-controls.json"
        ))
        .unwrap();
        let generated: serde_json::Value = serde_json::from_str(include_str!(
            "../resources/metamodels/sysml-2.0-pilot-2026-08/validation-rules.extract.json"
        ))
        .unwrap();
        assert_eq!(oracle["schema_version"], 1);
        assert_eq!(
            oracle["provenance"]["pilot_commit"],
            generated["source"]["pilot_revision"]
        );
        assert_eq!(
            oracle["provenance"]["jar_sha256"],
            generated["source"]["jar_sha256"]
        );
        assert_eq!(
            oracle["provenance"]["source_sha256"],
            generated["source"]["sources_sha256"]
        );
        let cases = oracle["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 52);
        let mut ids = std::collections::BTreeSet::new();
        let mut counts = BTreeMap::new();
        for case in cases {
            let id = case["id"].as_str().unwrap();
            assert!(ids.insert(id), "duplicate oracle case {id}");
            let method = case["method"].as_str().unwrap();
            *counts.entry(method).or_insert(0) += 1;
            let subject = match method {
                "checkImport" => "import",
                "checkAnnotation" => "annotation",
                "checkReferenceUsage" => "reference",
                "checkEnumerationDefinition" => "enumeration",
                "checkEnumerationUsage" | "checkAnalysisCaseUsage" | "checkVerificationCaseUsage" |
                "checkUseCaseUsage" | "checkRenderingUsage" | "checkViewpointUsage" | "checkMetadataUsage" => "typed-usage",
                other => panic!("unexpected oracle method {other}"),
            };
            let reference = evaluate(method, subject, &PilotInputs(&case["inputs"]))
                .unwrap_or_else(|error| panic!("{id} interpreter: {error}"));
            let actual = super::super::evaluate(method, subject, &PilotInputs(&case["inputs"]))
                .unwrap_or_else(|error| panic!("{id} generated Rust: {error}"));
            assert_eq!(actual, reference, "generated/interpreted mismatch for {id}");
            let actual: Vec<_> = actual
                .into_iter()
                .map(|diagnostic| {
                    serde_json::json!({
                        "severity": diagnostic.severity,
                        "code": diagnostic.code,
                        "message": diagnostic.message,
                        "subject": diagnostic.subject,
                        "feature": diagnostic.feature,
                        // Typed IR uses None for the four-argument error/warning overload;
                        // Xtext's acceptor receives INSIGNIFICANT_INDEX (-1) for that overload.
                        "index": diagnostic.index.unwrap_or(-1),
                        "data": diagnostic.data,
                    })
                })
                .collect();
            assert_eq!(serde_json::json!(actual), case["diagnostics"], "{id}");
        }
        assert_eq!(
            counts,
            BTreeMap::from([("checkAnnotation", 8), ("checkImport", 12), ("checkReferenceUsage", 2), ("checkEnumerationDefinition", 2), ("checkEnumerationUsage", 4), ("checkAnalysisCaseUsage", 4), ("checkVerificationCaseUsage", 4), ("checkUseCaseUsage", 4), ("checkRenderingUsage", 4), ("checkViewpointUsage", 4), ("checkMetadataUsage", 4)])
        );
    }

    #[test]
    fn standalone_blocks_preserve_lexical_scope() {
        let local = |value| Statement::Let {
            name: "value".into(),
            value: Expression::Bool { value },
        };
        let body = vec![
            Statement::Block {
                body: vec![local(false)],
            },
            Statement::Block {
                body: vec![local(true)],
            },
            local(true),
        ];
        let mut locals = Locals::new();
        statements(&body, &mut locals, &NoReads, &mut Vec::new()).unwrap();
        assert_eq!(
            locals,
            BTreeMap::from([("value".into(), Value::Bool(true))])
        );
    }

    #[test]
    fn diagnostic_arguments_follow_upstream_call_order() {
        struct Reads(std::cell::RefCell<Vec<String>>);
        impl SemanticModel for Reads {
            fn supports_getter(&self, _: &str) -> bool {
                true
            }
            fn get(&self, _: &str, symbol: &str) -> Result<Value, String> {
                self.0.borrow_mut().push(symbol.into());
                Ok(match symbol {
                    "subject" => Value::object("diagnostic-subject", "Element"),
                    "feature" => Value::Null,
                    other => Value::String(other.into()),
                })
            }
        }
        let read = |symbol: &str| Expression::Get {
            symbol: symbol.into(),
            receiver: Box::new(Expression::Local {
                name: "element".into(),
            }),
        };
        let rule = Rule {
            id: "test#diagnosticOrder".into(),
            name: "diagnosticOrder".into(),
            parameter: Parameter {
                name: "element".into(),
                type_name: "Element".into(),
            },
            dependencies: vec![],
            body: vec![Statement::Diagnostic {
                severity: "warning".into(),
                message: read("message"),
                code: read("code"),
                subject: read("subject"),
                feature: read("feature"),
                index: None,
                data: vec![read("data")],
            }],
        };
        let model = Reads(std::cell::RefCell::new(Vec::new()));
        let diagnostics = execute(&rule, "element", &model).unwrap();
        assert_eq!(
            *model.0.borrow(),
            ["message", "subject", "feature", "code", "data"]
        );
        assert_eq!(
            diagnostics[0].subject.as_deref(),
            Some("diagnostic-subject")
        );
        assert_eq!(diagnostics[0].data, ["data"]);
    }
}
