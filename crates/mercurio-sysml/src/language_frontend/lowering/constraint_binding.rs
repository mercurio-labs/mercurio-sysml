//! Lower bounded calculation and constraint applications to shared runtime frames.
//! Bindings retain feature identity and execute once; no predicate substitution.
use std::collections::{BTreeMap, BTreeSet};

use super::expression_contract::usage_expression_contract;
use super::ir::{ResolvedBinding, ResolvedDefinition, ResolvedExpr, ResolvedUsage};
use mercurio_foundation::kir::{ExpressionContract, ExpressionMultiplicity, ExpressionValueType};
use mercurio_foundation::language_contracts::diagnostics::Diagnostic;

type Templates = BTreeMap<String, ResolvedDefinition>;
const MAX_CALL_DEPTH: usize = 64;
const MAX_NODES: usize = 10_000;

pub(super) fn bind_constraint_usages(
    definitions: &mut [ResolvedDefinition],
    usages: &mut [ResolvedUsage],
    context_definitions: &[ResolvedDefinition],
) -> Result<(), Diagnostic> {
    let templates: Templates = context_definitions
        .iter()
        .chain(definitions.iter())
        .filter(|d| {
            matches!(
                d.construct.as_str(),
                "ConstraintDefinition" | "CalculationDefinition"
            )
        })
        .map(|d| (format!("type.{}", d.qualified_name), d.clone()))
        .collect();
    for definition in definitions {
        if let Some(expression) = &definition.expression {
            definition.expression = Some(
                lower_calls(
                    expression,
                    &templates,
                    &mut vec![format!("type.{}", definition.qualified_name)],
                    &mut MAX_NODES.clone(),
                )
                .map_err(|message| Diagnostic::new(message, Some(definition.span.clone())))?,
            );
        }
        visit(&mut definition.members, &templates)?;
    }
    visit(usages, &templates)
}

// Only transparent single-parent inheritance has unambiguous composition here.
fn effective_template<'a>(id: &str, templates: &'a Templates) -> Option<&'a ResolvedDefinition> {
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

fn visit(usages: &mut [ResolvedUsage], templates: &Templates) -> Result<(), Diagnostic> {
    for usage in usages {
        visit(&mut usage.members, templates)?;
        if let Some((target, expression)) = &usage.assignment {
            usage.assignment = Some((
                target.clone(),
                lower_calls(
                    expression,
                    templates,
                    &mut Vec::new(),
                    &mut MAX_NODES.clone(),
                )
                .map_err(|message| Diagnostic::new(message, Some(usage.span.clone())))?,
            ));
        }
        if let Some(expression) = &usage.expression {
            usage.expression = Some(
                lower_calls(
                    expression,
                    templates,
                    &mut Vec::new(),
                    &mut MAX_NODES.clone(),
                )
                .map_err(|message| Diagnostic::new(message, Some(usage.span.clone())))?,
            );
            continue;
        }
        if !(usage.construct == "ConstraintUsage"
            || usage.construct == "CalculationUsage"
            || (usage.construct == "RequireUsage"
                && usage.modifiers.iter().any(|m| m == "constraint")))
            || usage.multiplicity.is_some()
            || !usage.additional_type_refs.is_empty()
            || !usage.specializes.is_empty()
            || !usage.redefined_features.is_empty()
            || !usage.subsetted_features.is_empty()
        {
            continue;
        }
        let Some(id) = usage.type_ref.as_deref() else {
            continue;
        };
        let Some(template) = effective_template(id, templates) else {
            continue;
        };
        let Some(body) = result_expression(template) else {
            continue;
        };
        let mut raw = BTreeMap::new();
        let mut aliases = BTreeMap::new();
        let mut consumed = BTreeSet::new();
        let mut supported = true;
        for formal in template.members.iter().filter(|m| !is_return(m)) {
            let feature = feature_id(formal);
            let actuals: Vec<_> = usage
                .members
                .iter()
                .filter(|m| m.redefined_features.contains(&feature))
                .collect();
            if !simple_member(formal) {
                supported = false;
                break;
            }
            let value = match actuals.as_slice() {
                [] => formal.expression.clone().map(|value| (value, true)),
                [actual] if actual.redefined_features.len() == 1 && actual.members.is_empty() => {
                    consumed.insert(&actual.qualified_name);
                    aliases.insert(feature_id(actual), feature.clone());
                    if actual.expression.is_some()
                        && formal.expression.is_some()
                        && !formal.modifiers.iter().any(|m| m == "default")
                    {
                        supported = false;
                        break;
                    }
                    actual.expression.clone().or(formal.expression.clone())
                        .map(|value| checked(value, actual).map(|value| (value, actual.expression.is_none()))).transpose()?
                }
                _ => {
                    supported = false;
                    break;
                }
            };
            let Some((value, lexical)) = value else {
                supported = false;
                break;
            };
            raw.insert(feature, (checked(value, formal)?, lexical));
        }
        if !supported || consumed.len() != usage.members.len() {
            continue;
        }
        for (expression, _) in raw.values_mut() {
            rewrite_aliases(expression, &aliases);
        }
        // Incomplete structural applications retain the existing unevaluated
        // representation. Calls with arguments instead report compile diagnostics.
        let body = checked_result(body, template)?;
        usage.expression = lower_prepared_frame(id, raw, body, templates, &mut Vec::new(), &mut MAX_NODES.clone()).ok();
    }
    Ok(())
}

