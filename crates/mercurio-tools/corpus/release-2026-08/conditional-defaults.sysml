package NextDefaults {
    occurrence untyped { occurrence child; }
    part firstPart;
    part secondPart;
    part thirdPart;
    connection binary connect firstPart to secondPart;
    connection nary connect (firstPart, secondPart, thirdPart);
    action def Workflow {
        action sourceStep;
        action targetStep;
        succession move first sourceStep if true then targetStep;
    }
    state def Lifecycle {
        state sourceStep;
        state targetStep;
        transition move first sourceStep then targetStep;
    }
}
