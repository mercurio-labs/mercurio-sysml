//! Execute translated Pilot import validation over the collected namespace view.
use super::*;
use crate::validation_rules::{self, SemanticModel, Value as RuleValue};

const IMPORT_NAMESPACE: &str = "org.omg.sysml.lang.sysml.Import.getImportOwningNamespace()";
const ELEMENT_OWNER: &str = "org.omg.sysml.lang.sysml.Element.getOwner()";
const IMPORT_VISIBILITY: &str = "org.omg.sysml.lang.sysml.Import.getVisibility()";

pub(super) fn import_visibility(import: &CollectedImport) -> &str {
    if import.decl.is_expose {
        return crate::namespace_grammar::expose_visibility();
    }
    import
        .decl
        .modifiers
        .iter()
        .find(|m| matches!(m.as_str(), "private" | "protected" | "public"))
        .map(String::as_str)
        .unwrap_or("public")
}

struct ImportModel<'a> {
    has_namespace: bool,
    namespace_has_owner: bool,
    visibility: &'a str,
}

impl SemanticModel for ImportModel<'_> {
    fn supports_getter(&self, symbol: &str) -> bool {
        matches!(symbol, IMPORT_NAMESPACE | ELEMENT_OWNER | IMPORT_VISIBILITY)
    }
    fn get(&self, receiver: &str, symbol: &str) -> Result<RuleValue, String> {
        match (receiver, symbol) {
            ("import", IMPORT_NAMESPACE) => Ok(if self.has_namespace {
                RuleValue::object("namespace", "org.omg.sysml.lang.sysml.Namespace")
            } else {
                RuleValue::Null
            }),
            ("namespace", ELEMENT_OWNER) => Ok(if self.namespace_has_owner {
                RuleValue::object("namespace-owner", "org.omg.sysml.lang.sysml.Element")
            } else {
                RuleValue::Null
            }),
            ("import", IMPORT_VISIBILITY) => {
                let name = match self.visibility {
                    "private" => "PRIVATE",
                    "protected" => "PROTECTED",
                    "public" => "PUBLIC",
                    other => return Err(format!("invalid native import visibility {other}")),
                };
                Ok(RuleValue::Enum {
                    type_name: "org.omg.sysml.lang.sysml.VisibilityKind".into(),
                    name: name.into(),
                })
            }
            _ => Err(format!(
                "unimplemented import semantic read {receiver}.{symbol}"
            )),
        }
    }
}

pub(super) fn validate_import(import: &CollectedImport) -> Result<(), Diagnostic> {
    // The textual root is an unnamed Namespace with no owner. Every named
    // package/definition/usage scope has an owning element, including at root.
    let model = ImportModel {
        has_namespace: true,
        namespace_has_owner: import.owner_qualified_name.is_some(),
        visibility: import_visibility(import),
    };
    let diagnostics = validation_rules::evaluate("checkImport", "import", &model)
        .map_err(|message| Diagnostic::new(message, Some(import.decl.span.clone())))?;
    for diagnostic in diagnostics {
        if diagnostic.severity != "error"
            || diagnostic.subject.as_deref() != Some("import")
            || diagnostic.feature.is_some()
            || diagnostic.index.is_some()
            || !diagnostic.data.is_empty()
        {
            return Err(Diagnostic::new(
                "translated import diagnostic needs an unsupported native mapping",
                Some(import.decl.span.clone()),
            ));
        }
        return Err(Diagnostic::semantic(
            format!("{}: {}", diagnostic.code, diagnostic.message),
            Some(import.decl.span.clone()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_import_check_covers_visibility_and_owner_nullness() {
        for has_namespace in [false, true] {
            for namespace_has_owner in [false, true] {
                for visibility in ["private", "public", "protected"] {
                    let model = ImportModel {
                        has_namespace,
                        namespace_has_owner,
                        visibility,
                    };
                    let failures =
                        validation_rules::evaluate("checkImport", "import", &model).unwrap();
                    let invalid = has_namespace && !namespace_has_owner && visibility != "private";
                    assert_eq!(failures.len(), usize::from(invalid));
                    if invalid {
                        assert_eq!(failures[0].code, "validateImportTopLevelVisibility");
                        assert_eq!(failures[0].message, "Top level import must be private");
                        assert_eq!(failures[0].subject.as_deref(), Some("import"));
                        assert_eq!(failures[0].severity, "error");
                    }
                }
            }
        }
    }
}