fn is_return(member: &ResolvedUsage) -> bool {
    member.modifiers.iter().any(|m| m == "return")
}
fn simple_member(member: &ResolvedUsage) -> bool {
    member.members.is_empty()
        && matches!(
            member.construct.as_str(),
            "ReferenceUsage" | "AttributeUsage" | "Feature"
        )
        && !member.modifiers.iter().any(|m| m == "out" || m == "inout")
}
fn feature_id(member: &ResolvedUsage) -> String {
    format!("feature.{}", member.qualified_name)
}
fn result_expression(template: &ResolvedDefinition) -> Option<ResolvedExpr> {
    let returns: Vec<_> = template.members.iter().filter(|m| is_return(m)).collect();
    match (template.expression.as_ref(), returns.as_slice()) {
        (Some(body), [] | [_]) if returns.iter().all(|m| m.expression.is_none()) => {
            Some(body.clone())
        }
        (None, [result]) => result.expression.clone(),
        _ => None,
    }
}
fn checked(expression: ResolvedExpr, usage: &ResolvedUsage) -> Result<ResolvedExpr, Diagnostic> {
    Ok(match usage_expression_contract(usage)? {
        Some(contract) => ResolvedExpr::Checked {
            expression: Box::new(expression),
            contract,
        },
        None => expression,
    })
}
fn checked_result(body: ResolvedExpr, template: &ResolvedDefinition) -> Result<ResolvedExpr, Diagnostic> {
    let body = match template.members.iter().find(|m| is_return(m)) {
        Some(result) => checked(body, result)?,
        None => body,
    };
    if template.construct == "ConstraintDefinition" {
        return Ok(ResolvedExpr::Checked {
            expression: Box::new(body),
            contract: ExpressionContract {
                value_type: ExpressionValueType::Boolean,
                multiplicity: Some(ExpressionMultiplicity { lower: 1, upper: Some(1) }),
            },
        });
    }
    Ok(body)
}

