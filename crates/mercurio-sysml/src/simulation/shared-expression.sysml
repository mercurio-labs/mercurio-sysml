package SimulationConstraintsChannels {
    import ScalarValues::*;

    part def ThermalChamber {
        attribute temperature : Real = 20.0;
        attribute targetTemperature : Real = 80.0;
        attribute heatRate : Real = 10.0;

        state lifecycle {
            state Cold;
            state Heating {
                do action integrate {
                    assert constraint {
                        temperature == temperature + (if 1 + 1 == 2 ? heatRate - 0.0 else 1 / 0) * duration;
                    }
                }
            }
            state Ready;

            transition cold_heating first Cold accept start do assign temperature := temperature - ((4503599627370496 * 2 + 1 - 4503599627370496 * 2) - 1) then Heating;
            transition heating_ready first Heating accept when (false implies (1 / 0 > 0)) and temperature - 20.0 >= targetTemperature - 20.0 then Ready;
        }
    }

    analysis def HeatProfile :> AnalysisCase {
        subject chamber : ThermalChamber;
        assume constraint = chamber.temperature == 20.0;

        requirement targetByDeadline {
            attribute deadline : Real = 5.0;
            require constraint {
                1 + 1 == 2 and
                SequenceFunctions::size((1, null, (2, 3))) == 3 and
                (1..3)#(2) == 2 and
                (null ?? 1e0) == 1 and
                (true or (1 / 0 > 0)) and
                chamber.temperature - 20.0 >= chamber.targetTemperature - 20.0;
            }
        }

        objective chamberTemperature {
            subject = chamber.temperature;
        }
    }
}
