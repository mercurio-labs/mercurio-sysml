// SV-0 fixture (tier 1). The expose predicate is the standard way to narrow a
// scope by metadata. Mirrors the pilot's safety / non-safety view pair, which
// is the DISCRIMINATING case: these two views must not resolve to the same
// exposed set. That is SV-2's exit criterion.
package SafetyViews {
    private import Views::*;

    metadata def Safety;

    part def Brake;
    part def Radio;

    part vehicle {
        part brake : Brake { @Safety; }
        part radio : Radio;
    }

    view 'safety features view' {
        expose vehicle::**[@Safety];
        render asTreeDiagram;
    }

    view 'non-safety features view' {
        expose vehicle::**[not (@Safety)];
        render asTreeDiagram;
    }
}
