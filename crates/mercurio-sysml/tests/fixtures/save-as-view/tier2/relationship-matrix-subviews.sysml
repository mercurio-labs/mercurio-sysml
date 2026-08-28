// SV-0 fixture (tier 2). expose scopes ONE namespace set, but a matrix has two
// axes -- so the standard's own composition mechanism carries it: a matrix view
// with two nested view usages (subviews, Views.sysml:16), one per axis. The
// relation set that fills the cells rides the metadata annotation.
//
// matrix_preset is PINNED, never expanded: preset axis filters are multi-kind
// predicates in code (is_matrix_part_element matches PartUsage|PartDefinition|
// ItemUsage|ItemDefinition|BlockDefinition|...) while TableRowTypeDto holds a
// single type name, so expanding a preset into the DTO is not possible. Pinning
// means a preset-semantics change is DETECTED rather than silently absorbed.
package MatrixViews {
    private import Views::*;
    private import Mercurio::*;

    part def Component;
    action def Function;

    package Components { part c1 : Component; part c2 : Component; }
    package Functions { action f1 : Function; action f2 : Function; }

    view 'allocation matrix' {
        @ViewRenderParams {
            matrixPreset = "allocation";
            relations = ("allocate");
        }
        render asRelationshipMatrix;

        view rows { expose Components::**; }
        view columns { expose Functions::**; }
    }
}
