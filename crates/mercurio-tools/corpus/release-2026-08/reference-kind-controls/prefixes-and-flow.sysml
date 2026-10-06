package ReferencePrefixes {
    attribute def Payload;
    package Targets {
        constraint condition { true }
        requirement need;
        action work {
            action sender { out payload : Payload; }
            action receiver { in payload : Payload; }
            succession flow handoff from sender.payload to receiver.payload;
        }
    }
    part system;
    assert not Targets::condition;
    assert not constraint inline { false }
    not satisfy Targets::need by system;
    assert satisfy Targets::need by system;
    assert not satisfy Targets::need by system;
    perform Targets::work.handoff;
}
