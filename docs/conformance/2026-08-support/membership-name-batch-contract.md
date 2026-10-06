# Stored Membership name batch

The integrated resource assessment rejects a Membership with no explicit names. The normative pinned KerML specification defines both stored names as optional (PDF pages 150 and 152, section 8.3.2.4.3); its OwningMembership names are derived redefinitions. The pinned resolved Ecore specifies memberName and memberShortName as optional stored EString attributes (0..1), with no setting delegate or default literal. The pinned generated Membership getters return null for absent values. This is distinct from OwningMembership's derived names, which retain their existing effective-element-name dependency.

| Feature | Imported representation | Native consumer | Verification / remaining boundary |
|---|---|---|---|
| Membership long/short names | Resolved Ecore attribute, flags, multiplicity, redefinition and null default confirmed by pinned getters | membership_names_indexed_query validates the contract and each present value, preserving long/short order and empty strings | Six independent Pilot controls, 24 persisted/reordered contexts, repeated calls, malformed-value rejection and graph edits |
| Owning membership names | Resolved ownership and generated effective-name dispatch | Existing generated_effective_names_inner_query | Existing controls retained; unresolved name prerequisites remain explicit |
| Import conflicts | Shared membership-name consumer and canonical endpoint reads | indexed_import_conflicts | Endpoint failure/deferred ordering and malformed later-name rejection retained; integrated closure remains required |

This batch does not implement another delegate, infer completion, alter endpoint readiness, publish the candidate or issue a family certificate. Boundary: full native definition regression, candidate-pipeline tests, optimized build and one fresh same-input 24-library/caller assessment. Subsequent dependencies are recorded explicitly.
