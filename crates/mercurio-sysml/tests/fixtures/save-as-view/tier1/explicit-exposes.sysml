// SV-0 fixture (tier 1). A curated set: one expose per element. This is what a
// saved exploration compiles to -- it records the pruned set exactly and is
// immune to traversal drift. Requires DiagramQueryOptionsDto.scope, which SV-3
// adds (additive, serde default Traversal); without it there is no field to
// read a materialized diagram view back into.
package CuratedViews {
    private import Views::*;

    part def Alpha;
    part def Beta;
    part def Gamma;

    view 'curated pair' {
        expose Alpha;
        expose Gamma;
        render asTreeDiagram;
    }
}
