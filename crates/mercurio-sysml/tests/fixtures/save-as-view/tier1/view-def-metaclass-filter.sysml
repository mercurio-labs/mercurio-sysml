// SV-0 fixture (tier 1, deliberately narrow). A view def carries the reusable
// query shape; the usage binds it to a root. This is the lens / saved-view
// distinction the plan turns on: a view def is a root-free lens, a view usage
// is a root-bound saved view.
//
// NOTE the boundary this fixture records: `filter` maps to TableRowTypeDto ONLY
// when the filtered name is a metaclass. SysML filter conditions are evaluated
// against an element's metadata features plus its implicit metaclass feature --
// never its specialization hierarchy -- so a user-definition row type with
// include_subtypes has NO filter encoding and is tier 2.
package FilteredViews {
    private import Views::*;

    view def 'Part Structure View' {
        filter @SysML::PartUsage;
    }

    part def Chassis;
    part vehicle {
        part chassis : Chassis;
    }

    view 'vehicle parts' : 'Part Structure View' {
        expose vehicle::**;
        render asElementTable;
    }
}
