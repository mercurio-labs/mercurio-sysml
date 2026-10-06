"""Evidence-backed closure of reviewed families, dependencies and release gates.

A reviewed contract defines the behavioral denominator. Certificates validate
that contract, its exact inputs and its named passing evidence; they do not infer
semantic support from imported identities or passing test totals.
"""
import hashlib
import json
from pathlib import Path
from strategy_qualification import _hashes, _load, _path, _passing_tests, _unique

SCHEMA = "dev.mercurio.qualification-contracts.v1"
CERTIFICATE_SCHEMA = "dev.mercurio.qualification-certificate.v1"
STAGES = {"imported", "native", "semantic"}
GATES = {"obligation_inventory", "native_pipeline", "sample_pipeline", "pilot_semantics", "timing_assessment"}


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()


def targets(plan, checked):
    result = {}
    for row in checked:
        key = "family/" + row["id"]
        result[key] = {"kind": "family", "subjects": sorted(set(row["upstream_ids"])),
                       "active": row["applicability"] == "used_by_pin",
                       "acceptance": row["remaining_required_behavior"],
                       "dependencies": ["dependency/" + d for d in row["semantic_dependencies"]],
                       "closed": row["completion"] == "complete"}
    for name, row in plan["dependencies"].items():
        result["dependency/" + name] = {"kind": "dependency", "subjects": [name], "active": True,
            "acceptance": row["remaining"], "dependencies": [], "closed": row["status"] == "complete"}
    families = [k for k, v in result.items() if v["active"]]
    gate_dependencies = {
        "obligation_inventory": [],
        "native_pipeline": [*families, "gate/obligation_inventory"],
        "sample_pipeline": ["gate/native_pipeline"],
        "pilot_semantics": ["gate/sample_pipeline"],
        "timing_assessment": ["gate/pilot_semantics"],
    }
    for gate in plan["release_gates"]:
        result["gate/" + gate["id"]] = {"kind": "gate", "subjects": [gate["id"]], "active": True,
            "acceptance": gate["acceptance"], "dependencies": gate_dependencies[gate["id"]],
            "closed": gate["status"] == "qualified"}
    return result


def target_identity(target):
    # Remaining-behavior prose stays an acceptance requirement even after closure.
    result = {k: v for k, v in target.items() if k not in ("closed", "subjects")}
    result["subject_count"] = len(target["subjects"])
    result["subject_sha256"] = digest(target["subjects"])
    return result


def _anchors(root, entries, inputs, label, modes=False):
    if not isinstance(entries, list) or not entries:
        raise ValueError(label + ": explicit evidence anchors required")
    for entry in entries:
        path, anchor = entry.get("path"), entry.get("anchor")
        if path not in inputs or not isinstance(anchor, str) or not anchor.strip():
            raise ValueError(label + ": fingerprinted anchor required")
        if anchor not in _path(root, path).read_text(encoding="utf-8"):
            raise ValueError(label + ": missing anchor")
        if modes and entry.get("implementation") not in ("generated", "handwritten_rust", "mixed"):
            raise ValueError(label + ": explicit implementation boundary required")


