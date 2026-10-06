# Extraction-ready definition-driven core preview

This is the bounded immediate deliverable approved on 2026-10-06. The [six-item checklist](remaining-gap-checklist.md), [fixed contract](definition-core-preview-contract.json) and [acceptance report](definition-core-preview-acceptance.json) define its finish line. Full 2026-08 qualification is deferred. The candidate is unpromoted.

## Supported subset and evidence

The contract freezes the existing independent Pilot whole-document controls: 90 positive documents, eight syntax negatives, 1,403 source-constructed canonical nodes and 2,595 stored-reference observations. Each contract row identifies the source by SHA-256 and its language/context. The exact source inventory is in the independent `definition-document-pilot-controls.json` and exported `sources.json`; source hashes and expected stages cannot be substituted.

Supported storage/linking controls include packages and nested namespaces; named declarations and usages; qualified references, aliases, private imports and shadowing; ordinary typing, specialization and conjugation; and selected inherited/default, positional, action/calculation and port structures supplied by the self-contained controls. Some controls declare standard-library roots locally. These establish the specific stored-model behavior, not full behavior of those SysML constructs or qualification against every actual library.

| Category | Imported representation | Native consumer / verified preview behavior | Outside the preview claim |
| --- | --- | --- | --- |
| Ecore classifiers, inheritance, primitive and enum types | Resolved effective Ecore plus faithful source/annotation extract | Generated contracts and shared shape/ancestry checks; 19 Ecore controls, including all 87 stored declarations | All class-specific semantic consequences |
| Multiplicities, containment/opposites, ordering/uniqueness, defaults and flags | All 415 declarations retain metadata and defaults | Canonical construction/integrity checks and the frozen stored-model matrix | Complete required-derived-value validation; every mutation context |
| Ecore annotations, 70 operations and delegate bindings | Retained in structured definitions with resolved dependency identities | Available to consumers; only explicitly selected bounded services execute | General operation/delegate execution, signatures counted as support |
| Xtext inheritance, terminals, alternatives/groups, cardinalities, assignments/actions and cross-references | Three resolved grammars; separate language programs; imported terminal and assignment contracts | Shared native interpreter, construction and supported linking; exact positive/negative matrix | Complete language/rule coverage, every predicate/prediction site and arbitrary nesting/resource demand |
| Scoping, naming and parser postprocessing | Imported dispatch/contracts plus identified external dependencies | Explicit handwritten Rust algorithms exercised by the subset; effective owning-membership names are checked by the existing exact Pilot test | Universal scope/provider closure, every inherited/positional strategy and unresolved names treated as valid |
| Persistence and exports | Canonical KIR and abstract-syntax JSON | Native serialization; fresh abstract-syntax import and repeated model comparison | Full semantic publication/validation or persistence of all unimplemented derived behavior |

## Reproduce and consume

Run from `mercurio-sysml`:

```powershell
python -B tools/qualify_definition_core_preview.py
```

Each acceptance run executes five generation checks, adversarial acceptance tests, focused native tests and the actual export with Java unavailable. No live Pilot run, full release corpus or performance benchmark is required at this boundary. Cached observations retain the pinned upstream revision and source/tool provenance.

The successful output includes:

- `acceptance.json`: six-item result, exact matrix counts and fingerprints.
- `bundle/definitions/`: resolved Ecore/Xtext extracts and generated program data, including faithful annotations/delegate metadata and explicit unsupported sites.
- `bundle/sources.json` and `bundle/models.jsonl`: fixed source inputs and native KIR/abstract-syntax/fresh-import results.
- `bundle/native-entrypoints.json`: native APIs, required workspace dependency, imported consumers and explicitly handwritten algorithms.
- `bundle/definition-core-preview-contract.json`: fixed supported subset and exclusions.

The existing maintainer binary also exposes `audit_release_compile --definition-core-preview <spec.json> <models.jsonl>`. The spec contains `cases`, each with a distinct `case_id`, `language` (`kerml` or `sysml`) and `source`. Optional `reference_queries` contain canonical `owner_path`/`field` selectors, never target expectations; their original and fresh-import native views are exported separately from raw storage. Successful records have `status=core_constructed`; errors retain their actual syntax/lexical/unsupported/resource/artifact/construction/reference-projection/persistence stage and export no partial model. `semantic_validation` and `transformation_completion` remain `not_assessed`.

The reusable native core stays in `mercurio-sysml`, using `mercurio-foundation`. Its existing entry points are `definition_document::parse_and_link`, `parse_and_link_sources`, `inspect_source_structure`, `query_constructed_references`, and the abstract-syntax import/export APIs. This preview packages structured definitions and model exports for reuse/extraction; **an independently extracted or published Rust crate is not this milestone**. Source identities are structural identities and are not guaranteed stable under arbitrary edits.

## Preserved later work

[Full-release acceptance](full-qualification-checklist.md) is byte-for-byte preserved from the prior plan. [Preservation evidence](definition-core-preview-preservation.json) fingerprints unfinished source and evidence, preserves the R42/R43/R44 candidate records, and records archived executable/source witnesses. The old candidate's 119-resource/94-library component checkpoint remains component evidence; it is not promoted into preview or full-release qualification.

Deferred work includes full implicit/superclass transformation, result/self-result connectors, enclosing Feature/Expression lifecycle, complete bounds and semantic validators, remaining derived operations/delegates and global resources, unresolved normative disagreements and n-ary crossing, every release sample and family, detailed final semantic comparison, paired timing, and release promotion. Additional Xtend translation is deferred unless separately authorized for a later milestone.

The written specification remains normative and cached Pilot observations remain independent implementation evidence. Structural agreement does not resolve semantic disagreements; their existing reviews stay intact. Preview acceptance is **6/6 items**, separate from full-release qualification, currently **0/34 families**. A successful preview does not trigger the celebration requested for the first qualified family.
