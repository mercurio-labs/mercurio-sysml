// Instrument handoff investigation. Temperature numbers represent degrees C; rates degrees C/s.
// Piecewise constant sampled cooling, not a heat-transfer solver or unit-conversion model.
// At t=4 temperature is 40; at deadline 12 it is 32; handoff at 25 occurs at t=19.
// Change finalCoolingRate from -1.0 to -3.0: handoff occurs at t=9.
package Mercurio {
    package Missions {
        import ScalarValues::*;
        metadata def Clock {
            attribute maxTime : Real;
            attribute fixedStep : Real;
            attribute sampleInterval : Real;
            attribute maxSteps : Integer;
        }
        metadata def InitialStimulus {
            attribute subject : String;
            attribute trigger : String;
        }
    }
}

package InstrumentCooldown {
    import ScalarValues::*;
    part def Instrument {
        attribute temperature : Real = 80.0;
        attribute slowThreshold : Real = 40.0;
        attribute handoffTemperature : Real = 25.0;
        attribute fastCoolingRate : Real = -10.0;
        attribute finalCoolingRate : Real = -1.0;
        state lifecycle {
            state Idle;
            state FastCooling {
                do action integrateFastCooling {
                    assert constraint { temperature == temperature + fastCoolingRate * duration; }
                }
            }
            state FinalCooling {
                do action integrateFinalCooling {
                    assert constraint { temperature == temperature + finalCoolingRate * duration; }
                }
            }
            state Ready;
            transition startCooling first Idle accept start then FastCooling;
            transition reduceRate first FastCooling accept when temperature <= slowThreshold then FinalCooling;
            transition finishCooling first FinalCooling accept when temperature <= handoffTemperature then Ready;
        }
    }
    analysis def CooldownProfile :> AnalysisCase {
        @Mercurio::Missions::Clock { maxTime = 25.0; fixedStep = 1.0; sampleInterval = 1.0; maxSteps = 100; }
        @Mercurio::Missions::InitialStimulus { subject = "instrument"; trigger = "start"; }
        subject instrument : Instrument;
        requirement handoffByDeadline {
            attribute deadline : Real = 12.0;
            require constraint { instrument.temperature <= instrument.handoffTemperature; }
        }
    }
}
