'''Publish compact reports; retain full compressed snapshots in the assessment directory.'''
import argparse, csv, datetime, json, os, shutil
from pathlib import Path
ROOT=Path('../target/assessment-2026-08-detailed')
DEST=Path('docs/conformance/2026-08-assessment')
def read(path): return json.loads(Path(path).read_text(encoding='utf-8'))
def write(path,value): Path(path).write_text(json.dumps(value,indent=2)+'\n',encoding='utf-8')
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,default=ROOT)
    parser.add_argument('--dest',type=Path,default=DEST)
    args=parser.parse_args();root,dest=args.root,args.dest
    semantic=read(root/'semantic-analysis.json');performance=read(root/'performance-analysis.json');lock=read(root/'source-lock.json')
    stdlib_argument = ' --stdlib "' + Path(os.path.relpath(lock['stdlib_path'], Path.cwd())).as_posix() + '"' if lock.get('stdlib_path') else ''
    if semantic['scope']['recorded_targets']!=310 or performance['total_measurements']!=48:
        raise ValueError('assessment has not completed every planned run')
    dest.mkdir(parents=True,exist_ok=True)
    artifact_root=Path(os.path.relpath(root.resolve(),dest.resolve())).as_posix()
    compact={k:v for k,v in semantic.items() if k not in ('differences','cases')}
    compact['cases']=[{k:v for k,v in c.items() if k!='artifact'} for c in semantic['cases']]
    write(dest/'semantic-artifacts.json',{'artifact_root':artifact_root,'cases':[{'relative_path':c['relative_path'],'path':Path(c['artifact']['path']).resolve().relative_to(root.resolve()).as_posix(),'uncompressed_sha256':c['artifact']['uncompressed_sha256']} for c in semantic['cases']]});
    write(dest/'semantic-summary.json',compact);write(dest/'semantic-differences.json',semantic['differences']);write(dest/'performance-summary.json',performance)
    for name in ['performance-results.json','build-provenance.json']:
        shutil.copyfile(root/name,dest/name)
    write(dest/'source-lock.json',{k:v for k,v in lock.items() if k!='groups'})
    with (dest/'semantic-cases.csv').open('w',newline='',encoding='utf-8') as f:
        fields=['relative_path','native_elements','pilot_elements','exact_pairs','mismatched_pairs','native_only','pilot_only']
        writer=csv.DictWriter(f,fieldnames=fields);writer.writeheader()
        for c in semantic['cases']:writer.writerow({k:c[k] for k in fields})
    groups={g['id']:g for g in lock['groups']};table=[];csv_rows=[]
    for row in performance['groups']:
        g=groups[row['group']];name=g['cases'][0].rsplit('/',1)[0];n=row.get('native');p=row.get('pilot')
        if not n or not p:continue
        values=[name,str(len(g['cases'])),f"{n['wall_ms']['median']/1000:.3f}",f"{p['wall_ms']['median']/1000:.3f}",f"{n['model_work_ms']['median']/1000:.3f}",f"{p['model_work_ms']['median']/1000:.3f}"]
        table.append('| '+' | '.join(values)+' |')
        csv_rows.append({'source_group':name,'targets':len(g['cases']),'native_wall_median_ms':n['wall_ms']['median'],'native_wall_min_ms':n['wall_ms']['min'],'native_wall_max_ms':n['wall_ms']['max'],'pilot_wall_median_ms':p['wall_ms']['median'],'pilot_wall_min_ms':p['wall_ms']['min'],'pilot_wall_max_ms':p['wall_ms']['max'],'native_model_phase_median_ms':n['model_work_ms']['median'],'pilot_model_phase_median_ms':p['model_work_ms']['median'],'native_runs':n['repetitions'],'pilot_runs':p['repetitions']})
    if csv_rows:
        with (dest/'performance.csv').open('w',newline='',encoding='utf-8') as f:
            writer=csv.DictWriter(f,fieldnames=list(csv_rows[0]));writer.writeheader();writer.writerows(csv_rows)
    raw=semantic['raw_aggregate'];proj=semantic['named_declaration_projection'];expression=semantic.get('ordered_expression_projection', {}).get('counts', {});examples=[]
    for field,file in [('metaclass','ConstraintTest.sysml'),('metaclass','ConjugationTest.sysml'),('metaclass','Dependencies.kerml'),('is_variable','PartTest.sysml'),('initializer','Expressions.kerml')]:
        candidates=[d for d in semantic['differences'] if d['property']==field and d['source'].endswith(file)]
        if field=='metaclass' and file=='ConstraintTest.sysml':
            candidates.sort(key=lambda d:d['native']!='AssertUsage')
        if candidates:
            d=candidates[0];examples.append(f"- `{d['source']}:{d['line']}`, `{d['name']}`: **{field}** differs. Native `{d['native']}`; Pilot `{d['pilot']}`.")
    provenance=read(root/'build-provenance.json')
    maintenance_note = ("The standalone validator inventory extractor and its inventory/coverage metadata\nwere corrected during this run; their before/after hashes are recorded separately.\nThe frozen source snapshot and measured compiler binaries are retained unchanged."
                        if provenance.get('maintenance_changes_during_assessment') else
                        "The source snapshot, assessment scripts and measured compiler binaries were frozen for this run.")
    oracle_note = ""
    if (root/'oracle-reuse-provenance.json').exists():
        shutil.copyfile(root/'oracle-reuse-provenance.json',dest/'oracle-reuse-provenance.json')
        oracle_note = "This run reuses only the preceding run's unchanged Pilot oracle exports, verified by source-set, runtime, helper and export hashes. All native comparisons and all timing trials were rerun. See `oracle-reuse-provenance.json`."
    run_root=Path(os.path.relpath(root.resolve(), Path.cwd())).as_posix()
    text=f'''# 2026-08 semantic and performance assessment

Executed {datetime.datetime.now(datetime.timezone.utc).isoformat()}. This assesses
the candidate worktree; **it is not release qualification**. Source, library,
runtime and binary hashes are recorded alongside this report. The native compiler and Pilot runtime were held fixed during measurement.
The exporter records ordered reference sequences separately from the legacy edge
set. Source/build provenance identifies the exact helper used for this run.
{maintenance_note}

{oracle_note}

## Detailed semantic comparison

All 310 targets were attempted in **96 isolated, identical source sets**. Release
and Pilot files were checked byte for byte. Native uses the candidate library;
Pilot uses commit `{lock['pilot_commit']}` and the 0.62.0 runtime. The semantic
source oracle uses `CheckMode.ALL` with its precomputed index disabled. The
performance run uses the published index, as described below.

- Completed comparisons: **{raw.get('compared',0)}/310**; failures/incomplete:
  **{310-raw.get('compared',0)}** (listed in `semantic-summary.json`).
- Source elements: **{raw.get('native_elements',0):,} native**, **{raw.get('pilot_elements',0):,} Pilot**.
- Raw paired results: **{raw.get('exact_pairs',0):,} exact**, **{raw.get('mismatched_pairs',0):,} mismatched**.
- Unpaired: **{raw.get('native_only',0):,} native-only**, **{raw.get('pilot_only',0):,} Pilot-only**.

These are not counts of independent semantic bugs. Pilot materializes membership,
typing, redefinition and expression objects that native KIR often stores as
properties. Identifiers, scalar/list forms, derived/default fields, order and
duplicate relationships also differ. Anonymous objects on one source line can be
paired imperfectly. No tolerance or exclusion turns these differences into passes.

Separate triage matches only unique `(declared name, start line)` pairs within a
source file. It maps reference IDs through observed pairs in the same source
group and compares six reference fields as sets. It does not declare absent
fields, anonymous objects or missing expressions equivalent. Raw reports remain:

- **{proj.get('unique_named_pairs',0):,}** uniquely paired named declarations.
- **{proj.get('relation_fields_equal_after_explicit_projection',0):,}/{proj.get('relation_fields_checked',0):,}** checked reference fields agree after the stated projection.
- **{proj.get('relation_fields_missing_direct_counterpart',0):,}** reference fields present on only one side are outside that denominator.
- **{proj.get('metaclass_mismatches',0):,}** metaclass differences and
  **{proj.get('boolean_differences',0):,}** direct Boolean differences.
- The eight additional preserved Boolean fields have **{proj.get('extended_boolean_differences',0):,}** differences across **{proj.get('extended_boolean_fields_checked',0):,}** direct comparisons; **{proj.get('extended_boolean_fields_missing_counterpart',0):,}** missing counterparts remain unassessed. The earlier six-field Boolean count above retains the prior assessment's scope.
- **{proj.get('pilot_initializer_without_native_expression_ir',0):,}/{proj.get('named_declarations_with_pilot_initializer',0):,}** matched declarations with a Pilot `FeatureValue` lack initializer IR on that native declaration. Inspect alternate lowering before treating every case as lost semantics.
- **{proj.get('name_or_direction_differences',0):,}** missing/different nonempty Pilot short-name or direction fields.
- Ambiguous named anchors excluded from declaration pairing: **{proj.get('ambiguous_named_anchors',0):,}**; **{proj.get('pilot_named_anchors_without_native_pair',0):,}** Pilot named anchors have no native pair (not automatically lost semantics).
- **{proj.get('literal_initializers_checked',0):,}** literal initializer values compared;
  **{proj.get('literal_initializer_mismatches',0):,}** literal-value differences.

Examples requiring implementation review:

{chr(10).join(examples)}

`semantic-differences.json` records every triaged source/line/property difference;
`semantic-cases.csv` records per-file counts. Ordered initializer-tree observations
are in `semantic-summary.json` under `ordered_expression_projection`: supported
literal, reference, operator and invocation forms retain argument order and
multiplicity. Missing evidence, unsupported forms, ambiguous references and cycles
remain explicitly unassessed. Tree equality does not establish type, implicit
relationship or evaluation parity. Recorded ordered-tree observations are
`{json.dumps(expression, sort_keys=True)}`. Different trees may reflect lazy argument
wrappers or operator spellings (such as `^` versus `**`) in Pilot; inspect the
retained trees before classifying a structural difference as a semantic defect.

The exporter includes every source EAttribute (including literal values and
operators), every stored non-container source EReference, and its existing
derived-reference whitelist. Stored and selected derived references additionally retain ordered target lists, including duplicates and empty lists. Every exported source element is retained by the
strict snapshot. This is not an exhaustive proof for every computed EMF reference
or OMG constraint. The bounded tree projection does not establish all expression forms, multiplicity semantics, or behavioral execution. Registry-query
fallbacks may retain direct properties without
establishing full derived-value coverage. Full compressed snapshots and raw diffs
remain under `{artifact_root}/groups/`;
`semantic-artifacts.json` records their paths and uncompressed SHA-256 digests.

## Performance assessment

**{performance['successful_measurements']}/{performance['total_measurements']}** measured executions succeeded.
Eight groups cover 73 targets across both languages, small models, the two broad Simple Tests
suites, behavior/requirements training, and the specification vehicle example.
Each engine runs three times per group in fresh processes; engine order alternates
across trials and groups. Within each group both engines read/parse inputs once;
native compiles targets sequentially and Pilot validates loaded resources
sequentially, retaining their ordinary in-process caches. Below are median seconds. `performance.csv` includes
min/max; `performance-results.json` includes every phase and trial.

| Source group | Targets | Native process | Pilot process | Native model phase | Pilot model phase |
| --- | ---: | ---: | ---: | ---: | ---: |
{chr(10).join(table)}

Process time includes startup and library loading through completed target
compilation/validation. Model phase means input reading/parsing plus native
compilation/validation or Pilot validation (including lazy resolution). It excludes
explicit initialization and library-load phases. Library resolution can still
occur lazily, so subtracting load time does not perfectly equalize preparation.
Graph materialization, export, comparison, compression and builds are excluded.

Native uses Cargo `--release` and prebuilt KIR. Pilot loads source libraries with
the pinned release's checked-in `.index.json` enabled, its normal configuration.
The OS file cache is warm/uncontrolled, not flushed; the JVM is fresh per trial.
This is not a warm incremental-edit or UI benchmark. Pilot's maximum heap is
3 GiB. Host: Windows 11, Intel i7-11850H (8 cores/16 threads), approximately 64 GiB
RAM. Versions and hashes are in `build-provenance.json`.

**These timings compare current workflows, not equivalent semantic work.** Pilot
runs its complete validator set; native validation and preservation are incomplete.
Lower native time is not a speedup claim for a fully conformant compiler. Three
trials provide descriptive medians/ranges, not confidence intervals or tail latency.

## Reproduce

From `mercurio-sysml`, after preparing the pinned sources/runtime and candidate
library in the [implementation checkpoint](../2026-08-support/README.md):

```powershell
cargo build --release --locked -j 1 -p mercurio-tools --features legacy-pilot-tools --bin compare_pilot_semantics --bin audit_release_compile
javac -cp ../target/upstream/kernel-0.62.0/sysml/jupyter-sysml-kernel-0.62.0-all.jar -d {run_root}/classes tools/pilot-exporter/src/main/java/dev/mercurio/pilot/PilotModelExporter.java
python -B tools/run_release_assessment.py --out {run_root} --stage prepare{stdlib_argument}
python -u -B tools/run_release_assessment.py --out {run_root} --stage semantics{stdlib_argument}
python -u -B tools/run_release_assessment.py --out {run_root} --stage performance{stdlib_argument}
python -B tools/summarize_release_assessment.py --root {run_root}
python -B tools/publish_release_assessment.py --root {run_root} --dest {dest.as_posix()}
```

The semantic stage exports up to three independent source sets concurrently; all
workers finish before sequential benchmark trials start. The runner checks hashes,
isolates source sets, retains timeout/failure records
and resumes completed semantic groups. Cached exports must declare the exact
requested support set. Use a new output directory when changing compiler inputs;
do not combine candidates.
'''
    (dest/'README.md').write_text(text,encoding='utf-8')
    print(dest/'README.md')
if __name__=='__main__':main()
