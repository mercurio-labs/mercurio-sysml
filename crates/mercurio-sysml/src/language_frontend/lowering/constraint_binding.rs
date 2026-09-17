//! Instantiate the bounded, explicitly bound subset of constraint predicates.
//! Identity-based substitution happens before KIR emission, so every host uses
//! the shared evaluator. Incomplete/unsupported applications retain no result.
use std::collections::{BTreeMap, BTreeSet};

use super::ir::{ResolvedDefinition, ResolvedExpr, ResolvedUsage};

pub(super) fn bind_constraint_usages(
    definitions: &mut [ResolvedDefinition],
    usages: &mut [ResolvedUsage],
    context_definitions: &[ResolvedDefinition],
) {
    let templates: BTreeMap<_, _> = context_definitions
        .iter()
        .chain(definitions.iter())
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
        let mut raw = BTreeMap::new();
        let mut aliases = BTreeMap::new();
        let mut consumed = BTreeSet::new();
        let mut supported = true;
        for formal in &template.members {
            let id = format!("feature.{}", formal.qualified_name);
            let actuals: Vec<_> = usage
                .members
                .iter()
                .filter(|m| m.redefined_features.contains(&id))
                .collect();
            if !formal.members.is_empty()
                || !matches!(
                    formal.construct.as_str(),
                    "ReferenceUsage" | "AttributeUsage"
                )
            {
                supported = false;
                break;
            }
            let value = match actuals.as_slice() {
                [] => formal.expression.as_ref(),
                [actual] if actual.redefined_features.len() == 1 && actual.members.is_empty() => {
                    consumed.insert(&actual.qualified_name);
                    aliases.insert(format!("feature.{}", actual.qualified_name), id.clone());
                    if actual.expression.is_some()
                        && formal.expression.is_some()
                        && !formal.modifiers.iter().any(|m| m == "default")
                    {
                        supported = false;
                        break;
                    }
                    actual.expression.as_ref().or(formal.expression.as_ref())
                }
                _ => {
                    supported = false;
                    break;
                }
            };
            let Some(value) = value else {
                supported = false;
                break;
            };
            raw.insert(id, value.clone());
        }
        if !supported || consumed.len() != usage.members.len() {
            continue;
        }
        let mut bindings = BTreeMap::new();
        let mut budget = 10_000;
        for (id, value) in &raw {
            let mut visiting = BTreeSet::from([id.clone()]);
            let Some(value) = expand_binding(
                value,
                &raw,
                &aliases,
                &mut visiting,
                &mut budget,
                &usage.qualified_name,
                &template.qualified_name,
            ) else {
                supported = false;
                break;
            };
            bindings.insert(id.clone(), value);
        }
        if supported && fits_expansion(predicate, &bindings, &mut budget) {
            usage.expression = substitute(predicate, &bindings)
                .filter(|result| !references_scope(result, &template.qualified_name));
        }
    }
}

// Resolve dependencies by feature identity with finite work and cycle checks.
fn expand_binding(
    expr: &ResolvedExpr,
    bindings: &BTreeMap<String, ResolvedExpr>,
    aliases: &BTreeMap<String, String>,
    visiting: &mut BTreeSet<String>,
    budget: &mut usize,
    usage_scope: &str,
    template_scope: &str,
) -> Option<ResolvedExpr> {
    if *budget == 0 || visiting.len() > 128 {
        return None;
    }
    *budget -= 1;
    Some(match expr {
        ResolvedExpr::SelfRef => return None,
        ResolvedExpr::FeaturePath { segments } => {
            let first = segments.first()?;
            let key = aliases.get(&first.feature_id).unwrap_or(&first.feature_id);
            if let Some(value) = bindings.get(key) {
                if segments.len() != 1 || !visiting.insert(key.clone()) {
                    return None;
                }
                let expanded = expand_binding(
                    value,
                    bindings,
                    aliases,
                    visiting,
                    budget,
                    usage_scope,
                    template_scope,
                )?;
                visiting.remove(key);
                expanded
            } else {
                if references_scope(expr, usage_scope) || references_scope(expr, template_scope) {
                    return None;
                }
                expr.clone()
            }
        }
        ResolvedExpr::Literal(_) => expr.clone(),
        ResolvedExpr::Unary { op, expr } => ResolvedExpr::Unary {
            op: op.clone(),
            expr: Box::new(expand_binding(
                expr,
                bindings,
                aliases,
                visiting,
                budget,
                usage_scope,
                template_scope,
            )?),
        },
        ResolvedExpr::Binary { left, op, right } => ResolvedExpr::Binary {
            op: op.clone(),
            left: Box::new(expand_binding(
                left,
                bindings,
                aliases,
                visiting,
                budget,
                usage_scope,
                template_scope,
            )?),
            right: Box::new(expand_binding(
                right,
                bindings,
                aliases,
                visiting,
                budget,
                usage_scope,
                template_scope,
            )?),
        },
        ResolvedExpr::Tuple { items } => ResolvedExpr::Tuple {
            items: items
                .iter()
                .map(|e| {
                    expand_binding(
                        e,
                        bindings,
                        aliases,
                        visiting,
                        budget,
                        usage_scope,
                        template_scope,
                    )
                })
                .collect::<Option<_>>()?,
        },
        ResolvedExpr::Call { function, args } => ResolvedExpr::Call {
            function: function.clone(),
            args: args
                .iter()
                .map(|e| {
                    expand_binding(
                        e,
                        bindings,
                        aliases,
                        visiting,
                        budget,
                        usage_scope,
                        template_scope,
                    )
                })
                .collect::<Option<_>>()?,
        },
    })
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

fn fits_expansion(
    expr: &ResolvedExpr,
    bindings: &BTreeMap<String, ResolvedExpr>,
    budget: &mut usize,
) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    match expr {
        ResolvedExpr::FeaturePath { segments } => segments
            .first()
            .and_then(|s| bindings.get(&s.feature_id))
            .map(|value| fits_expansion(value, &BTreeMap::new(), budget))
            .unwrap_or(true),
        ResolvedExpr::Unary { expr, .. } => fits_expansion(expr, bindings, budget),
        ResolvedExpr::Binary { left, right, .. } => {
            fits_expansion(left, bindings, budget) && fits_expansion(right, bindings, budget)
        }
        ResolvedExpr::Tuple { items } => items.iter().all(|e| fits_expansion(e, bindings, budget)),
        ResolvedExpr::Call { args, .. } => args.iter().all(|e| fits_expansion(e, bindings, budget)),
        _ => true,
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
