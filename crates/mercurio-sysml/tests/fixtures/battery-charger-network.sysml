// Educational lumped model: percentages, seconds, volts, ohms, watts and degrees C
// are scalar conventions here, not a unit-aware electrochemical battery model.
package Mercurio {
    package Missions {
        import ScalarValues::*;
        metadata def Clock { attribute maxTime : Real; attribute fixedStep : Real; attribute sampleInterval : Real; attribute maxSteps : Integer; attribute adaptive : Boolean; attribute absoluteTolerance : Real; attribute relativeTolerance : Real; attribute minimumStep : Real; attribute eventTolerance : Real; attribute maxRefinements : Integer; }
        metadata def InitialStimulus { attribute subject : String; attribute trigger : String; }
        metadata def ConstraintNetwork { attribute unknowns : String; attribute tolerance : Real; }
    }
}
package ChargingSystem {
    import ScalarValues::*;
    part def BatteryCharger {
        @Mercurio::Missions::ConstraintNetwork {
            unknowns = "packVoltage, availableCurrent, batteryCurrent, heatLoss, chargePower, coolingPower";
            tolerance = 1e-9;
        }
        // Battery state and charger settings.
        attribute stateOfCharge : Real = 40.0;
        attribute temperature : Real = 20.0;
        attribute capacityEnergy : Real = 1000.0;
        attribute nominalVoltage : Real = 20.0;
        attribute voltageSlope : Real = 0.03;
        attribute chargerVoltage : Real = 24.0;
        attribute chargerResistance : Real = 1.0;
        attribute lossFraction : Real = 0.25;
        attribute chargingEnabled : Real = 0.0;
        // Thermal controller: hysteresis prevents immediate on/off oscillation.
        attribute ambientTemperature : Real = 20.0;
        attribute thermalCapacity : Real = 10.0;
        attribute coolingConductance : Real = 0.5;
        attribute cutoffTemperature : Real = 25.0;
        attribute restartTemperature : Real = 22.0;
        attribute targetVoltage : Real = 22.1;
        // Algebraic outputs: no independent initial guesses or assignment writers.
        attribute packVoltage : Real;
        attribute availableCurrent : Real;
        attribute batteryCurrent : Real;
        attribute heatLoss : Real;
        attribute chargePower : Real;
        attribute coolingPower : Real;
        constraint batteryVoltage { packVoltage == nominalVoltage + voltageSlope * stateOfCharge }
        constraint chargerLoad { chargerResistance * availableCurrent + packVoltage == chargerVoltage }
        constraint commandedCurrent { batteryCurrent == chargingEnabled * availableCurrent }
        constraint chargerLoss { heatLoss == lossFraction * chargerVoltage * batteryCurrent }
        constraint powerBalance { chargePower + heatLoss == chargerVoltage * batteryCurrent }
        constraint thermalLoss { coolingPower == coolingConductance * (temperature - ambientTemperature) }
        state controller {
            do action evolveBattery {
                assert constraint { stateOfCharge == stateOfCharge + (100.0 * chargePower / capacityEnergy) * duration; }
                assert constraint { temperature == temperature + ((heatLoss - coolingPower) / thermalCapacity) * duration; }
            }
            state Idle;
            state Charging;
            state Cooling;
            state Ready;
            transition start first Idle accept start do assign chargingEnabled := 1.0 then Charging;
            transition thermalCutoff first Charging accept when temperature >= cutoffTemperature do assign chargingEnabled := 0.0 then Cooling;
            transition restart first Cooling accept when temperature <= restartTemperature do assign chargingEnabled := 1.0 then Charging;
            transition charged first Charging accept when packVoltage >= targetVoltage do assign chargingEnabled := 0.0 then Ready;
        }
    }
    analysis def ChargeAndCool :> AnalysisCase {
        @Mercurio::Missions::Clock { maxTime = 30.0; fixedStep = 2.0; sampleInterval = 0.25; maxSteps = 500; adaptive = true; absoluteTolerance = 1e-9; relativeTolerance = 1e-7; minimumStep = 1e-8; eventTolerance = 1e-7; maxRefinements = 32; }
        @Mercurio::Missions::InitialStimulus { subject = "system"; trigger = "start"; }
        subject system : BatteryCharger;
        requirement chargedByHandoff {
            attribute deadline : Real = 20.0;
            require constraint { system.stateOfCharge >= 70.0; }
        }
        requirement cooledByEnd {
            attribute deadline : Real = 30.0;
            require constraint { system.temperature <= system.cutoffTemperature; }
        }
    }
}
