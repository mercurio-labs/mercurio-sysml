// SV-0 fixture (tier 2). A Mercurio-only diagram kind, expressed entirely in
// standard constructs: a user rendering def subtyping the standard
// GraphicalRendering, plus one metadata annotation carrying the render params
// that have no standard home (relations, depth, direction, style flags).
//
// Both Mercurio::asStateMachineDiagram and Mercurio::ViewRenderParams are SV-4's
// to define in the Mercurio library package. This fixture pins the SHAPE the map
// depends on; SV-4 pins the names.
package StateViews {
    private import Views::*;
    private import Mercurio::*;

    part def Controller {
        state operatingMode;
    }

    view 'controller modes' {
        @ViewRenderParams {
            relations = ("specializes", "subsets");
            depth = 5;
            layoutDirection = "TB";
            showEdgeLabels = false;
        }
        expose Controller::**;
        render asStateMachineDiagram;
    }
}
