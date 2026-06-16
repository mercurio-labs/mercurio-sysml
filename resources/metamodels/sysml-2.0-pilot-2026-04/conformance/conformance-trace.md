# Pilot Conformance Trace

- Release: `2026-04`
- Status: `passed`
- Generated: `2026-06-16T19:23:05.2693421Z`
- Profile: `sysml-2.0-pilot-2026-04`
- Corpus: `small`
- Source lock: `locks/source.lock.json`

## Source Fingerprints

- Mercurio commit: `5ae5cd5b7b04b26b0fce12a879d3f1c44f5a0dc0`
- Mercurio branch: `feature/promote-pilot-2026-04-resources`
- Mercurio dirty: `true`
- Mercurio tracked tree SHA256: `ae6cf0d8b495971baa8fde0790238dacea525e1953acd81a3199854d966e72c5`
- Pilot commit: `241934dab96f2f07ddbfd7b351449cb975148064`
- Pilot branch: `master`
- Pilot dirty: `false`
- Pilot tracked tree SHA256: `69462b8d341eb1193e89529f466ee236636f99a19ed2ac8a335193e37b9c98e8`

### Workspace Repositories

| Repository | Branch | Dirty | Commit | Tree SHA256 |
|---|---|---:|---|---|
| `mercurio-ai` | `main` | `false` | `78540fbef25685373091dc236394aa9af392e126` | `4a00dbb9774b7a10fbd99c3cfa2b126705213a894c178ff0ca41e034067936b0` |
| `mercurio-examples` | `main` | `true` | `a2c832033fce724e7bc54f76a865dbffc8fc0eba` | `5480cf0b01a50811786c4ab4e4c155bfae60c505525c7b54aadd8a65eaeeddd7` |
| `mercurio-foundation` | `main` | `false` | `73c8e2198c53e8aa4ab32306d81f149850e8cf53` | `5eff1ff678530329c77f37688164dff117b41adda38798af5138de43e0687dec` |
| `mercurio-host-adapters` | `main` | `true` | `1eb5d78be82669f38a7d69d745af4b0f9b1e53c0` | `f24b6c30fa62c0b80c6821b8b4644e6645c0f3030b0a370f1caf8c1a713556cd` |
| `mercurio-plugins` | `main` | `false` | `be9076df297624705759775ea0d00bc9cc5045c7` | `e5adfbe054dff3964c2b8f9e1b96fa6a4d8c244c32cb7a4b7be088a848d3393b` |
| `mercurio-product` | `feature/lab-studio-redesign` | `true` | `dac9b932f6cac0067eee3ab77eafb60dded1a67d` | `4d34438863bd512c4a6e1d541b5265f0fa5196f7474b8f7a9651653e03dd9600` |
| `mercurio-sysml` | `feature/promote-pilot-2026-04-resources` | `true` | `5ae5cd5b7b04b26b0fce12a879d3f1c44f5a0dc0` | `ae6cf0d8b495971baa8fde0790238dacea525e1953acd81a3199854d966e72c5` |


| Stage | Status | Duration ms | Report | Metrics |
|---|---:|---:|---|---|
| `candidate_staging` | `passed` | 2938 | `C:\dev\git\mercurio\target\pilot-release-2026-04-promote-small\reports/candidate-staging.json` | `{"candidate_root":"C:\\dev\\git\\mercurio\\target\\pilot-release-2026-04-promote-small\\candidate/resources/metamodels\\sysml-2.0-pilot-2026-04","file_count":17,"tree_sha256":"d2796e83ba34e5bf47b2fcb99605e2adaeac78694a26ce4a3946d89093280c68"}` |
| `pilot_java_artifacts` | `passed` | 4638 | `../external\SysML-v2-Pilot-Implementation\org.omg.sysml.interactive/target\org.omg.sysml.interactive-0.57.0-SNAPSHOT-all.jar` | `{"artifact":"org.omg.sysml.interactive-*-all.jar"}` |
| `python_wrappers` | `passed` | 883 | `C:\dev\git\mercurio\target\pilot-release-2026-04-promote-small\stdlib/python` | `{"file_count":9,"metamodel_class_count":13,"module":"mercurio_sysml_2_0","py_compile_exit_code":0,"python_file_count":8,"stdlib_catalog_entries":{"isq":0,"si":0}}` |
| `syntax_parity` | `passed` | 47714 | `C:\dev\git\mercurio\target\pilot-release-2026-04-promote-small\reports/syntax-parity.json` | `{"accepted_difference_gate":{"accepted":[],"accepted_differences":0,"total_differences":0,"unaccepted":[],"unaccepted_differences":0},"aggregate":{"exact_match_cases":5,"failed_cases":0,"total_mismatches":0,"total_pilot_only":0,"total_rust_only":0},"case_count":5}` |
| `semantic_parity` | `passed` | 297456 | `C:\dev\git\mercurio\target\pilot-release-2026-04-promote-small\reports/semantic-parity.json` | `{"accepted_difference_gate":{"accepted":[],"accepted_differences":0,"total_differences":0,"unaccepted":[],"unaccepted_differences":0},"aggregate":{"compare":{"avg_ms":2,"max_ms":10,"median_ms":2,"min_ms":0,"total_ms":14},"exact_match_cases":5,"failed_cases":0,"mercurio":{"avg_ms":2813,"max_ms":3117,"median_ms":2908,"min_ms":2372,"total_ms":14067},"pilot":{"avg_ms":5987,"max_ms":7948,"median_ms":6318,"min_ms":4303,"total_ms":29936},"total_mercurio_only":0,"total_mismatches":0,"total_pilot_only":0},"case_count":5}` |
| `compile_diagnostics_parity` | `passed` | 6312 | `C:\dev\git\mercurio\target\pilot-release-2026-04-promote-small\reports/compile-errors-parity.json` | `{"accepted_difference_gate":{"accepted":[],"accepted_differences":0,"total_differences":0,"unaccepted":[],"unaccepted_differences":0},"aggregate":{"both_fail_cases":0,"both_pass_cases":5,"failed_cases":0,"pilot_only_fail_cases":0,"primary_problem_match_cases":0,"rust_only_fail_cases":0,"status_match_cases":5},"case_count":5}` |
| `candidate_promotion` | `passed` | 2192 | `C:\dev\git\mercurio\target\pilot-release-2026-04-promote-small\reports/candidate-promotion.json` | `{"file_count":17,"marked_latest":false,"promoted_root":"C:\\dev\\git\\mercurio\\mercurio-sysml\\resources/metamodels\\sysml-2.0-pilot-2026-04","registry":"C:\\dev\\git\\mercurio\\mercurio-sysml\\resources/metamodels\\registry.json"}` |
