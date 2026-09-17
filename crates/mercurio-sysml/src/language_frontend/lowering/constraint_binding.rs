//! Instantiate the bounded, explicitly bound subset of constraint predicates.
//! Identity-based substitution happens before KIR emission, so every host uses
//! the shared evaluator. Incomplete/unsupported applications retain no result.
use std::collections::{BTreeMap, BTreeSet};

use super::ir::{ResolvedDefinition, ResolvedExpr, ResolvedUsage};

pub(super) fn bind_constraint_usages(
    definitions: &mut [ResolvedDefinition],
    usages: &mut [ResolvedUsage],
) {
    let templates: BTreeMap<_, _> = definitions
        .iter()
        .filter(|d| d.construct == "ConstraintDefinition")
        .map(|d| (format!("type.{}", d.qualified_name), d.clone()))
        .collect();
    for definition in definitions {
        visit(&mut definition.members, &templates);
    }
    visit(usages, &templates);
}

// Follow only transparent single-parent specializations. A local body can
// redefine parameters/results or add constraints, so it requires fuller
// composition semantics and must not be silently skipped.
fn effective_template<'a>(
    id: &str,
    templates: &'a BTreeMap<String, ResolvedDefinition>,
) -> Option<&'a ResolvedDefinition> {
    let mut template = templates.get(id)?;
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(&template.qualified_name) {
            return None;
        }
        if template.specializes.is_empty() {
            return Some(template);
        }
        if template.expression.is_some() || !template.members.is_empty() {
            return None;
        }
        let [parent] = template.specializes.as_slice() else {
            return None;
        };
        template = templates.get(parent)?;
    }
}

fn visit(usages: &mut [ResolvedUsage], templates: &BTreeMap<String, ResolvedDefinition>) {
    for usage in usages {
        visit(&mut usage.members, templates);
        // A local result or multiple types requires composition semantics, which
        // this bounded instantiation path intentionally does not invent.
        if !(usage.construct == "ConstraintUsage"
            || (usage.construct == "RequireUsage"
                && usage.modifiers.iter().any(|m| m == "constraint")))
            || usage.expression.is_some()
            || usage.multiplicity.is_some()
            || !usage.additional_type_refs.is_empty()
            || !usage.specializes.is_empty()
            || !usage.redefined_features.is_empty()
            || !usage.subsetted_features.is_empty()
        {
            continue;
        }
        let Some(template) = usage
            .type_ref
            .as_ref()
            .and_then(|id| effective_template(id, templates))
        else {
            continue;
        };
        let Some(predicate) = &template.expression else {
            continue;
        };
        if usage.members.len() != template.members.len() {
            continue;
        }
        let mut bindings = BTreeMap::new();
        let mut supported = true;
        for formal in &template.members {
            let id = format!("feature.{}", formal.qualified_name);
            let actuals: Vec<_> = usage
                .members
                .iter()
                .filter(|m| m.redefined_features.contains(&id))
                .collect();
            // Require one explicit value per formal. Default values, unbound
            // parameters, and duplicate redefinitions need further elaboration.
            let [actual] = actuals.as_slice() else {
                supported = false;
                break;
            };
            let Some(value) = &actual.expression else {
                supported = false;
                break;
            };
            if formal.expression.is_some()
                || !formal.members.is_empty()
                || !matches!(
                    formal.construct.as_str(),
                    "ReferenceUsage" | "AttributeUsage"
                )
                || actual.redefined_features.len() != 1
                || !actual.members.is_empty()
                || actual.modifiers.iter().any(|m| m == "default")
                || references_scope(value, &usage.qualified_name)
                || references_scope(value, &template.qualified_name)
            {
                supported = false;
                break;
            }
            bindings.insert(id, value.clone());
        }
        if supported {
            usage.expression = substitute(predicate, &bindings)
                .filter(|result| !references_scope(result, &template.qualified_name));
        }
    }
}

fn references_scope(expr: &ResolvedExpr, scope: &str) -> bool {
    match expr {
        ResolvedExpr::SelfRef => true,
        ResolvedExpr::FeaturePath { segments } => segments
            .iter()
            .any(|s| s.feature_id.starts_with(&format!("feature.{scope}."))),
        ResolvedExpr::Tuple { items } => items.iter().any(|e| references_scope(e, scope)),
        ResolvedExpr::Unary { expr, .. } => references_scope(expr, scope),
        ResolvedExpr::Binary { left, right, .. } => {
            references_scope(left, scope) || references_scope(right, scope)
        }
        ResolvedExpr::Call { args, .. } => args.iter().any(|e| references_scope(e, scope)),
        ResolvedExpr::Literal(_) => false,
    }
}

fn substitute(
    expr: &ResolvedExpr,
    bindings: &BTreeMap<String, ResolvedExpr>,
) -> Option<ResolvedExpr> {
    Some(match expr {
        ResolvedExpr::SelfRef => return None,
        ResolvedExpr::FeaturePath { segments } => {
            if let Some(value) = segments.first().and_then(|s| bindings.get(&s.feature_id)) {
                // Navigation through bound object parameters needs instance-aware
                // path composition, so do not emit a misleading scalar result.
                if segments.len() != 1 {
                    return None;
                }
                value.clone()
            } else {
                // Lexical captures need an instance-aware environment.
                return None;
            }
        }
        ResolvedExpr::Tuple { items } => ResolvedExpr::Tuple {
            items: items
                .iter()
                .map(|e| substitute(e, bindings))
                .collect::<Option<_>>()?,
        },
        ResolvedExpr::Unary { op, expr } => ResolvedExpr::Unary {
            op: op.clone(),
            expr: Box::new(substitute(expr, bindings)?),
        },
        ResolvedExpr::Binary { left, op, right } => ResolvedExpr::Binary {
            left: Box::new(substitute(left, bindings)?),
            op: op.clone(),
            right: Box::new(substitute(right, bindings)?),
        },
        ResolvedExpr::Call { function, args } => ResolvedExpr::Call {
            function: function.clone(),
            args: args
                .iter()
                .map(|e| substitute(e, bindings))
                .collect::<Option<_>>()?,
        },
        ResolvedExpr::Literal(_) => expr.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mercurio_foundation::language_contracts::ast::SourceSpan;

    fn definition(name: &str, parents: &[&str]) -> ResolvedDefinition {
        ResolvedDefinition {
            expression: None,
            construct: "ConstraintDefinition".into(),
            qualified_name: name.into(),
            declared_name: name.into(),
            is_abstract: false,
            specializes: parents.iter().map(|p| format!("type.{p}")).collect(),
            members: Vec::new(),
            docs: Vec::new(),
            span: SourceSpan {
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 1,
            },
        }
    }

    #[test]
    fn template_lookup_rejects_cycles_missing_parents_and_multiple_inheritance() {
        let templates = BTreeMap::from([
            ("type.A".into(), definition("A", &["B"])),
            ("type.B".into(), definition("B", &["A"])),
            ("type.Missing".into(), definition("Missing", &["External"])),
            (
                "type.Multiple".into(),
                definition("Multiple", &["Base", "Other"]),
            ),
            ("type.Base".into(), definition("Base", &[])),
            ("type.Other".into(), definition("Other", &[])),
        ]);
        for id in ["type.A", "type.Missing", "type.Multiple"] {
            assert!(effective_template(id, &templates).is_none(), "{id}");
        }
    }
}
