package FlagDefaults {
    allocation def Allocation;
    interface def Interface { end port leftEnd; end port rightEnd; end port thirdEnd; }
}
package AuditFlags {
    part def PayloadPart;
    allocation def A :> FlagDefaults::Allocation;
    allocation def B :> A {
        end leftSide : PayloadPart;
        end rightSide : PayloadPart;
    }
    part p1 : PayloadPart;
    part p2 : PayloadPart;
    allocation linked : B allocate p1 to p2;
    interface def Bus :> FlagDefaults::Interface {
        end port left;
        end port right;
    }
    port def SignalPort {
        out item outbound;
        in item inbound;
    }
    part def Holder {
        port socket : ~SignalPort { attribute sample; }
    }
    action def Behavior {
        merge merged;
        fork forked;
        join joined;
        decide decided;
    }
    variation action def Choice {
        variant action one;
        variant action two;
    }
    variation use case Paths { variant use case path; }
    message envelope;
}
