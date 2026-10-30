#!/usr/bin/env python3
"""Real Linux enforcement gate; unsupported or skipped enforcement is a failure."""

import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import signal
import subprocess
import sys


REQUIRED = {
    "landlock_enforces_read_write_delete_and_outside_confinement",
    "landlock_retains_directory_object_after_root_replacement",
    "landlock_enforces_confinement_during_symlink_replacement",
    "landlock_blocks_network_peer_process_access_and_inherited_descriptors",
    "landlock_exec_failure_never_runs_unconfined",
    "landlock_cancellation_reaps_the_isolated_child_before_scope_join",
}
PORTABLE = "landlock_does_not_guess_endpoint_rules_or_retry_invalid_exec_unconfined"
CALIBRATION = "unconfined_atomic_symlink_replacement_control"
SYMLINK_RACE = "landlock_enforces_confinement_during_symlink_replacement"


class GateFailure(Exception):
    pass


def probe():
    info = {"kernel": platform.release(), "architecture": platform.machine(),
            "system": platform.system()}
    if info["system"] != "Linux" or info["architecture"] not in ("x86_64", "aarch64"):
        raise GateFailure("enforcement gate requires Linux x86_64 or aarch64")
    libc = ctypes.CDLL(None, use_errno=True)
    libc.syscall.restype = ctypes.c_long
    ctypes.set_errno(0)
    # landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION).
    abi = libc.syscall(ctypes.c_long(444), ctypes.c_void_p(), ctypes.c_size_t(0), ctypes.c_uint(1))
    info.update(landlock_abi=abi, probe_errno=ctypes.get_errno(),
                seccomp_mode=libc.prctl(21, 0, 0, 0, 0))
    return info


def test_inventory(text):
    return set(re.findall(r"^([a-zA-Z0-9_:]+): test$", text, re.MULTILINE))


def test_invocation(binary, offline, report):
    if binary is not None:
        binary = binary.resolve(strict=True)
        if not binary.is_file() or not os.access(binary, os.X_OK):
            raise GateFailure("test binary must be an executable regular file")
        with binary.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        report["test_binary"] = {"path": str(binary), "sha256": digest}
        return [str(binary)]
    cargo = ["cargo", "test", "--locked", "-p", "etas_host", "--test", "landlock"]
    if offline:
        cargo.append("--offline")
    return cargo + ["--"]


def verify_execution(text, expected):
    results = re.findall(r"^test ([a-zA-Z0-9_:]+) \.\.\. (ok|FAILED|ignored)(?:.*)$", text, re.MULTILINE)
    for name in expected:
        occurrences = [status for test, status in results if test == name]
        if occurrences != ["ok"]:
            raise GateFailure(f"required test {name}: expected one execution, got {occurrences}")
    return dict(results)


def evidence_lines(text):
    return [line for line in text.splitlines() if line.startswith("ETAS_ISOLATION_")]


def verify_enforcement_evidence(text):
    evidence = evidence_lines(text)
    if not any(line.startswith("ETAS_ISOLATION_EVIDENCE ") for line in evidence):
        raise GateFailure("no activated guarantee evidence was emitted")
    if "ETAS_ISOLATION_CLEANUP cancelled=true reaped=true cleanup=settled" not in evidence:
        raise GateFailure("no isolated cancellation/reap evidence was emitted")


def repeat_symlink_probe(run, repetitions, report):
    report["symlink_repetitions_completed"] = 1  # The full enforcement invocation already ran it.
    for sample in range(1, repetitions):
        text = run(f"symlink-sample-{sample:03}", [
            "--ignored", "--exact", SYMLINK_RACE, "--test-threads=1", "--show-output",
        ])
        verify_execution(text, {SYMLINK_RACE})
        report["symlink_repetitions_completed"] += 1


def command(argv, name, root, destination, report, timeout):
    env = dict(os.environ, CARGO_TERM_COLOR="never")
    path = destination / (name + ".log")
    with path.open("wb") as log:
        process = subprocess.Popen(argv, cwd=root, env=env, stdout=log,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            report["cleanup"] = "unconfirmed_after_gate_timeout"
            raise GateFailure(f"{name} exceeded {timeout}s; enforcement not accepted")
    text = path.read_text(errors="replace")
    evidence = evidence_lines(text)
    report["commands"].append({"name": name, "argv": argv, "exit_code": code, "evidence": evidence})
    report.setdefault("evidence", []).extend(evidence)
    if code:
        raise GateFailure(f"{name} failed with status {code}; see {path.name}")
    return text


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=Path("target/linux-isolation-acceptance"))
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--symlink-repetitions", type=int, default=1,
                        help="total real symlink-race executions including the enforcement suite")
    parser.add_argument("--test-binary", type=Path,
                        help="execute a prebuilt landlock test binary inside the target VM")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("timeout must be positive")
    if args.symlink_repetitions <= 0:
        parser.error("symlink repetitions must be positive")
    root = Path(__file__).resolve().parents[1]
    destination = args.output.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    report = {"status": "failed", "commands": [], "required_tests": sorted(REQUIRED),
              "symlink_repetitions_requested": args.symlink_repetitions,
              "symlink_repetitions_completed": 0,
              "cleanup": "not_started", "kernel": platform.release(), "architecture": platform.machine()}
    try:
        report.update(probe())
        if report["landlock_abi"] < 5:
            raise GateFailure("Landlock ABI v5 is unavailable; no enforcement acceptance")
        invocation = test_invocation(args.test_binary, args.offline, report)
        def run(name, arguments):
            return command(invocation + arguments, name, root, destination, report, args.timeout)
        listed = test_inventory(run("inventory", ["--ignored", "landlock_", "--list"]))
        if not REQUIRED <= listed:
            raise GateFailure(f"required test inventory missing: {sorted(REQUIRED - listed)}")
        # New opt-in cases automatically become part of the gate, not silently filtered out.
        report["required_tests"] = sorted(listed)
        report["cleanup"] = "unconfirmed"
        ordinary = run("ordinary", ["--test-threads=1", "--show-output"])
        report["ordinary_results"] = verify_execution(ordinary, {PORTABLE, CALIBRATION})
        enforced = run("enforcement", ["--ignored", "landlock_", "--test-threads=1", "--show-output"])
        report["test_results"] = verify_execution(enforced, listed)
        verify_enforcement_evidence(enforced)
        repeat_symlink_probe(run, args.symlink_repetitions, report)
        report.update(status="passed", cleanup="settled")
    except (GateFailure, OSError) as error:
        report["error"] = str(error)
    finally:
        (destination / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps(report, indent=2))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
