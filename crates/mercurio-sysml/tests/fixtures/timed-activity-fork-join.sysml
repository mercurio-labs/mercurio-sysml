package Mercurio {
    package Missions {
        import ScalarValues::*;
        metadata def Clock {
            attribute maxTime : Real;
            attribute fixedStep : Real;
            attribute sampleInterval : Real;
            attribute maxSteps : Integer;
            attribute changeLoopLimit : Integer;
        }
        metadata def TimedActivity {
            attribute completionFeature : String;
        }
        metadata def Duration {
            attribute seconds : Real;
        }
        metadata def InitialStimulus {
            attribute subject : String;
            attribute trigger : String;
        }
        metadata def Termination {
            attribute onAllSatisfied : Boolean;
            attribute onAnyViolated : Boolean;
            attribute onBlocked : Boolean;
        }
    }
}

package ParallelMeasurement {
    import ScalarValues::*;
    part def Instrument {
        attribute completed : Boolean = false;
        action procedure {
            @Mercurio::Missions::TimedActivity { completionFeature = "completed"; }
            action a { @Mercurio::Missions::Duration { seconds = 2.0; } }
            action b { @Mercurio::Missions::Duration { seconds = 3.0; } }
            action c { @Mercurio::Missions::Duration { seconds = 5.0; } }
            action d { @Mercurio::Missions::Duration { seconds = 1.0; } }
            first a then b;
            first a then c;
            first b then d;
            first c then d;
        }
    }
    analysis def ActivityProfile {
        @Mercurio::Missions::Clock { maxTime = 10.0; fixedStep = 1.0; sampleInterval = 1.0; maxSteps = 100; }
        subject device : Instrument;
        requirement measuredByDeadline {
            attribute deadline : Real = 8.0;
            require constraint { device.completed == true; }
        }
    }
}
