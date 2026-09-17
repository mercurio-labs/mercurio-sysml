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
