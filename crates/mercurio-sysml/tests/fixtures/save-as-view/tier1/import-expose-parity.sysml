// SV-0 fixture (tier 1) — the parity case that shapes SV-1's implementation.
//
// `expose` is import-like: the SysML spec gives it the same namespace-query
// syntax as `import` (recursive `::**`, metadata predicate `[@Meta]`). The
// pilot's own training material uses that syntax on the import side —
// `40. Filtering/Filtering Example-2.sysml:28` reads
// `public import vehicle::**[@Safety];`.
//
// Mercurio today handles the two paths DIFFERENTLY, and neither is correct:
//
//   construct   `::**` wildcard          `[@Meta]` predicate
//   ---------   ----------------------   -------------------
//   import      preserved                silently dropped
//   expose      dropped (mis-parsed as   dropped
//               a usage declaration)
//
// Two consequences SV-1 must act on:
//
//  1. `expose` should route through the EXISTING import parsing path, which
//     already carries `::**`. It is not a greenfield construct.
//  2. The dropped predicate is a SHARED gap, and on the import side it is a
//     standing correctness bug independent of this plan: the pilot's
//     'Safety Features' package currently resolves as `vehicle::**` — every
//     part — rather than only the @Safety-annotated ones. Fixing the predicate
//     once fixes both paths.
package ParityViews {
    private import Views::*;

    metadata def Safety;

    part vehicle {
        part brake { @Safety; }
        part radio;
    }

    // Import side: wildcard survives, predicate does not.
    package 'Safety Features' {
        public import vehicle::**[@Safety];
    }

    // Expose side: neither survives. Both views below currently parse to the
    // identical member list (`expose vehicle` + `render asTreeDiagram`), which
    // is precisely why SV-2's exit criterion demands that they diverge.
    view 'safety view' {
        expose vehicle::**[@Safety];
        render asTreeDiagram;
    }

    view 'everything view' {
        expose vehicle::**;
        render asTreeDiagram;
    }
}
