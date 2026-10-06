//! Handwritten type-collection, metaclass and diagnostic bindings.
//! The complete seven caller predicates and helper body are generated.
use super::*;
use crate::validation_rules::{self, SemanticModel, Value as RuleValue};
const ALL_TYPES: &str = "org.omg.sysml.util.FeatureUtil.getAllTypesOf(org.omg.sysml.lang.sysml.Feature)";
const INSTANCE: &str = "java.lang.Class.isInstance(java.lang.Object)";

struct TypeModel<'a> {
    subject: &'a str,
    types: Vec<(String, String)>,
}
impl SemanticModel for TypeModel<'_> {
    fn supports_getter(&self, symbol: &str) -> bool { matches!(symbol, ALL_TYPES | INSTANCE) }
    fn get(&self, receiver: &str, symbol: &str) -> Result<RuleValue, String> {
        if receiver != self.subject || symbol != ALL_TYPES { return Err(format!("unsupported type-family read {receiver}.{symbol}")); }
        Ok(RuleValue::List(self.types.iter().map(|(id, kind)| RuleValue::object(id, kind)).collect()))
    }
    fn is_instance(&self, receiver: &str, required: &str) -> Result<bool, String> {
        let kind = self.types.iter().find(|(id, _)| id == receiver).map(|(_, kind)| kind)
            .ok_or_else(|| format!("unknown type object {receiver}"))?;
        let required = required.strip_prefix("org.omg.sysml.lang.sysml.")
            .ok_or_else(|| format!("unsupported model class {required}"))?;
        Ok(kind_conforms_to(kind, required))
    }
}

pub(super) fn owns_context(context: &str) -> bool {
    validation_rules::type_family_rules().iter().any(|(kind, _)|
        kind.rsplit('.').next() == context.rsplit("::").next())
}

pub(super) fn validate(usage: &ResolvedUsage, types: &[String], context: &ResolverContext, mappings: &MappingBundle) -> Result<(), Diagnostic> {
    let kind = mappings.metaclass_for(&usage.construct)?;
    let applicable: Vec<_> = validation_rules::type_family_rules().iter().filter(|(class, _)|
        kind_conforms_to(kind, class.rsplit('.').next().unwrap())).collect();
    if applicable.is_empty() { return Ok(()); }
    let mut objects = Vec::new();
    for id in types {
        let kind = if let Some(name) = id.strip_prefix("type.") {
            context.definition_index.get(name).map(|d| mappings.metaclass_for(&d.construct)).transpose()?
        } else { context.library_indexes.kinds.get(id).map(String::as_str) }
            .ok_or_else(|| Diagnostic::new(format!("missing metaclass for effective type {id}"), Some(usage.span.clone())))?;
        objects.push((id.clone(), kind.to_owned()));
    }
    let model = TypeModel {subject: &usage.qualified_name, types: objects};
    for (_, method) in applicable {
        let failures = validation_rules::evaluate(method, model.subject, &model)
            .map_err(|message| Diagnostic::new(message, Some(usage.span.clone())))?;
        for failure in failures {
            if failure.severity != "error" || failure.subject.as_deref() != Some(model.subject)
                || failure.feature.is_none() || failure.index.is_some() || !failure.data.is_empty() {
                return Err(Diagnostic::new("unsupported type-family diagnostic mapping", Some(usage.span.clone())));
            }
            // Native diagnostics currently carry declaration spans. Retain the
            // Ecore feature token in the message instead of silently discarding it.
            return Err(Diagnostic::semantic(format!("{}: {} [{}]", failure.code, failure.message, failure.feature.unwrap()), Some(usage.span.clone())));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_type_family_bindings_cover_cardinality_and_metaclasses() {
        for (class, method) in validation_rules::type_family_rules() {
            let required = class.rsplit('.').next().unwrap().replace("Usage", "Definition");
            let required = if required == "MetadataDefinition" { "Metaclass".to_string() } else {required};
            for (kinds, valid) in [(vec![], false), (vec![required.as_str()], true), (vec!["Namespace"], false), (vec![required.as_str(), required.as_str()], false)] {
                let types = kinds.iter().enumerate().map(|(i, k)| (format!("t{i}"), format!("SysML::{k}"))).collect();
                let model = TypeModel {subject: "subject", types};
                let failures = validation_rules::evaluate(method, "subject", &model).unwrap();
                assert_eq!(failures.is_empty(), valid, "{method} {kinds:?}");
                assert!(model.get("other", ALL_TYPES).is_err());
                assert!(model.is_instance("missing", class).is_err());
            }
        }
    }
}
