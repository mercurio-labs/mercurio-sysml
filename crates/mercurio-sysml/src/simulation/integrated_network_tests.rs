use super::session::{
    AnalysisExperimentRequest, AnalysisSimulationSession, SimulationExperimentAxis,
    run_analysis_experiment,
};
use super::*;
use crate::{compile_sysml_text, load_sysml_baseline};
use mercurio_foundation::simulation_core::SimulationSessionLifecycle;
const SOURCE: &str = include_str!("../../tests/fixtures/battery-charger-network.sysml");
fn runtime(source: &str) -> Runtime {
    Runtime::from_document(
        compile_sysml_text(source, "charger.sysml", &load_sysml_baseline().unwrap()).unwrap(),
    )
    .unwrap()
}
fn value(frame: &Value, feature: &str) -> f64 {
    frame["values"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["feature"] == feature)
        .unwrap()["value"]
        .as_f64()
        .unwrap()
}
#[test]
fn integrated_charger_executes_coupled_networks_and_thermal_hysteresis() {
    let runtime = runtime(SOURCE);
    let report = run_analysis_case(&runtime, "ChargeAndCool", "test").unwrap();
    let batch = report
        .artifacts
        .iter()
        .find(|a| a.kind == "simulation_trace")
        .unwrap()
        .payload
        .clone();
    let mut session = AnalysisSimulationSession::start(&runtime, "ChargeAndCool", &[]).unwrap();
    assert!(
        !session
            .snapshot()
            .unwrap()
            .inputs
            .iter()
            .any(|input| input.feature == "chargePower")
    );
    while session.snapshot().unwrap().lifecycle == SimulationSessionLifecycle::Paused {
        session.advance(7).unwrap();
    }
    let done = session.snapshot().unwrap();
    assert_eq!(
        done.lifecycle,
        SimulationSessionLifecycle::Completed,
        "{:?}",
        done.error
    );
    assert_eq!(done.trace["timeline"], batch["timeline"]);
    let frames = done.trace["timeline"].as_array().unwrap();
    for frame in frames {
        let evaluations = frame["network_evaluations"].as_array().unwrap();
        assert_eq!(evaluations.len(), 1);
        assert_eq!(evaluations[0]["result"]["status"], "solved");
        assert!(
            evaluations[0]["result"]["residuals"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["withinTolerance"] == true)
        );
        assert!(
            (value(frame, "chargePower") + value(frame, "heatLoss")
                - value(frame, "chargerVoltage") * value(frame, "batteryCurrent"))
            .abs()
                < 1e-8
        );
    }
    assert!(
        frames
            .iter()
            .any(|f| f["states"].to_string().contains("Cooling"))
    );
    assert!(
        frames
            .iter()
            .filter(|f| f["states"].to_string().contains("Cooling"))
            .all(|f| value(f, "batteryCurrent").abs() < 1e-9)
    );
    assert_eq!(
        done.trace["requirement_outcomes"][0]["status"], "violated",
        "{}",
        done.trace["requirement_outcomes"]
    );
    let experiment = run_analysis_experiment(
        &runtime,
        "ChargeAndCool",
        AnalysisExperimentRequest {
            axes: vec![SimulationExperimentAxis {
                subject_id: "subject.ChargingSystem.ChargeAndCool.system".into(),
                feature: "coolingConductance".into(),
                values: vec![serde_json::json!(0.5), serde_json::json!(1.5)],
            }],
        },
    )
    .unwrap();
    let statuses = experiment
        .variants
        .iter()
        .map(|v| v.snapshot.as_ref().unwrap().trace["requirement_outcomes"][0]["status"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        statuses,
        vec![
            serde_json::json!("violated"),
            serde_json::json!("satisfied")
        ]
    );
}
#[test]
fn authored_dynamic_network_rejects_bad_bindings_and_nonunique_initial_solutions() {
    for (source, diagnostic) in [
        (
            SOURCE.replace("unknowns = \"packVoltage", "unknowns = \"missing"),
            "missing",
        ),
        (
            SOURCE.replace(
                "chargerResistance : Real = 1.0",
                "chargerResistance : Real = 0.0",
            ),
            "Inconsistent",
        ),
        (
            SOURCE.replace(
                "packVoltage == nominalVoltage + voltageSlope * stateOfCharge",
                "packVoltage * packVoltage == nominalVoltage + voltageSlope * stateOfCharge",
            ),
            "Unsupported",
        ),
        (
            SOURCE.replace(
                "do assign chargingEnabled := 1.0",
                "do assign chargePower := 1.0",
            ),
            "solved outputs",
        ),
    ] {
        let runtime = runtime(&source);
        let error = AnalysisSimulationSession::start(&runtime, "ChargeAndCool", &[])
            .err()
            .expect("invalid network must be rejected");
        assert!(format!("{error:?}").contains(diagnostic), "{error:?}");
    }
}

#[test]
fn failed_controller_effect_restores_last_committed_sample() {
    let runtime = runtime(&SOURCE.replace(
        "cutoffTemperature do assign chargingEnabled := 0.0",
        "cutoffTemperature do assign chargerResistance := 0.0",
    ));
    let mut session = AnalysisSimulationSession::start(&runtime, "ChargeAndCool", &[]).unwrap();
    loop {
        let before = session.snapshot().unwrap();
        assert_eq!(before.lifecycle, SimulationSessionLifecycle::Paused);
        let after = session.advance(1).unwrap();
        if after.lifecycle == SimulationSessionLifecycle::Failed {
            assert_eq!(after.logical_time_s, before.logical_time_s);
            assert_eq!(after.advance_cycles, before.advance_cycles);
            assert_eq!(after.trace["timeline"], before.trace["timeline"]);
            assert!(after.error.unwrap().contains("Inconsistent"));
            break;
        }
    }
}

#[test]
fn adaptive_charger_converges_across_coarse_and_fine_maximum_steps() {
    fn run(source: &str) -> Value {
        let runtime = runtime(source);
        let mut session = AnalysisSimulationSession::start(&runtime, "ChargeAndCool", &[]).unwrap();
        while session.snapshot().unwrap().lifecycle == SimulationSessionLifecycle::Paused {
            session.advance(17).unwrap();
        }
        let done = session.snapshot().unwrap();
        assert_eq!(
            done.lifecycle,
            SimulationSessionLifecycle::Completed,
            "{:?}",
            done.error
        );
        assert_eq!(done.logical_time_s, 30.0);
        done.trace
    }
    let coarse = run(&SOURCE.replace("sampleInterval = 0.25", "sampleInterval = 5.0"));
    let fine = run(&SOURCE
        .replace("fixedStep = 2.0", "fixedStep = 0.0625")
        .replace("maxSteps = 500", "maxSteps = 2000")
        .replace("sampleInterval = 0.25", "sampleInterval = 5.0"));
    let cutoffs = |trace: &Value| {
        trace["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|frame| frame["events"].to_string().contains("thermalCutoff"))
            .map(|frame| frame["t"].as_f64().unwrap())
            .collect::<Vec<_>>()
    };
    let (coarse_times, fine_times) = (cutoffs(&coarse), cutoffs(&fine));
    assert!(!coarse_times.is_empty());
    assert_eq!(coarse_times.len(), fine_times.len());
    for (a, b) in coarse_times.iter().zip(&fine_times) {
        assert!((a - b).abs() < 1e-4, "coarse={a}, fine={b}");
    }
    for feature in ["temperature", "stateOfCharge"] {
        let a = value(
            coarse["timeline"].as_array().unwrap().last().unwrap(),
            feature,
        );
        let b = value(
            fine["timeline"].as_array().unwrap().last().unwrap(),
            feature,
        );
        assert!((a - b).abs() < 1e-4, "{feature}: {a} vs {b}");
    }
    for frame in coarse["timeline"].as_array().unwrap() {
        if let Some(integration) = frame.get("integration") {
            assert!(integration["error_ratio"].as_f64().unwrap() <= 1.0);
            if let Some(width) = integration["event_bracket_s"].as_f64() {
                assert!(width <= 1e-7);
            }
        }
    }
    assert!(
        coarse["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .any(|frame| { frame["integration"]["event_bracket_s"].as_f64().is_some() })
    );
}

#[test]
fn authored_adaptive_settings_reject_invalid_or_disabled_tolerances() {
    for source in [
        SOURCE.replace("minimumStep = 1e-8", "minimumStep = 3.0"),
        SOURCE.replace("relativeTolerance = 1e-7", "relativeTolerance = 0.0"),
        SOURCE.replace("adaptive = true", "adaptive = false"),
    ] {
        let error = AnalysisSimulationSession::start(&runtime(&source), "ChargeAndCool", &[])
            .err()
            .expect("invalid numerical settings");
        let diagnostic = format!("{error:?}");
        assert!(
            diagnostic.contains("numerics") || diagnostic.contains("adaptive tolerances"),
            "{diagnostic}"
        );
    }
}
