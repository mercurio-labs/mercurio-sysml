package ReleaseOccurrenceDefaults {
    part def Vehicle { attribute mass: ScalarValues::Real; }
    individual part def Car :> Vehicle {
        snapshot part atStart;
        timeslice part inUse;
    }
    variation part def Engine :> Vehicle { variant part four; variant part six; }
    use case travel;
    action sequence { action transmit send { } }
}
