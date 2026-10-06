//! Handwritten execution of written KerML FeatureValue invariants.
//! Ecore supplies ownership, types and Boolean defaults. The computed
//! redefinition and Usage variability algorithms are separate native consumers.
//! An unavailable consumer is reported as unverified, never as a passing check.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FeatureValueIssue {
    pub feature_id: String,
    pub valuation_id: String,
    pub constraint: &'static str,
    pub message: String,
    pub unverified: bool,
}

pub(crate) fn assess(elements: &[KirElement]) -> Result<Vec<FeatureValueIssue>, Diagnostic> {
    if let Some(issue) = ecore_model::validate_publication(elements, ecore_model::ReferenceCompleteness::Closed).first() {
        return Err(error(format!("invalid FeatureValue assessment input: {}.{}: {}", issue.element_id, issue.field, issue.message)));
    }
    let index = elements.iter().map(|e| (e.id.as_str(), e)).collect::<BTreeMap<_, _>>();
    let query = DefinitionNameQuery::new(elements, &index);
    let mut issues = Vec::new();
    for feature in elements.iter().filter(|e| metaclass_conforms(&e.kind, "Feature")) {
        let values = stored_children_indexed(&index, feature, "owned_relationship")?.into_iter()
            .filter(|r| metaclass_conforms(&r.kind, "FeatureValue")).collect::<Vec<_>>();
        if values.is_empty() { continue; }
        let mut issue = |value: &KirElement, constraint, message: String, unverified| issues.push(FeatureValueIssue {
            feature_id: feature.id.clone(), valuation_id: value.id.clone(), constraint, message, unverified,
        });
        // Written FeatureValue description: a Feature has at most one valuation.
        // This is a normative structural obligation, distinct from Pilot's
        // first-valuation overriding predicate and the derived value multiplicity.
        if values.len() > 1 {
            for value in &values { issue(value, "FeatureValue.singleValuation", "Feature has more than one owned FeatureValue".into(), false); }
        }
        for value in &values {
            if scope_container(&index, value)?.map(|owner| owner.id.as_str()) != Some(feature.id.as_str()) {
                return Err(error("FeatureValue requires reciprocal canonical Feature ownership"));
            }
            let expressions = definition_reference_targets(elements, value, "value")?;
            if expressions.len() != 1 {
                issue(value, "FeatureValue.value", "FeatureValue requires one value Expression".into(), false);
            }
            if scope_boolean(value, "is_initial")? {
                let variable = if metaclass_conforms(&feature.kind, "Usage") {
                    let kind = feature.kind.rsplit("::").next().unwrap_or(&feature.kind);
                    if usage_variability_generated::GETTER_BINDINGS.contains(&kind) {
                        definition_may_time_vary(elements, feature)
                    } else { Err(error("unassessed Usage.isVariable binding")) }
                } else { scope_boolean(feature, "is_variable") };
                match variable {
                    Ok(true) => {},
                    Ok(false) => issue(value, "validateFeatureValueIsInitial", "Initialized feature must be variable".into(), false),
                    Err(failure) => issue(value, "validateFeatureValueIsInitial", failure.message, true),
                }
            }
        }
        // The written invariant reads EVERY directly or indirectly redefined
        // Feature's valuations, excluding this Feature itself. Do not substitute
        // raw explicit relationships for the computed native redefinition query.
        match materialized_redefinition_closure_query(&query, feature) {
            Ok(redefined) => {
                for id in redefined.iter().filter(|id| *id != &feature.id) {
                    let target = index.get(id.as_str()).ok_or_else(|| error("missing redefined Feature"))?;
                    for inherited in stored_children_indexed(&index, target, "owned_relationship")?.into_iter()
                        .filter(|r| metaclass_conforms(&r.kind, "FeatureValue")) {
                        if !scope_boolean(inherited, "is_default")? {
                            for value in &values { issue(value, "validateFeatureValueOverriding",
                                format!("Cannot override binding FeatureValue {} on redefined Feature {}", inherited.id, target.id), false); }
                        }
                    }
                }
            },
            Err(failure) => for value in &values { issue(value, "validateFeatureValueOverriding", failure.message.clone(), true); },
        }
    }
    // Also reject detached/non-Feature ownership rather than silently omitting
    // a required featureWithValue dependency from this graph-wide assessment.
    for value in elements.iter().filter(|e| metaclass_conforms(&e.kind, "FeatureValue")) {
        if !scope_container(&index, value)?.is_some_and(|owner| metaclass_conforms(&owner.kind, "Feature")) {
            return Err(error("FeatureValue.featureWithValue requires a canonical Feature owner"));
        }
    }
    issues.sort_by(|left, right| (&left.feature_id, &left.valuation_id, left.constraint, &left.message, left.unverified)
        .cmp(&(&right.feature_id, &right.valuation_id, right.constraint, &right.message, right.unverified)));
    Ok(issues)
}
