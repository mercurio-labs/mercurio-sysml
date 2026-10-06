import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from qualification_certificates import SCHEMA, CERTIFICATE_SCHEMA, GATES, digest, targets, target_identity, validate_certificates


class QualificationCertificateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name, content in {"native.rs": "fn consume() {}", "normative.txt": "Required normative behavior", "review.txt": "Reviewed complete contexts", "oracle.json": '{"independent": true}'}.items():
            (self.root / name).write_text(content, encoding="utf-8")
        self.plan = {"qualification_contracts": "contracts.json", "dependencies": {"scoping": {"remaining": "All scope origins", "status": "partial"}}, "release_gates": [{"id": name, "acceptance": "Complete " + name, "status": "pending"} for name in sorted(GATES)]}
        self.checked = [{"id": "ecore.types", "upstream_ids": ["A", "B"], "applicability": "used_by_pin", "remaining_required_behavior": "All type semantics", "semantic_dependencies": [], "completion": "open"}]
        self.contracts = {"schema": SCHEMA, "targets": {key: {"identity": target_identity(target), "review_status": "unreviewed", "obligations": []} for key, target in targets(self.plan, self.checked).items()}}
        self.certificates = {}
        self.reports = {}
        (self.root / "samples").mkdir()
        (self.root / "samples/model.kerml").write_text("package P;", encoding="utf-8")
        (self.root / "samples.json").write_text(json.dumps({"schema": "dev.mercurio.sample-manifest.v1", "source_roots": ["samples"], "samples": {"samples/model.kerml": self.sha("samples/model.kerml")}}), encoding="utf-8")
        (self.root / "observations.json").write_text(json.dumps({"samples/model.kerml": ["nodes"]}), encoding="utf-8")

    def sha(self, name):
        return hashlib.sha256((self.root / name).read_bytes()).hexdigest()

    def current_inputs(self):
        return {"native.rs": self.sha("native.rs")}

    def review(self, name, close=False):
        target = targets(self.plan, self.checked)[name]
        entry = self.contracts["targets"][name]
        entry.update(identity=target_identity(target), review_status="reviewed", inputs={path: self.sha(path) for path in ("native.rs", "normative.txt", "review.txt", "oracle.json")}, review=[{"path": "review.txt", "anchor": "Reviewed complete contexts"}])
        stages = ["gate"] if target["kind"] == "gate" else ["imported", "native", "semantic"]
        rows = []
        for stage in stages:
            contexts = [{"id": polarity, "polarity": polarity, "subjects": target["subjects"], "tests": ["qualification::" + stage + "_" + polarity], "observations": ["oracle.json"]} for polarity in (["positive", "negative", "boundary"] if stage == "semantic" else ["positive"])]
            rows.append({"id": stage, "stage": stage, "subjects": target["subjects"], "acceptance": "Every required behavior", "normative": [{"path": "normative.txt", "anchor": "Required normative behavior"}], "consumers": [{"path": "native.rs", "anchor": "fn consume", "implementation": "handwritten_rust"}], "contexts": contexts})
        entry["obligations"] = rows
        report = self.prepare_report(name, entry) if target["kind"] == "gate" else None
        if not close:
            return
        if name.startswith("family/"):
            next(row for row in self.checked if name == "family/" + row["id"])["completion"] = "complete"
        elif name.startswith("dependency/"):
            self.plan["dependencies"][name.split("/", 1)[1]]["status"] = "complete"
        else:
            next(row for row in self.plan["release_gates"] if name == "gate/" + row["id"])["status"] = "qualified"
        tests = sorted({test for row in rows for context in row["contexts"] for test in context["tests"]})
        log = name.replace("/", "-") + ".log"
        (self.root / log).write_text("".join("test " + test + " ... ok\n" for test in tests) + f"test result: ok. {len(tests)} passed; 0 failed;\n", encoding="utf-8")
        cert_path = name.replace("/", "-") + ".json"
        cert = {"schema": CERTIFICATE_SCHEMA, "target": name, "contract_sha256": digest(entry), "inputs": copy.deepcopy(entry["inputs"]), "native_input_sha256": digest(self.current_inputs()), "verified_contexts": {row["id"]: {context["id"]: sorted(context["subjects"]) for context in row["contexts"]} for row in rows}, "test_log": {"path": log, "sha256": self.sha(log)}, "passed_tests": tests}
        entry["certificate"] = cert_path
        if report is not None:
            report_path = name.replace("/", "-") + "-report.json"
            (self.root / report_path).write_text(json.dumps(report), encoding="utf-8")
            cert["report"] = {"path": report_path, "sha256": self.sha(report_path)}
            self.reports[report_path] = report
        self.certificates[cert_path] = cert

    def prepare_report(self, name, entry):
        if name == "gate/obligation_inventory":
            return None
        report = {"schema": "dev.mercurio.release-qualification-report.v1", "gate": name, "native_input_sha256": digest(self.current_inputs()), "failures": []}
        if name == "gate/native_pipeline":
            report.update(verified_targets=[key for key, value in targets(self.plan, self.checked).items() if value["active"] and value["kind"] != "gate"], stages=["parse", "construct", "link", "validate", "persist"], runtime_dependencies=[{"name": "mercurio", "language": "rust"}])
            return report
        entry["sample_manifest"] = "samples.json"
        for path in ["samples.json", "samples/model.kerml"]:
            entry["inputs"][path] = self.sha(path)
        row = {"sample": "samples/model.kerml", "source_sha256": self.sha("samples/model.kerml")}
        report.update(sample_manifest_sha256=self.sha("samples.json"), samples=[row])
        if name == "gate/sample_pipeline":
            row["stages"] = {stage: "passed" for stage in ("parse", "construct", "link", "validate", "persist")}
        elif name == "gate/pilot_semantics":
            entry["observation_inventory"] = "observations.json"
            entry["inputs"]["observations.json"] = self.sha("observations.json")
            row["observations"] = [{"id": "nodes", "native": 1, "pilot": 1, "outcome": "match"}]
        else:
            policy = {"repetitions": 3, "warmup": 1, "pipeline_boundary": "parse through validation", "max_native_to_pilot_ratio": 2}
            entry["timing_policy"] = policy
            report.update(timing_policy=copy.deepcopy(policy), toolchains={"native": "rustc pinned", "pilot": "jdk pinned"})
            row.update(seconds={"native": [1, 1, 1], "pilot": [2, 2, 2]}, median_ratio=0.5)
        return report

    def qualify_all(self):
        for name in list(self.contracts["targets"]):
            self.review(name, close=True)

    def check(self):
        (self.root / "contracts.json").write_text(json.dumps(self.contracts), encoding="utf-8")
        for name, report in self.reports.items():
            (self.root / name).write_text(json.dumps(report), encoding="utf-8")
        for name, cert in self.certificates.items():
            if "report" in cert:
                cert["report"]["sha256"] = self.sha(cert["report"]["path"])
            (self.root / name).write_text(json.dumps(cert), encoding="utf-8")
        return validate_certificates(self.plan, self.checked, self.root, self.current_inputs)

    def test_unreviewed_inventory_never_reports_completion(self):
        result = self.check()
        self.assertEqual(result["qualified_targets"], [])
        self.assertEqual(result["required_reviewed_obligations"], 0)
        self.assertEqual(result["behavioral_denominator_status"], "unreviewed")

    def test_valid_reviewed_family_certificate(self):
        self.review("family/ecore.types", close=True)
        result = self.check()
        self.assertEqual(result["qualified_targets"], ["family/ecore.types"])
        self.assertEqual(result["closed_reviewed_obligations"], 3)
        self.assertEqual(result["behavioral_denominator_status"], "unreviewed")

    def test_valid_complete_dependency_and_gate_chain(self):
        for name in list(self.contracts["targets"]):
            self.review(name, close=True)
        result = self.check()
        self.assertEqual(len(result["qualified_targets"]), 7)
        self.assertEqual(result["behavioral_denominator_status"], "reviewed")

    def test_source_change_invalidates_certificate(self):
        self.review("family/ecore.types", close=True)
        (self.root / "native.rs").write_text("fn consume() { changed(); }", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "stale fingerprint"):
            self.check()

    def test_transitive_input_change_invalidates_certificate(self):
        self.review("family/ecore.types", close=True)
        original = self.current_inputs()
        self.current_inputs = lambda: {**original, "dependency.rs": "new"}
        with self.assertRaisesRegex(ValueError, "complete native input snapshot"):
            self.check()

    def test_missing_import_native_or_semantic_stage_fails(self):
        for stage in ("imported", "native", "semantic"):
            self.review("family/ecore.types")
            row = self.contracts["targets"]["family/ecore.types"]
            row["obligations"] = [r for r in row["obligations"] if r["stage"] != stage]
            with self.assertRaisesRegex(ValueError, "every subject"):
                self.check()

    def test_every_subject_requires_negative_and_boundary_contexts(self):
        self.review("family/ecore.types")
        row = self.contracts["targets"]["family/ecore.types"]["obligations"][-1]
        row["contexts"][-1]["subjects"] = ["A"]
        with self.assertRaisesRegex(ValueError, "all three context polarities"):
            self.check()

    def test_open_semantic_dependency_blocks_family_closure(self):
        self.checked[0]["semantic_dependencies"] = ["scoping"]
        self.review("family/ecore.types", close=True)
        with self.assertRaisesRegex(ValueError, "open semantic dependencies"):
            self.check()

    def test_inventory_gate_requires_all_contracts_reviewed(self):
        self.review("gate/obligation_inventory", close=True)
        with self.assertRaisesRegex(ValueError, "every active behavioral contract"):
            self.check()

    def test_timing_cannot_close_without_semantics(self):
        self.review("gate/timing_assessment", close=True)
        with self.assertRaisesRegex(ValueError, "open semantic dependencies"):
            self.check()

    def test_removed_target_or_changed_scope_requires_review(self):
        del self.contracts["targets"]["dependency/scoping"]
        with self.assertRaisesRegex(ValueError, "Exact qualification target"):
            self.check()

    def test_changed_acceptance_requires_review(self):
        self.checked[0]["remaining_required_behavior"] = "Less behavior"
        with self.assertRaisesRegex(ValueError, "inventory or acceptance changed"):
            self.check()

    def test_missing_context_cannot_be_certified(self):
        self.review("family/ecore.types", close=True)
        cert = next(iter(self.certificates.values()))
        del cert["verified_contexts"]["semantic"]["boundary"]
        with self.assertRaisesRegex(ValueError, "Exact verified"):
            self.check()

    def test_failing_log_cannot_close(self):
        self.review("family/ecore.types", close=True)
        cert = next(iter(self.certificates.values()))
        name = cert["test_log"]["path"]
        (self.root / name).write_text("test qualification::failed ... FAILED\ntest result: FAILED. 0 passed; 1 failed;\n", encoding="utf-8")
        cert["test_log"]["sha256"] = self.sha(name)
        with self.assertRaises(ValueError):
            self.check()

    def test_closed_without_certificate_fails(self):
        self.review("family/ecore.types")
        self.checked[0]["completion"] = "complete"
        with self.assertRaisesRegex(ValueError, "requires a reviewed qualification certificate"):
            self.check()

    def test_unreviewed_closure_fails(self):
        self.checked[0]["completion"] = "complete"
        with self.assertRaisesRegex(ValueError, "Unreviewed obligations"):
            self.check()

    def test_normative_anchor_and_consumer_boundary_required(self):
        self.review("family/ecore.types")
        row = self.contracts["targets"]["family/ecore.types"]["obligations"][0]
        row["consumers"][0]["implementation"] = "unknown"
        with self.assertRaisesRegex(ValueError, "implementation boundary"):
            self.check()

    def test_sample_pipeline_cannot_omit_validation(self):
        self.qualify_all()
        self.reports["gate-sample_pipeline-report.json"]["samples"][0]["stages"].pop("validate")
        with self.assertRaisesRegex(ValueError, "every required native stage"):
            self.check()

    def test_unlisted_sample_reopens_manifest(self):
        self.qualify_all()
        (self.root / "samples/new.sysml").write_text("package New;", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "omits or invents sources"):
            self.check()

    def test_production_java_blocks_native_qualification(self):
        self.qualify_all()
        self.reports["gate-native_pipeline-report.json"]["runtime_dependencies"].append({"name": "pilot", "language": "java"})
        with self.assertRaisesRegex(ValueError, "cannot contain Java"):
            self.check()

    def test_semantic_mismatch_cannot_claim_match(self):
        self.qualify_all()
        self.reports["gate-pilot_semantics-report.json"]["samples"][0]["observations"][0]["native"] = True
        with self.assertRaisesRegex(ValueError, "mismatch cannot be reported"):
            self.check()

    def test_missing_semantic_slot_rejected(self):
        self.qualify_all()
        self.reports["gate-pilot_semantics-report.json"]["samples"][0]["observations"] = []
        with self.assertRaises(ValueError):
            self.check()

    def test_disagreement_requires_normative_evidence_and_passing_test(self):
        self.qualify_all()
        observation = self.reports["gate-pilot_semantics-report.json"]["samples"][0]["observations"][0]
        observation.update(native=2, outcome="normative_disagreement")
        with self.assertRaisesRegex(ValueError, "explicit evidence anchors"):
            self.check()
        observation.update(normative=[{"path": "normative.txt", "anchor": "Required normative behavior"}], verification_tests=["qualification::gate_positive"])
        self.assertEqual(self.check()["behavioral_denominator_status"], "reviewed")

    def test_incomparable_or_incomplete_timing_rejected(self):
        self.qualify_all()
        report = self.reports["gate-timing_assessment-report.json"]
        report["timing_policy"]["warmup"] = 0
        with self.assertRaisesRegex(ValueError, "Comparable timing"):
            self.check()
        report["timing_policy"]["warmup"] = 1
        report["samples"][0]["seconds"]["pilot"].pop()
        with self.assertRaisesRegex(ValueError, "timing repetitions"):
            self.check()

    def test_timing_acceptance_is_computed_not_a_checkbox(self):
        self.qualify_all()
        row = self.reports["gate-timing_assessment-report.json"]["samples"][0]
        row["seconds"]["native"] = [20, 20, 20]
        with self.assertRaisesRegex(ValueError, "reviewed criterion"):
            self.check()

    def test_missing_native_input_snapshot_cannot_qualify(self):
        self.review("family/ecore.types", close=True)
        self.current_inputs = lambda: {}
        with self.assertRaisesRegex(ValueError, "complete native input snapshot"):
            self.check()

    def test_native_pipeline_requires_every_target(self):
        self.qualify_all()
        self.reports["gate-native_pipeline-report.json"]["verified_targets"].pop()
        with self.assertRaisesRegex(ValueError, "every active implementation target"):
            self.check()

    def test_nonfinite_timing_cannot_qualify(self):
        self.qualify_all()
        self.reports["gate-timing_assessment-report.json"]["samples"][0]["seconds"]["native"][0] = float("nan")
        with self.assertRaisesRegex(ValueError, "finite positive timing"):
            self.check()

    def test_gate_reports_cannot_use_different_manifests(self):
        self.qualify_all()
        entry = self.contracts["targets"]["gate/timing_assessment"]
        manifest = json.loads((self.root / "samples.json").read_text())
        manifest["review_note"] = "different scope review"
        (self.root / "timing-samples.json").write_text(json.dumps(manifest), encoding="utf-8")
        entry["sample_manifest"] = "timing-samples.json"
        entry["inputs"]["timing-samples.json"] = self.sha("timing-samples.json")
        cert = self.certificates[entry["certificate"]]
        cert["inputs"] = copy.deepcopy(entry["inputs"])
        cert["contract_sha256"] = digest({k: v for k, v in entry.items() if k != "certificate"})
        self.reports["gate-timing_assessment-report.json"]["sample_manifest_sha256"] = self.sha("timing-samples.json")
        with self.assertRaisesRegex(ValueError, "same frozen sample manifest"):
            self.check()

    def test_certificate_path_cannot_escape_repository(self):
        self.review("family/ecore.types", close=True)
        self.contracts["targets"]["family/ecore.types"]["certificate"] = "../escape.json"
        with self.assertRaisesRegex(ValueError, "escapes"):
            self.check()

    def test_reviewed_but_open_targets_do_not_count_as_closed(self):
        self.review("family/ecore.types")
        result = self.check()
        self.assertEqual(result["required_reviewed_obligations"], 3)
        self.assertEqual(result["closed_reviewed_obligations"], 0)
        self.assertEqual(result["qualified_targets"], [])

    def test_imported_and_native_subjects_cannot_lack_verification_contexts(self):
        for stage in ("imported", "native"):
            with self.subTest(stage=stage):
                self.review("family/ecore.types")
                row = next(r for r in self.contracts["targets"]["family/ecore.types"]["obligations"] if r["stage"] == stage)
                row["contexts"][0]["subjects"] = ["A"]
                with self.assertRaisesRegex(ValueError, "Every obligation subject requires a verification context"):
                    self.check()

    def test_imported_and_native_subjects_can_use_separate_contexts(self):
        self.review("family/ecore.types")
        for row in self.contracts["targets"]["family/ecore.types"]["obligations"]:
            if row["stage"] in ("imported", "native"):
                first = row["contexts"][0]
                second = copy.deepcopy(first)
                first["subjects"] = ["A"]
                second.update(id="second", subjects=["B"])
                row["contexts"].append(second)
        self.assertEqual(self.check()["reviewed_targets"], 1)