def validate_contract(entry, target, root):
    if entry.get("identity") != target_identity(target):
        raise ValueError("Qualification target inventory or acceptance changed; review required")
    if entry.get("review_status") not in ("unreviewed", "reviewed"):
        raise ValueError("Invalid qualification review state")
    if entry["review_status"] == "unreviewed":
        if target["closed"] or entry.get("certificate"):
            raise ValueError("Unreviewed obligations cannot close")
        return []
    if not target["active"]:
        raise ValueError("Unused source families cannot be qualified")
    inputs = entry.get("inputs")
    _hashes(root, inputs, "reviewed qualification inputs")
    _anchors(root, entry.get("review"), inputs, "review")
    rows = entry.get("obligations")
    if not isinstance(rows, list):
        raise ValueError("Reviewed obligations required")
    _unique([row.get("id") for row in rows], "qualification obligation IDs", True)
    covered = {stage: set() for stage in (STAGES if target["kind"] != "gate" else {"gate"})}
    for row in rows:
        stage = row.get("stage")
        if stage not in covered or not str(row.get("acceptance", "")).strip():
            raise ValueError("Invalid qualification stage or acceptance")
        subjects = _unique(row.get("subjects"), "obligation subjects", True)
        if not subjects <= set(target["subjects"]):
            raise ValueError("Unknown obligation subjects")
        covered[stage].update(subjects)
        _anchors(root, row.get("normative"), inputs, "normative evidence")
        _anchors(root, row.get("consumers"), inputs, "native consumer", True)
        contexts = row.get("contexts")
        if not isinstance(contexts, list):
            raise ValueError("Explicit verification contexts required")
        _unique([c.get("id") for c in contexts], "context IDs", True)
        if stage == "semantic" and {c.get("polarity") for c in contexts} != {"positive", "negative", "boundary"}:
            raise ValueError("Semantic verification requires positive, negative and boundary contexts")
        exercised_subjects = set()
        context_subjects = {polarity: set() for polarity in ("positive", "negative", "boundary")}
        for context in contexts:
            selected = _unique(context.get("subjects"), "context subjects", True)
            if not selected <= subjects:
                raise ValueError("Context claims subjects outside its obligation")
            exercised_subjects.update(selected)
            if context.get("polarity") in context_subjects:
                context_subjects[context["polarity"]].update(selected)
            tests = _unique(context.get("tests"), "context tests", True)
            if any("::" not in test for test in tests):
                raise ValueError("Fully qualified test names required")
            _unique(context.get("observations"), "context observations", True)
            if not set(context["observations"]) <= set(inputs):
                raise ValueError("Observation inputs must be fingerprinted")
        if exercised_subjects != subjects:
            raise ValueError("Every obligation subject requires a verification context")
        if stage == "semantic" and any(selected != subjects for selected in context_subjects.values()):
            raise ValueError("Every semantic subject requires all three context polarities")
    if any(subjects != set(target["subjects"]) for subjects in covered.values()):
        raise ValueError("Imported, native and semantic obligations must cover every subject")
    return rows


PIPELINE_STAGES = {"parse", "construct", "link", "validate", "persist"}


def _sample_manifest(entry, root):
    path = entry.get("sample_manifest")
    if path not in entry["inputs"]:
        raise ValueError("Frozen sample manifest must be a reviewed input")
    manifest = _load(_path(root, path))
    if manifest.get("schema") != "dev.mercurio.sample-manifest.v1":
        raise ValueError("Invalid frozen sample manifest")
    samples = manifest.get("samples")
    _hashes(root, samples, "complete sample inputs")
    if any(not name.endswith((".sysml", ".kerml")) for name in samples):
        raise ValueError("Sample manifest must contain language sources")
    if any(entry["inputs"].get(name) != value for name, value in samples.items()):
        raise ValueError("Every frozen sample must be a reviewed input")
    # Completeness is reviewed against declared pinned source roots. A manifest
    # with a silently omitted source, or a source added later, cannot qualify.
    roots = _unique(manifest.get("source_roots"), "sample source roots", True)
    discovered = set()
    for directory in roots:
        relative = Path(directory)
        resolved = (root / relative).resolve()
        if relative.is_absolute() or ".." in relative.parts or not resolved.is_relative_to(root) or not resolved.is_dir():
            raise ValueError("Sample source root escapes or is absent")
        for source in resolved.rglob("*"):
            if source.is_file() and source.suffix in (".sysml", ".kerml"):
                discovered.add(source.relative_to(root).as_posix())
    if discovered != set(samples):
        raise ValueError("Frozen sample manifest omits or invents sources")
    return samples


