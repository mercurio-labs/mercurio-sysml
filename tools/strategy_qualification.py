"""Validate bounded semantic obligations; never imply release or family closure."""
import hashlib
import json
from pathlib import Path, PurePosixPath
import re

SCHEMA = "dev.mercurio.strategy-certificate.v1"


def contract_digest(batch):
    contract = {k: v for k, v in batch.items() if k != "certificate"}
    contract["obligations"] = [
        {k: v for k, v in row.items() if k != "status"}
        for row in batch["obligations"]
    ]
    return hashlib.sha256(json.dumps(contract, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")).hexdigest()


def _unique(values, label, nonempty=False):
    if not isinstance(values, list) or any(not isinstance(v, str) or not v.strip() for v in values):
        raise ValueError(f"{label}: expected string list")
    if len(set(values)) != len(values) or (nonempty and not values):
        raise ValueError(f"{label}: duplicate or empty inventory")
    return set(values)


def _path(root, name):
    if not isinstance(name, str) or not name or "\\" in name:
        raise ValueError("evidence path must be repository relative POSIX path")
    relative = PurePosixPath(name)
    if relative.is_absolute() or ".." in relative.parts or ":" in name:
        raise ValueError(f"evidence path escapes repository: {name}")
    result = (root / name).resolve()
    if not result.is_relative_to(root) or not result.is_file():
        raise ValueError(f"missing or escaping evidence: {name}")
    return result


def _hashes(root, entries, label):
    if not isinstance(entries, dict) or not entries:
        raise ValueError(f"{label}: nonempty fingerprints required")
    for name, digest in entries.items():
        if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValueError(f"{label}: invalid digest for {name}")
        actual = hashlib.sha256(_path(root, name).read_bytes()).hexdigest()
        if actual != digest:
            raise ValueError(f"{label}: stale fingerprint for {name}")


def _load(path):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=pairs)


def _passing_tests(log):
    passed = re.findall(r"^test ([\w:]+) \.\.\. ok\s*$", log, re.MULTILINE)
    _unique(passed, "test log", True)
    summaries = re.findall(r"^test result: (\w+)\. (\d+) passed; (\d+) failed;", log, re.MULTILINE)
    if not summaries or any(status != "ok" or int(failed) != 0 for status, _, failed in summaries):
        raise ValueError("test log requires successful summary with zero failures")
    if sum(int(count) for _, count, _ in summaries) != len(passed):
        raise ValueError("test log passed count does not match named outcomes")
    if re.search(r"^test .* \.\.\. FAILED\s*$|^error:", log, re.MULTILINE):
        raise ValueError("test log contains failures")
    return set(passed)


def validate_strategy_batches(plan, root):
    """Return required/closed counts after validating all declared obligations."""
    root = Path(root).resolve()
    batches = plan.get("strategy_batches", [])
    if not isinstance(batches, list):
        raise ValueError("strategy_batches must be a list")
    _unique([b.get("id") for b in batches], "batch IDs")
    result = {"required": 0, "closed": 0, "batches": []}
    for batch in batches:
        label = batch["id"]
        if not isinstance(batch.get("scope"), str) or not batch["scope"].strip():
            raise ValueError(f"{label}: explicit scope required")
        _unique(batch.get("bindings"), f"{label} bindings", True)
        rows = batch.get("obligations")
        if not isinstance(rows, list):
            raise ValueError(f"{label}: obligations must be a list")
        ids = _unique([row.get("id") for row in rows], f"{label} obligations", True)
        closed = set()
        for row in rows:
            if not isinstance(row.get("acceptance"), str) or not row["acceptance"].strip():
                raise ValueError(f"{label}: acceptance required")
            deps = _unique(row.get("dependencies"), "dependencies")
            if not deps <= ids or row["id"] in deps:
                raise ValueError(f"{label}: invalid dependency")
            tests = _unique(row.get("tests"), "tests", True)
            if any(not re.fullmatch(r"\w+(?:::\w+)+", t) for t in tests):
                raise ValueError(f"{label}: fully qualified test names required")
            if row.get("status") not in ("open", "closed"):
                raise ValueError(f"{label}: invalid status")
            if row["status"] == "closed":
                closed.add(row["id"])
        # Reject circular acceptance dependencies, including cycles among closed rows.
        pending = {r["id"]: set(r["dependencies"]) for r in rows}
        resolved = set()
        while pending:
            ready = {name for name, deps in pending.items() if deps <= resolved}
            if not ready:
                raise ValueError(f"{label}: cyclic dependencies")
            resolved.update(ready)
            pending = {name: deps for name, deps in pending.items() if name not in ready}
        for row in rows:
            if row["id"] in closed and not set(row["dependencies"]) <= closed:
                raise ValueError(f"{label}: closed obligation has open dependency")
        _hashes(root, batch.get("inputs"), "inputs")
        certificate = batch.get("certificate")
        if closed and not certificate:
            raise ValueError(f"{label}: closed obligations require certificate")
        if certificate:
            cert = _load(_path(root, certificate))
            if cert.get("schema") != SCHEMA or cert.get("batch_id") != label:
                raise ValueError(f"{label}: certificate identity mismatch")
            if cert.get("contract_sha256") != contract_digest(batch):
                raise ValueError(f"{label}: contract mismatch")
            if cert.get("inputs") != batch["inputs"]:
                raise ValueError(f"{label}: certificate input inventory mismatch")
            _hashes(root, cert.get("observations"), "observations")
            claimed = _unique(cert.get("closed_obligations"), "closed obligations", True)
            if claimed != closed:
                raise ValueError(f"{label}: exact closed obligation set required")
            test_log = cert.get("test_log", {})
            _hashes(root, {test_log.get("path"): test_log.get("sha256")}, "test log")
            actual = _passing_tests(_path(root, test_log["path"]).read_text(encoding="utf-8-sig"))
            passed = _unique(cert.get("passed_tests"), "passed tests", True)
            if passed != actual:
                raise ValueError(f"{label}: certificate passing test inventory mismatch")
            required = {test for row in rows if row["id"] in closed for test in row["tests"]}
            if not required <= passed:
                raise ValueError(f"{label}: missing passing obligation tests")
        item = {"id": label, "required": len(rows), "closed": len(closed), "closed_obligations": sorted(closed)}
        result["batches"].append(item)
        result["required"] += len(rows)
        result["closed"] += len(closed)
    return result
