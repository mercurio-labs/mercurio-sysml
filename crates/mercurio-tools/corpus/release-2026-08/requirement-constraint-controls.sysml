package RequirementConstraintControls {
    constraint def Condition;
    constraint available : Condition;
    requirement def Need {
        private assume constraint assumed : Condition;
        require constraint required : Condition;
        assume available;
        require available;
    }
}
