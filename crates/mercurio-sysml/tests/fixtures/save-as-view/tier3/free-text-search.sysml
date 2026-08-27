// SV-0 fixture (tier 3 -- NOT reifiable, and that is the correct answer).
// ModelViewSpecDto.query is a free-text search string. A search is not a view:
// no scope, no notation, no stable element set. SV-3 must return a typed
// NotReifiable naming the offending field rather than inventing an encoding.
//
// There is deliberately no .view.json beside this file; the expectation is a
// refusal, recorded in manifest.json.
package SearchViews {
    private import Views::*;
    part def Anything;
}
