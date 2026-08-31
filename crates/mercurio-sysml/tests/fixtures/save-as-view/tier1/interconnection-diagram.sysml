// SV-0 fixture (tier 1). asInterconnectionDiagram is the standard rendering for
// internal structure; it maps to the `internal_block` diagram kind.
package InterconnectViews {
    private import Views::*;

    part def Controller;
    part def Sensor;
    part def System {
        part controller : Controller;
        part sensor : Sensor;
    }

    view 'system internals' {
        expose System::**;
        render asInterconnectionDiagram;
    }
}
