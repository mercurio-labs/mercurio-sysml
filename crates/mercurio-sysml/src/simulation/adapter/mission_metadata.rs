use super::*;
use serde_json::Map;

fn invalid(message: impl Into<String>) -> SysmlSimulationAdapterError {
    SysmlSimulationAdapterError::InvalidAnalysisCase(format!(
        "mission.metadata.invalid: {}",
        message.into()
    ))
}

fn properties<'a>(
    runtime: &Runtime,
    case: &'a Element,
    name: &str,
    allowed: &[&str],
) -> Result<Option<&'a Map<String, Value>>, SysmlSimulationAdapterError> {
    let key = format!("Mercurio::Missions::{name}");
    let Some(annotation) = case.properties.get("metadata").and_then(|m| m.get(&key)) else {
        return Ok(None);
    };
    let definition = format!("type.Mercurio.Missions.{name}");
    if !runtime
        .graph()
        .element_by_element_id(&definition)
        .is_some_and(|e| e.kind.ends_with("MetadataDefinition"))
    {
        return Err(invalid(format!(
            "{key} requires its authored metadata definition"
        )));
    }
    let properties = annotation
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid(format!("{key} properties must be an object")))?;
    if let Some(unknown) = properties
        .keys()
        .find(|key| !allowed.contains(&key.as_str()))
    {
        return Err(invalid(format!("unsupported {name} field {unknown}")));
    }
    Ok(Some(properties))
}

fn number(properties: &Map<String, Value>, key: &str) -> Result<f64, SysmlSimulationAdapterError> {
    let value = properties
        .get(key)
        .ok_or_else(|| invalid(format!("missing {key}")))?;
    let number = value
        .as_f64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<f64>().ok()))
        .ok_or_else(|| invalid(format!("{key} must be numeric")))?;
    if !number.is_finite() {
        return Err(invalid(format!("{key} must be finite")));
    }
    Ok(number)
}
fn positive_integer(
    properties: &Map<String, Value>,
    key: &str,
    fallback: usize,
) -> Result<usize, SysmlSimulationAdapterError> {
    if !properties.contains_key(key) {
        return Ok(fallback);
    }
    let number = number(properties, key)?;
    if number < 1.0 || number.fract() != 0.0 || number >= usize::MAX as f64 {
        return Err(invalid(format!(
            "{key} must be a positive representable integer"
        )));
    }
    Ok(number as usize)
}
fn boolean(
    properties: &Map<String, Value>,
    key: &str,
) -> Result<bool, SysmlSimulationAdapterError> {
    match properties.get(key) {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(Value::String(s)) if s == "true" => Ok(true),
        Some(Value::String(s)) if s == "false" => Ok(false),
        _ => Err(invalid(format!("{key} must be Boolean"))),
    }
}

pub(super) fn apply(
    runtime: &Runtime,
    case: &Element,
    scenario: &mut ConcurrentSimulationScenario,
) -> Result<(), SysmlSimulationAdapterError> {
    if let Some(p) = properties(
        runtime,
        case,
        "Clock",
        &[
            "maxTime",
            "fixedStep",
            "sampleInterval",
            "maxSteps",
            "changeLoopLimit",
        ],
    )? {
        let max_time_s = number(p, "maxTime")?;
        let fixed_step_s = number(p, "fixedStep")?;
        let sample_interval_s = number(p, "sampleInterval")?;
        if max_time_s < 0.0 || fixed_step_s <= 0.0 || sample_interval_s <= 0.0 {
            return Err(invalid(
                "maxTime must be nonnegative; fixedStep and sampleInterval must be positive seconds",
            ));
        }
        let change_loop_limit = positive_integer(p, "changeLoopLimit", 20)?;
        scenario.max_steps = positive_integer(p, "maxSteps", scenario.max_steps)?;
        scenario.step_duration_s = fixed_step_s;
        scenario.clock_config = Some(SimulationClockConfig {
            max_time_s,
            fixed_step_s,
            sample_interval_s,
            change_loop_limit,
        });
    }
    if let Some(p) = properties(
        runtime,
        case,
        "Termination",
        &["onAllSatisfied", "onAnyViolated", "onBlocked"],
    )? {
        scenario.termination_policy =
            mercurio_foundation::simulation_core::SimulationTerminationPolicy {
                on_all_satisfied: boolean(p, "onAllSatisfied")?,
                on_any_violated: boolean(p, "onAnyViolated")?,
                on_blocked: boolean(p, "onBlocked")?,
            };
    }
    if let Some(p) = properties(runtime, case, "InitialStimulus", &["subject", "trigger"])? {
        let subject_name = p
            .get("subject")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| invalid("InitialStimulus requires subject"))?;
        let trigger = p
            .get("trigger")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| invalid("InitialStimulus requires trigger"))?;
        let matches = native_analysis_subject_elements(runtime, case)
            .into_iter()
            .filter(|s| s.element_id == subject_name || element_label_element(s) == subject_name)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(invalid("InitialStimulus subject must resolve uniquely"));
        }
        let subject_id = &matches[0].element_id;
        for subject in &mut scenario.subjects {
            subject.events.clear();
            if subject.subject_id == *subject_id {
                subject.events.push(SimulationEvent {
                    id: format!("{}.mission.initial", case.element_id),
                    trigger: trigger.into(),
                });
            }
        }
    }
    Ok(())
}