fn lower_calls(
    expr: &ResolvedExpr,
    templates: &Templates,
    stack: &mut Vec<String>,
    budget: &mut usize,
) -> Result<ResolvedExpr, String> {
    if *budget == 0 {
        return Err("expression invocation exceeds the 10000-node compilation budget".into());
    }
    *budget -= 1;
    Ok(match expr {
        ResolvedExpr::Call { function, args } if templates.contains_key(function) => {
            if stack.len() >= MAX_CALL_DEPTH {
                return Err(
                    "expression invocation exceeds the 64-call compilation depth limit".into(),
                );
            }
            if stack.contains(function) {
                return Err(format!(
                    "recursive expression invocation is not supported: {function}"
                ));
            }
            let template = effective_template(function, templates)
                .ok_or_else(|| format!("unsupported callable inheritance: {function}"))?;
            let body = result_expression(template).ok_or_else(|| {
                format!("callable has no supported result expression: {function}")
            })?;
            let parameters: Vec<_> = template
                .members
                .iter()
                .filter(|m| {
                    !is_return(m) && m.modifiers.iter().any(|v| v == "in")
                })
                .collect();
            if args.len() > parameters.len() {
                return Err(format!(
                    "too many arguments for {function}: expected at most {}, got {}",
                    parameters.len(),
                    args.len()
                ));
            }
            let mut raw = BTreeMap::new();
            for (index, parameter) in parameters.iter().enumerate() {
                if !simple_member(parameter) {
                    return Err(format!("unsupported parameter in {function}"));
                }
                let expression = match args.get(index) {
                    Some(_)
                        if parameter.expression.is_some()
                            && !parameter.modifiers.iter().any(|m| m == "default") =>
                    {
                        return Err(format!(
                            "cannot override fixed parameter {} in {function}",
                            parameter.declared_name
                        ));
                    }
                    Some(value) => value,
                    None => parameter.expression.as_ref().ok_or_else(|| {
                        format!("missing argument {} in {function}", parameter.declared_name)
                    })?,
                };
                // Caller arguments are lowered before pushing the callee: nested
                // f(f(x)) is finite syntax, not recursion in the function body.
                let expression = if index < args.len() {
                    lower_calls(expression, templates, stack, budget)?
                } else {
                    expression.clone()
                };
                raw.insert(
                    feature_id(parameter),
                    (checked(expression, parameter).map_err(|e| e.to_string())?, index >= args.len()),
                );
            }
            for local in template.members.iter().filter(|m| {
                !is_return(m)
                    && !parameters
                        .iter()
                        .any(|p| p.qualified_name == m.qualified_name)
            }) {
                if !simple_member(local) {
                    return Err(format!("unsupported local feature in {function}"));
                }
                let value = local.expression.clone().ok_or_else(|| {
                    format!(
                        "unbound local feature {} in {function}",
                        local.declared_name
                    )
                })?;
                raw.insert(
                    feature_id(local),
                    (checked(value, local).map_err(|e| e.to_string())?, true),
                );
            }
            let body = checked_result(body, template).map_err(|e| e.to_string())?;
            lower_prepared_frame(function, raw, body, templates, stack, budget)?
        }
        ResolvedExpr::Call { function, args } => ResolvedExpr::Call {
            function: function.clone(),
            args: args
                .iter()
                .map(|a| lower_calls(a, templates, stack, budget))
                .collect::<Result<_, _>>()?,
        },
        ResolvedExpr::Invoke {
            function,
            bindings,
            body,
        } => lower_frame(function, bindings, body, templates, stack, budget)?,
        ResolvedExpr::Checked {
            expression,
            contract,
        } => ResolvedExpr::Checked {
            expression: Box::new(lower_calls(expression, templates, stack, budget)?),
            contract: contract.clone(),
        },
        ResolvedExpr::Tuple { items } => ResolvedExpr::Tuple {
            items: items
                .iter()
                .map(|a| lower_calls(a, templates, stack, budget))
                .collect::<Result<_, _>>()?,
        },
        ResolvedExpr::Unary { op, expr } => ResolvedExpr::Unary {
            op: op.clone(),
            expr: Box::new(lower_calls(expr, templates, stack, budget)?),
        },
        ResolvedExpr::Binary { left, op, right } => ResolvedExpr::Binary {
            left: Box::new(lower_calls(left, templates, stack, budget)?),
            op: op.clone(),
            right: Box::new(lower_calls(right, templates, stack, budget)?),
        },
        ResolvedExpr::SelfRef if !stack.is_empty() => {
            return Err("self-dependent invocation bodies are not supported".into());
        }
        _ => expr.clone(),
    })
}
// Expand declaration-owned calls before ordering bindings: a nested callable
// can capture another formal without that dependency appearing at the call site.
fn lower_prepared_frame(
    function: &str,
    mut raw: BTreeMap<String, (ResolvedExpr, bool)>,
    body: ResolvedExpr,
    templates: &Templates,
    stack: &mut Vec<String>,
    budget: &mut usize,
) -> Result<ResolvedExpr, String> {
    stack.push(function.to_string());
    let result = (|| {
        for (expression, lexical) in raw.values_mut() {
            if *lexical {
                *expression = lower_calls(expression, templates, stack, budget)?;
            }
        }
        let bindings = order_bindings(raw)?;
        let body = lower_calls(&body, templates, stack, budget)?;
        Ok(ResolvedExpr::Invoke { function: function.to_string(), bindings, body: Box::new(body) })
    })();
    stack.pop();
    result
}