def validate_gate_report(name, entry, certificate, inventory, root):
    if name == "gate/obligation_inventory":
        return  # All active contracts must be reviewed; checked globally below.
    descriptor = certificate.get("report", {})
    _hashes(root, {descriptor.get("path"): descriptor.get("sha256")}, "qualification report")
    report = _load(_path(root, descriptor["path"]))
    if report.get("schema") != "dev.mercurio.release-qualification-report.v1" or report.get("gate") != name:
        raise ValueError("Qualification report identity mismatch")
    if report.get("native_input_sha256") != certificate["native_input_sha256"] or report.get("failures") != []:
        raise ValueError("Qualification report has stale inputs or failures")
    if name == "gate/native_pipeline":
        expected = {key for key, target in inventory.items() if target["active"] and target["kind"] != "gate"}
        if _unique(report.get("verified_targets"), "native pipeline targets", True) != expected:
            raise ValueError("Native pipeline requires every active implementation target")
        if _unique(report.get("stages"), "native pipeline stages", True) != PIPELINE_STAGES:
            raise ValueError("Native pipeline omits a required stage")
        dependencies = report.get("runtime_dependencies")
        if not isinstance(dependencies, list) or not dependencies:
            raise ValueError("Production runtime dependency inventory required")
        _unique([d.get("name") for d in dependencies], "runtime dependencies", True)
        if any(d.get("language") not in ("rust", "c", "c++", "system") for d in dependencies):
            raise ValueError("Production qualification cannot contain Java or other unassessed runtime dependencies")
        if not any(d["language"] == "rust" for d in dependencies):
            raise ValueError("Native Rust runtime must be present")
        return
    samples = _sample_manifest(entry, root)
    if report.get("sample_manifest_sha256") != entry["inputs"][entry["sample_manifest"]]:
        raise ValueError("Qualification report uses different sample inputs")
    rows = report.get("samples")
    if not isinstance(rows, list) or _unique([r.get("sample") for r in rows], "reported samples", True) != set(samples):
        raise ValueError("Qualification report requires every frozen sample exactly once")
    for row in rows:
        if row.get("source_sha256") != samples[row["sample"]]:
            raise ValueError("Qualification sample input mismatch")
    if name == "gate/sample_pipeline":
        for row in rows:
            if row.get("stages") != {stage: "passed" for stage in PIPELINE_STAGES}:
                raise ValueError("Every sample must pass every required native stage")
        return
    if name == "gate/pilot_semantics":
        path = entry.get("observation_inventory")
        if path not in entry["inputs"]:
            raise ValueError("Reviewed semantic observation inventory required")
        observations = _load(_path(root, path))
        if set(observations) != set(samples):
            raise ValueError("Semantic inventory must cover every sample")
        for row in rows:
            expected = _unique(observations[row["sample"]], "semantic observations", True)
            actual = row.get("observations")
            if not isinstance(actual, list) or _unique([o.get("id") for o in actual], "reported observations", True) != expected:
                raise ValueError("Missing or extra semantic observations")
            for observation in actual:
                if "native" not in observation or "pilot" not in observation:
                    raise ValueError("Both independent semantic values required")
                if observation.get("outcome") == "match":
                    if digest(observation["native"]) != digest(observation["pilot"]):
                        raise ValueError("Semantic mismatch cannot be reported as matching")
                elif observation.get("outcome") == "normative_disagreement":
                    if digest(observation["native"]) == digest(observation["pilot"]):
                        raise ValueError("Normative disagreement must identify an actual difference")
                    _anchors(root, observation.get("normative"), entry["inputs"], "normative disagreement")
                    if not _unique(observation.get("verification_tests"), "disagreement tests", True) <= set(certificate["passed_tests"]):
                        raise ValueError("Normative disagreement lacks passing verification")
                else:
                    raise ValueError("Unresolved semantic difference")
        return
    if name == "gate/timing_assessment":
        import math
        import statistics
        policy = entry.get("timing_policy", {})
        repetitions, warmup = policy.get("repetitions"), policy.get("warmup")
        ceiling = policy.get("max_native_to_pilot_ratio")
        if type(repetitions) is not int or repetitions < 3 or type(warmup) is not int or warmup < 0:
            raise ValueError("Reviewed timing repetition/warmup policy required")
        if type(ceiling) not in (int, float) or not math.isfinite(ceiling) or ceiling <= 0:
            raise ValueError("Reviewed timing acceptance criterion required")
        if not isinstance(policy.get("pipeline_boundary"), str) or not policy["pipeline_boundary"].strip():
            raise ValueError("Reviewed timing pipeline boundary required")
        if report.get("timing_policy") != policy or set(report.get("toolchains", {})) != {"native", "pilot"} or not all(report["toolchains"].values()):
            raise ValueError("Comparable timing policy and toolchains required")
        for row in rows:
            timings = row.get("seconds", {})
            if set(timings) != {"native", "pilot"}:
                raise ValueError("Paired timing required")
            for values in timings.values():
                if not isinstance(values, list) or len(values) != repetitions or any(type(v) not in (int, float) or not math.isfinite(v) or v <= 0 for v in values):
                    raise ValueError("Complete finite positive timing repetitions required")
            ratio = statistics.median(timings["native"]) / statistics.median(timings["pilot"])
            if type(row.get("median_ratio")) not in (int, float) or ratio > ceiling or not math.isclose(row["median_ratio"], ratio, rel_tol=1e-9):
                raise ValueError("Timing does not satisfy or accurately report the reviewed criterion")
        return
    raise ValueError("Unknown release qualification gate")


