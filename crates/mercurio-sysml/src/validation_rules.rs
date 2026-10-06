//! Handwritten support for validators generated from resolved Pilot Xtend.
//!
//! Production predicates are compiled Rust. Their model getter implementations
//! remain explicit handwritten bindings. The JSON interpreter is test-only.
#[path = "validation_rules_generated.rs"]
mod generated;
#[cfg(test)]
#[path = "validation_rules_interpreter.rs"]
mod interpreter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Null,
    Int(i32),
    List(Vec<Value>),
    // The supported generated subset includes Boolean getters and locals.
    #[allow(dead_code)]
    Bool(bool),
    String(String),
    Enum {
        type_name: String,
        name: String,
    },
    Object {
        id: String,
        type_name: String,
    },
}

impl Value {
    pub(crate) fn object(id: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self::Object {
            id: id.into(),
            type_name: type_name.into(),
        }
    }
    #[allow(dead_code)] // Some selected predicate sets contain no Boolean getters.
    fn boolean(self) -> Result<bool, String> {
        if let Self::Bool(value) = self {
            Ok(value)
        } else {
            Err("validation expression requires Boolean".into())
        }
    }
    fn integer(self) -> Result<i32, String> {
        match self { Self::Int(value) => Ok(value), _ => Err("expected validation integer".into()) }
    }
    fn list(self) -> Result<Vec<Value>, String> {
        match self { Self::List(value) => Ok(value), _ => Err("expected validation collection".into()) }
    }
    fn string(self) -> Result<String, String> {
        if let Self::String(value) = self {
            Ok(value)
        } else {
            Err("validation diagnostic requires String".into())
        }
    }
    fn optional_identity(self) -> Result<Option<String>, String> {
        match self {
            Self::Null => Ok(None),
            Self::Object { id, .. } => Ok(Some(id)),
            _ => Err("validation diagnostic requires an object reference".into()),
        }
    }
}

pub(crate) trait SemanticModel {
    fn supports_getter(&self, symbol: &str) -> bool;
    fn get(&self, receiver_id: &str, symbol: &str) -> Result<Value, String>;
    fn is_instance(&self, _receiver_id: &str, _required_type: &str) -> Result<bool, String> {
        Err("unimplemented model metaclass service".into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuleDiagnostic {
    pub severity: String,
    pub message: String,
    pub code: String,
    pub subject: Option<String>,
    pub feature: Option<String>,
    pub index: Option<i64>,
    pub data: Vec<String>,
}

fn instance_of(value: Value, required: &str, model: &impl SemanticModel) -> Result<bool, String> {
    match value {
        Value::Null => Ok(false),
        Value::Object { id, .. } => model.is_instance(&id, required),
        _ => Err("instance check requires a model object".into()),
    }
}

fn identity_equal(left: Value, right: Value) -> Result<bool, String> {
    match (left, right) {
        (Value::Null, Value::Null) => Ok(true),
        (Value::Null, Value::Object { .. } | Value::Enum { .. })
        | (Value::Object { .. } | Value::Enum { .. }, Value::Null) => Ok(false),
        (Value::Object { id: left, .. }, Value::Object { id: right, .. }) => Ok(left == right),
        (
            Value::Enum {
                type_name: lt,
                name: ln,
            },
            Value::Enum {
                type_name: rt,
                name: rn,
            },
        ) => Ok(lt == rt && ln == rn),
        _ => Err("unsupported identity comparison types in validation IR".into()),
    }
}

/// Require all model services before executing a rule, including paths that a
/// particular input might short-circuit. A missing service is never an absent value.
fn require_getters(
    rule_id: &str,
    dependencies: &[&str],
    model: &impl SemanticModel,
) -> Result<(), String> {
    for dependency in dependencies {
        if !model.supports_getter(dependency) {
            return Err(format!(
                "{rule_id} requires unimplemented semantic getter {dependency}"
            ));
        }
    }
    Ok(())
}

fn read_getter(receiver: Value, symbol: &str, model: &impl SemanticModel) -> Result<Value, String> {
    let Value::Object { id, .. } = receiver else {
        return Err(format!(
            "null or non-object receiver for validation getter {symbol}"
        ));
    };
    model.get(&id, symbol)
}

pub(crate) fn type_family_rules() -> &'static [(&'static str, &'static str)] {
    generated::type_family_rules()
}

pub(crate) fn evaluate(
    name: &str,
    subject: &str,
    model: &impl SemanticModel,
) -> Result<Vec<RuleDiagnostic>, String> {
    generated::evaluate(name, subject, model)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct MissingGetter;
    impl SemanticModel for MissingGetter {
        fn supports_getter(&self, symbol: &str) -> bool {
            // Even if the absent namespace would short-circuit this input, the
            // rule requires a visibility service for other inputs.
            symbol != "org.omg.sysml.lang.sysml.Import.getVisibility()"
        }
        fn get(&self, _: &str, _: &str) -> Result<Value, String> {
            panic!("missing dependency must be rejected before any model reads")
        }
    }
    #[test]
    fn generated_rules_require_getters_before_any_model_access() {
        assert!(
            evaluate("checkImport", "import", &MissingGetter)
                .unwrap_err()
                .contains("unimplemented semantic getter")
        );
    }
    #[test]
    fn generated_dispatch_rejects_unknown_rules() {
        assert!(
            evaluate("unknown", "subject", &MissingGetter)
                .unwrap_err()
                .contains("unknown generated validation rule")
        );
    }
    #[test]
    fn null_getter_receiver_is_an_error() {
        assert!(
            read_getter(Value::Null, "getter", &MissingGetter)
                .unwrap_err()
                .contains("null or non-object receiver")
        );
    }
}
