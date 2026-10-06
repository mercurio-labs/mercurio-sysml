//! Handwritten native scalar bindings; predicates come from resolved Xtend.
use super::*;
use crate::validation_rules::{self, SemanticModel, Value};

const REFERENCE: &str = "org.omg.sysml.lang.sysml.Usage.isReference()";
const VARIATION: &str = "org.omg.sysml.lang.sysml.Definition.isVariation()";

struct ScalarModel<'a> {
    subject: &'a str,
    getter: &'a str,
    value: Option<bool>,
}
impl SemanticModel for ScalarModel<'_> {
    fn supports_getter(&self, symbol: &str) -> bool { symbol == self.getter }
    fn get(&self, receiver: &str, symbol: &str) -> Result<Value, String> {
        if receiver != self.subject || symbol != self.getter {
            return Err(format!("unsupported scalar read {receiver}.{symbol}"));
        }
        self.value.map(Value::Bool).ok_or_else(|| format!("missing derived scalar {} for {}", self.getter, self.subject))
    }
}

fn validate_one(method: &str, model: ScalarModel<'_>, span: &SourceSpan) -> Result<(), Diagnostic> {
    let failures = validation_rules::evaluate(method, model.subject, &model)
        .map_err(|message| Diagnostic::new(message, Some(span.clone())))?;
    for failure in failures {
        if failure.severity != "error" || failure.subject.as_deref() != Some(model.subject)
            || failure.feature.is_some() || failure.index.is_some() || !failure.data.is_empty() {
            return Err(Diagnostic::new("unsupported translated scalar diagnostic mapping", Some(span.clone())));
        }
        return Err(Diagnostic::semantic(format!("{}: {}", failure.code, failure.message), Some(span.clone())));
    }
    Ok(())
}

pub(super) fn validate(module: &ResolvedModule, mappings: &MappingBundle) -> Result<(), Diagnostic> {
    for definition in &module.definitions {
        if kind_conforms_to(mappings.metaclass_for(&definition.construct)?, "EnumerationDefinition") {
            validate_one("checkEnumerationDefinition", ScalarModel {
                subject: &definition.qualified_name, getter: VARIATION, value: Some(definition.is_variation),
            }, &definition.span)?;
        }
    }
    let mut pending: Vec<_> = module.usages.iter()
        .chain(module.definitions.iter().flat_map(|definition| &definition.members))
        .chain(module.aliases.iter().flat_map(|alias| &alias.members))
        .chain(module.imports.iter().flat_map(|import| &import.members)).collect();
    while let Some(usage) = pending.pop() {
        pending.extend(&usage.members);
        if kind_conforms_to(mappings.metaclass_for(&usage.construct)?, "ReferenceUsage") {
            validate_one("checkReferenceUsage", ScalarModel {
                subject: &usage.qualified_name, getter: REFERENCE,
                value: usage.derived_properties.get("is_reference").copied(),
            }, &usage.span)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_scalar_bindings_cover_flags_and_reject_missing_values() {
        for (method, getter, code) in [
            ("checkReferenceUsage", REFERENCE, "validateAttributeUsageIsReferential"),
            ("checkEnumerationDefinition", VARIATION, "validateEnumerationDefinitionIsVariation"),
        ] {
            for flag in [false, true] {
                let model = ScalarModel {subject: "subject", getter, value: Some(flag)};
                let diagnostics = validation_rules::evaluate(method, "subject", &model).unwrap();
                assert_eq!(diagnostics.len(), usize::from(!flag));
                if !flag { assert_eq!(diagnostics[0].code, code); }
                assert!(model.get("other", getter).is_err());
                assert!(model.get("subject", "unknown").is_err());
            }
            let model = ScalarModel {subject: "subject", getter, value: None};
            assert!(validation_rules::evaluate(method, "subject", &model).unwrap_err().contains("missing derived scalar"));
        }
    }
}