def validate_certificates(plan, checked, root, native_inputs=None):
    from pathlib import Path
    root = Path(root).resolve()
    inventory = targets(plan, checked)
    contract_path = plan.get("qualification_contracts")
    contracts = _load(_path(root, contract_path))
    if contracts.get("schema") != SCHEMA or set(contracts.get("targets", {})) != set(inventory):
        raise ValueError("Exact qualification target inventory required")
    required, closed = 0, 0
    reviewed, qualified = set(), set()
    for name, target in inventory.items():
        entry = contracts["targets"][name]
        obligations = validate_contract(entry, target, root)
        if entry["review_status"] == "reviewed":
            reviewed.add(name)
            required += len(obligations)
        certificate_path = entry.get("certificate")
        if target["closed"] and not certificate_path:
            raise ValueError("Closure requires a reviewed qualification certificate")
        if not certificate_path:
            continue
        certificate = _load(_path(root, certificate_path))
        if callable(native_inputs):
            native_inputs = native_inputs()
        if not native_inputs or certificate.get("native_input_sha256") != digest(native_inputs):
            raise ValueError("Qualification requires the current complete native input snapshot")
        contract = {k: v for k, v in entry.items() if k != "certificate"}
        if certificate.get("schema") != CERTIFICATE_SCHEMA or certificate.get("target") != name:
            raise ValueError("Qualification certificate identity mismatch")
        if certificate.get("contract_sha256") != digest(contract) or certificate.get("inputs") != entry["inputs"]:
            raise ValueError("Qualification certificate contract mismatch")
        # Both inventories are exact: a certificate cannot omit a context or
        # silently reuse a certificate for a smaller behavioral scope.
        expected = {r["id"]: {c["id"]: sorted(c["subjects"]) for c in r["contexts"]} for r in obligations}
        if certificate.get("verified_contexts") != expected:
            raise ValueError("Exact verified obligation/context inventory required")
        log = certificate.get("test_log", {})
        _hashes(root, {log.get("path"): log.get("sha256")}, "qualification test log")
        passed = _passing_tests(_path(root, log["path"]).read_text(encoding="utf-8-sig"))
        if _unique(certificate.get("passed_tests"), "qualification passing tests", True) != passed:
            raise ValueError("Qualification passing test inventory mismatch")
        tests = {t for r in obligations for c in r["contexts"] for t in c["tests"]}
        if not tests <= passed:
            raise ValueError("Missing passing qualification context tests")
        if not target["closed"]:
            raise ValueError("Qualification certificate cannot disagree with open status")
        if target["kind"] == "gate":
            validate_gate_report(name, entry, certificate, inventory, root)
        qualified.add(name)
        closed += len(obligations)
    for name in qualified:
        if not set(inventory[name]["dependencies"]) <= qualified:
            raise ValueError("Qualified target has open semantic dependencies")
    manifests = {contracts["targets"][name]["inputs"][contracts["targets"][name]["sample_manifest"]]
                 for name in qualified & {"gate/sample_pipeline", "gate/pilot_semantics", "gate/timing_assessment"}}
    if len(manifests) > 1:
        raise ValueError("Release gates must use the same frozen sample manifest")
    active = {name for name, target in inventory.items() if target["active"]}
    if "gate/obligation_inventory" in qualified and not active <= reviewed:
        raise ValueError("Inventory gate requires every active behavioral contract to be reviewed")
    return {"reviewed_targets": len(reviewed), "required_targets": len(active),
            "required_reviewed_obligations": required, "closed_reviewed_obligations": closed,
            "behavioral_denominator_status": "reviewed" if "gate/obligation_inventory" in qualified else "unreviewed",
            "qualified_targets": sorted(qualified), "contract_path": contract_path,
            "contract_sha256": hashlib.sha256(_path(root, contract_path).read_bytes()).hexdigest()}