fn lower_frame(
    function: &str,
    bindings: &[ResolvedBinding],
    body: &ResolvedExpr,
    templates: &Templates,
    stack: &mut Vec<String>,
    budget: &mut usize,
) -> Result<ResolvedExpr, String> {
    Ok(ResolvedExpr::Invoke {
        function: function.to_string(),
        bindings: bindings
            .iter()
            .map(|b| {
                Ok(ResolvedBinding {
                    feature: b.feature.clone(),
                    lexical: b.lexical,
                    expression: lower_calls(&b.expression, templates, stack, budget)?,
                })
            })
            .collect::<Result<_, String>>()?,
        body: Box::new(lower_calls(body, templates, stack, budget)?),
    })
}

fn dependencies(expr: &ResolvedExpr, output: &mut BTreeSet<String>) {
    match expr {
        ResolvedExpr::FeaturePath { segments } => {
            if let Some(first) = segments.first() {
                output.insert(first.feature_id.clone());
            }
        }
        ResolvedExpr::Checked { expression, .. }
        | ResolvedExpr::Unary {
            expr: expression, ..
        } => dependencies(expression, output),
        ResolvedExpr::Binary { left, right, .. } => {
            dependencies(left, output);
            dependencies(right, output);
        }
        ResolvedExpr::Call { args, .. } | ResolvedExpr::Tuple { items: args } => {
            for expression in args {
                dependencies(expression, output);
            }
        }
        ResolvedExpr::Invoke { bindings, body, .. } => {
            let mut nested = BTreeSet::new();
            for binding in bindings {
                dependencies(&binding.expression, &mut nested);
            }
            dependencies(body, &mut nested);
            for binding in bindings {
                nested.remove(&binding.feature);
            }
            output.extend(nested);
        }
        _ => {}
    }
}
fn order_bindings(raw: BTreeMap<String, (ResolvedExpr, bool)>) -> Result<Vec<ResolvedBinding>, String> {
    if raw.len() > 128 {
        return Err("invocation exceeds the 128-binding limit".into());
    }
    let keys: BTreeSet<_> = raw.keys().cloned().collect();
    let mut pending: BTreeMap<_, _> = raw
        .into_iter()
        .map(|(id, (expression, lexical))| {
            let mut deps = BTreeSet::new();
            dependencies(&expression, &mut deps);
            deps.retain(|id| keys.contains(id));
            (id, (expression, lexical, deps))
        })
        .collect();
    let mut complete = BTreeSet::new();
    let mut bindings = Vec::new();
    while !pending.is_empty() {
        let key = pending
            .iter()
            .find(|(_, (_, _, deps))| deps.is_subset(&complete))
            .map(|(id, _)| id.clone())
            .ok_or_else(|| "cyclic expression parameter/default bindings".to_string())?;
        let Some((expression, lexical, _)) = pending.remove(&key) else {
            return Err("invalid binding order".into());
        };
        complete.insert(key.clone());
        bindings.push(ResolvedBinding {
            feature: key,
            lexical,
            expression,
        });
    }
    Ok(bindings)
}
fn rewrite_aliases(expr: &mut ResolvedExpr, aliases: &BTreeMap<String, String>) {
    match expr {
        ResolvedExpr::FeaturePath { segments } => {
            for segment in segments {
                if let Some(id) = aliases.get(&segment.feature_id) {
                    segment.feature_id = id.clone();
                }
            }
        }
        ResolvedExpr::Checked { expression, .. }
        | ResolvedExpr::Unary {
            expr: expression, ..
        } => rewrite_aliases(expression, aliases),
        ResolvedExpr::Binary { left, right, .. } => {
            rewrite_aliases(left, aliases);
            rewrite_aliases(right, aliases);
        }
        ResolvedExpr::Call { args, .. } | ResolvedExpr::Tuple { items: args } => {
            for expression in args {
                rewrite_aliases(expression, aliases);
            }
        }
        ResolvedExpr::Invoke { bindings, body, .. } => {
            for binding in bindings {
                rewrite_aliases(&mut binding.expression, aliases);
            }
            rewrite_aliases(body, aliases);
        }
        _ => {}
    }
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
