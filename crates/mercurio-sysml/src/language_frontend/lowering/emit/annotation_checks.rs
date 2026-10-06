//! Execute translated Pilot annotation checks over canonical emitted ownership.
use super::*;
use crate::validation_rules::{self, SemanticModel, Value as RuleValue};

const GETTERS: [(&str, &str, &str); 3] = [
    (
        "org.omg.sysml.lang.sysml.Annotation.getOwnedAnnotatingElement()",
        "owned_annotating_element",
        "org.omg.sysml.lang.sysml.AnnotatingElement",
    ),
    (
        "org.omg.sysml.lang.sysml.Annotation.getOwningAnnotatingElement()",
        "owning_annotating_element",
        "org.omg.sysml.lang.sysml.AnnotatingElement",
    ),
    (
        "org.omg.sysml.lang.sysml.Annotation.getOwningAnnotatedElement()",
        "owning_annotated_element",
        "org.omg.sysml.lang.sysml.Element",
    ),
];

struct AnnotationModel<'a>(&'a KirElement);
impl SemanticModel for AnnotationModel<'_> {
    fn supports_getter(&self, symbol: &str) -> bool {
        GETTERS.iter().any(|(getter, _, _)| *getter == symbol)
    }
    fn get(&self, receiver: &str, symbol: &str) -> Result<RuleValue, String> {
        if receiver != self.0.id {
            return Err(format!("unexpected annotation receiver {receiver}"));
        }
        let (_, field, type_name) = GETTERS
            .iter()
            .find(|(getter, _, _)| *getter == symbol)
            .ok_or_else(|| format!("unimplemented annotation getter {symbol}"))?;
        // These three optional relationships are completely materialized by
        // emission. Their omitted KIR field represents an absent relationship.
        // This convention applies only to these explicit adapter bindings.
        match self.0.properties.get(*field) {
            None | Some(Value::Null) => Ok(RuleValue::Null),
            Some(Value::String(id)) if !id.is_empty() => Ok(RuleValue::object(id, *type_name)),
            _ => Err(format!("invalid canonical annotation reference {field}")),
        }
    }
}

fn failures(element: &KirElement) -> Result<Vec<Diagnostic>, Diagnostic> {
    let span = element
        .properties
        .get("metadata")
        .and_then(|v| v.get("source_span"))
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let failures =
        validation_rules::evaluate("checkAnnotation", &element.id, &AnnotationModel(element))
            .map_err(|message| Diagnostic::new(message, span.clone()))?;
    failures
        .into_iter()
        .map(|failure| {
            if failure.severity != "error"
                || failure.subject.as_deref() != Some(element.id.as_str())
                || failure.feature.is_some()
                || failure.index.is_some()
                || !failure.data.is_empty()
            {
                return Err(Diagnostic::new(
                    "translated annotation diagnostic needs an unsupported native mapping",
                    span.clone(),
                ));
            }
            Ok(Diagnostic::semantic(
                format!("{}: {}", failure.code, failure.message),
                span.clone(),
            ))
        })
        .collect()
}

pub(super) fn validate(elements: &[KirElement]) -> Result<(), Diagnostic> {
    for element in elements
        .iter()
        .filter(|e| e.kind.rsplit("::").next() == Some("Annotation"))
    {
        if let Some(error) = failures(element)?.into_iter().next() {
            return Err(error);
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn annotation_checks_cover_all_ownership_combinations() {
        // Independent truth table: two legal forms, six invalid forms, including
        // combinations producing both pinned validation diagnostics.
        let expected = [
            vec!["annotating element"],
            vec!["owned by its annotated element"],
            vec![],
            vec!["annotating element", "owned by its annotated element"],
            vec!["annotating element", "own its annotating element"],
            vec![],
            vec!["own its annotating element"],
            vec!["annotating element"],
        ];
        for mask in 0..8 {
            let mut properties = BTreeMap::new();
            for (index, field) in [
                "owned_annotating_element",
                "owning_annotating_element",
                "owning_annotated_element",
            ]
            .iter()
            .enumerate()
            {
                if mask & (1 << index) != 0 {
                    properties.insert((*field).into(), json!("element"));
                }
            }
            let element = KirElement {
                id: "annotation".into(),
                kind: "SysML::Annotation".into(),
                layer: 2,
                properties,
            };
            let actual = failures(&element).unwrap();
            assert_eq!(actual.len(), expected[mask].len(), "mask {mask}");
            for (diagnostic, text) in actual.iter().zip(&expected[mask]) {
                assert!(
                    diagnostic.message.contains(text),
                    "mask {mask}: {diagnostic:?}"
                );
            }
            assert_eq!(validate(&[element]).is_ok(), mask == 2 || mask == 5);
        }
    }
}
