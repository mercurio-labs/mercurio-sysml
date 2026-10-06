# Definition-driven core preview: immediate completion boundary

**Complete: 6/6 preview acceptance items, 90 supported documents and eight syntax negatives.** The [acceptance report](definition-core-preview-acceptance.json) records the verified run.

The user re-scoped the immediate goal on 2026-10-06. Complete this bounded preview and stop at its acceptance boundary. Full 2026-08 semantic qualification is a later milestone, with its original criteria preserved in [full-qualification-checklist.md](full-qualification-checklist.md). The candidate remains unpromoted.

The fixed [preview contract](definition-core-preview-contract.json) admits **90 independent positive document controls and eight syntax negatives**. Verification covers parsing, source-constructed canonical storage, supported self-contained linking, export and fresh persistence. It does not qualify complete semantic transformation or validation. A source outside this matrix is outside the verified preview claim even if the interpreter can parse it.

## Six acceptance items

- [x] P1 — Preserve unfinished semantic code, candidate records, evidence, executable witnesses and unchanged full-release acceptance; verify their fingerprints.
- [x] P2 — Verify pinned, resolved Xtext/Ecore definitions and current generated native contracts/programs, retaining annotations, operations and delegates as data with explicit implementation boundaries.
- [x] P3 — Pass all 90 supported document controls and eight prescribed syntax rejections against cached independent Pilot observations; compare canonical nodes, effective stored attributes, ordered containment and stored endpoints.
- [x] P4 — Export native KIR and abstract syntax, import into fresh documents and repeat structural comparison; reject incomplete results and overstated stage claims.
- [x] P5 — Run the native export with Java unavailable on PATH; document imported consumers and handwritten scoping, lowering, naming, integrity and persistence dependencies.
- [x] P6 — Produce a fingerprinted portable definition/model bundle, supported-subset inventory, exclusions and a reproducible acceptance report.

## Execution and stop condition

From this workspace, run `python -B tools/qualify_definition_core_preview.py`. The command verifies current generators, focused native/control tests, actual exported models, input integrity and preservation. It writes a new run under `target/definition-core-preview` and updates [definition-core-preview-acceptance.json](definition-core-preview-acceptance.json) only on success.

Finish when all **six preview acceptance items** pass. Do not resume the enclosing SysML lifecycle, expand Xtend translation, run all release samples, benchmark, promote the candidate or claim a qualified semantic family as part of this preview.

The [preview handoff](definition-core-preview.md) defines the extraction boundary and exclusions. The preserved full-release ledger still has **0/26 contexts, 0/10 obligations, 0/34 families and 0/5 release gates** qualified; those denominators do not measure preview completion.
