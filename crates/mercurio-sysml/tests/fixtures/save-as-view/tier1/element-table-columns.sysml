// SV-0 fixture. Tier 1 for column *structure*; tier 2 for what a column shows.
//
// `asElementTable` models ordered `columnView[0..*]` natively
// (Views.sysml:146-163), so multiple columns are real standard content. They are
// SUBSETS of that feature, not redefinitions: `:>` adds instances to a `[0..*]`
// feature, `:>>` replaces it. The pilot only ever writes one column --
// `view :>> columnView[1]`, where `[1]` is the multiplicity of the redefining
// feature, not an index (Views Example.sysml:16-20) -- so two redefinitions of
// `columnView` in one body are two definitions of the same feature, which is why
// the compiler rejects them as a duplicate id. That is the compiler being right.
//
// What a column SHOWS has no standard encoding at all: the pilot's column view
// contains only `render asTextualNotation`. So `TableColumnSpecDto.path` is
// tier 2 and rides the #Mercurio annotation (V-6.4). Only `key` and `label` --
// the column feature's own declared name, which is real model content -- are
// tier 1.
package TableViews {
    private import Views::*;

    part def Component;
    part alpha : Component;
    part beta : Component;

    rendering asNameAndDoc :> asElementTable {
        view name :> columnView {
            render asTextualNotation;
        }
        view documentation :> columnView {
            render asTextualNotation;
        }
    }

    view 'component table' {
        expose TableViews::**;
        render asNameAndDoc;
    }
}
