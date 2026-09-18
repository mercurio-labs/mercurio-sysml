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
