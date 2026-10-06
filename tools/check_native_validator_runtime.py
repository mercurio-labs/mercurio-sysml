"""Run bounded native validator controls with Java unavailable through the child environment.

The caller builds the supplied Rust test executable first. This does not certify
all product targets, packaging, or absence of hardcoded external process paths.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FILTERS = ("release_2026_08_single_type_controls","translated_type_family","translated_scalar","validation_rules", "annotation_checks", "translated_import_check", "release_2026_08_namespace")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--test-binary", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    binary = args.test_binary.resolve(strict=True)
    env = os.environ.copy()
    removed = []
    for key in list(env):
        upper = key.upper()
        if (upper in {"JAVA_HOME", "JDK_HOME", "JRE_HOME", "CLASSPATH", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS"}
                or "PILOT" in upper):
            removed.append(key)
            del env[key]
    env["PATH"] = ""
    cases = []
    args.out.parent.mkdir(parents=True, exist_ok=True)
    for selected in FILTERS:
        result = subprocess.run([str(binary), selected, "--nocapture"], cwd=ROOT, env=env,
                                capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=180)
        log = result.stdout + result.stderr
        match = re.search(r"test result: ok\. (\d+) passed;", log)
        log_path = args.out.with_name(args.out.stem + "-" + selected + ".log")
        log_path.write_text(log, encoding="utf-8", newline="\n")
        if result.returncode != 0 or not match or int(match.group(1)) == 0:
            raise RuntimeError(f"Native control {selected} failed or selected no tests; see {log_path}")
        cases.append({"filter": selected, "passed": int(match.group(1)), "exit_code": result.returncode,
                      "log": str(log_path.resolve()), "log_sha256": hashlib.sha256(log_path.read_bytes()).hexdigest()})
    report = {
        "schema_version": 1,
        "scope": "Bounded native validator and namespace test executable with empty PATH and Java/Pilot environment removed",
        "test_binary": str(binary), "test_binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "child_environment": {"PATH": "", "removed_variable_names": sorted(removed)},
        "cases": cases,
        "limits": ["Java installations were not removed; absolute-path Java execution is not audited by this runner.",
                   "Rust unit tests include a test-only reference interpreter. Production code is verified separately.",
                   "This does not qualify whole-product packaging or other target platforms."],
    }
    args.out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"Passed {sum(case['passed'] for case in cases)} focused native tests with an isolated Java-free child environment")


if __name__ == "__main__":
    main()
