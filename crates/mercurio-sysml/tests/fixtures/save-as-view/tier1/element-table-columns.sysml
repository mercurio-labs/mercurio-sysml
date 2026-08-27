// SV-0 fixture (tier 1). The most interoperable row in the whole map:
// asElementTable already models ordered columnView[0..*], each a nested view
// with its own viewRendering (Views.sysml:146-163). TableColumnSpecDto maps
// onto it directly. Invent nothing here.
package TableViews {
    private import Views::*;

    part def Component;
    part alpha : Component;
    part beta : Component;

    rendering asNameAndDoc :> asElementTable {
        view :>> columnView[1] {
            expose name;
            render asTextualNotation;
        }
        view :>> columnView[2] {
            expose documentation;
            render asTextualNotation;
        }
    }

    view 'component table' {
        expose TableViews::**;
        render asNameAndDoc;
    }
}
