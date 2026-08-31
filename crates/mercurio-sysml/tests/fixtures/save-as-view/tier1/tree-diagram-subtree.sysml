// SV-0 fixture (tier 1). Scope by recursive expose; notation by a standard
// rendering. The canonical shape from the pilot's own Views Example.
package VehicleViews {
    private import Views::*;

    part def Vehicle;
    part def Engine;
    part def Wheel;

    part vehicle : Vehicle {
        part engine : Engine;
        part wheel : Wheel[4];
    }

    view 'vehicle structure view' {
        doc /* The part breakdown of the vehicle, as a tree. */
        expose vehicle::**;
        render asTreeDiagram;
    }
}
