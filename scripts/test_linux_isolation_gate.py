import unittest
import hashlib
from pathlib import Path
import tempfile
import sys

from linux_isolation_gate import (CALIBRATION, PORTABLE, GateFailure, REQUIRED, SYMLINK_RACE, command,
                                  repeat_symlink_probe, test_inventory,
                                  test_invocation, verify_enforcement_evidence,
                                  verify_execution)


class InventoryTests(unittest.TestCase):
    def test_inventory_requires_real_test_entries(self):
        self.assertEqual(test_inventory("running 0 tests\n"), set())
        name = next(iter(REQUIRED))
        self.assertEqual(test_inventory(f"{name}: test\n"), {name})

    def test_zero_filtered_ignored_failed_and_duplicate_are_rejected(self):
        name = next(iter(REQUIRED))
        for output in ["running 0 tests\n", f"test {name} ... ignored\n",
                       f"test {name} ... FAILED\n", f"test {name} ... ok\ntest {name} ... ok\n"]:
            with self.subTest(output=output), self.assertRaises(GateFailure):
                verify_execution(output, {name})

    def test_every_required_test_must_execute(self):
        output = "".join(f"test {name} ... ok\n" for name in sorted(REQUIRED))
        self.assertEqual(set(verify_execution(output, REQUIRED)), REQUIRED)
        with self.assertRaises(GateFailure):
            verify_execution(output, REQUIRED | {"landlock_new_required_case"})

    def test_cargo_invocation_retains_locked_and_offline_build(self):
        self.assertEqual(test_invocation(None, True, {}), [
            "cargo", "test", "--locked", "-p", "etas_host", "--test", "landlock", "--offline", "--",
        ])

    def test_prebuilt_binary_is_identified_and_uses_the_same_inventory_checks(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / "landlock"
            binary.write_bytes(b"test binary fixture")
            report = {}
            with self.assertRaises(GateFailure):
                test_invocation(binary, True, report)
            binary.chmod(0o700)
            self.assertEqual(test_invocation(binary, True, report), [str(binary.resolve())])
            self.assertEqual(report["test_binary"]["sha256"],
                             hashlib.sha256(binary.read_bytes()).hexdigest())
            with self.assertRaises(GateFailure):
                verify_execution("running 0 tests\n", REQUIRED)

    def test_missing_prebuilt_binary_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(FileNotFoundError):
                test_invocation(Path(directory) / "missing", False, {})

    def test_repetitions_run_the_exact_real_probe_and_count_only_verified_samples(self):
        calls = []
        def run(name, arguments):
            calls.append((name, arguments))
            return f"test {SYMLINK_RACE} ... ok\n"
        report = {}
        repeat_symlink_probe(run, 4, report)
        self.assertEqual(report["symlink_repetitions_completed"], 4)
        self.assertEqual([name for name, _ in calls], [
            "symlink-sample-001", "symlink-sample-002", "symlink-sample-003",
        ])
        for _, arguments in calls:
            self.assertEqual(arguments, [
                "--ignored", "--exact", SYMLINK_RACE, "--test-threads=1", "--show-output",
            ])

    def test_repetition_stops_on_ignored_failed_duplicate_or_missing_test(self):
        for status in ["ignored", "FAILED", "missing", "duplicate"]:
            report = {}
            def run(_name, _arguments):
                if status == "missing":
                    return "running 0 tests\n"
                if status == "duplicate":
                    return f"test {SYMLINK_RACE} ... ok\n" * 2
                return f"test {SYMLINK_RACE} ... {status}\n"
            with self.subTest(status=status), self.assertRaises(GateFailure):
                repeat_symlink_probe(run, 4, report)
            self.assertEqual(report["symlink_repetitions_completed"], 1)

    def test_failed_command_keeps_observations_without_becoming_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = {"commands": []}
            with self.assertRaises(GateFailure):
                command([sys.executable, "-c", "print('ETAS_ISOLATION_FAILURE errno=2'); raise SystemExit(1)"],
                        "failed-probe", root, root, report, 5)
            self.assertEqual(report["commands"][0]["exit_code"], 1)
            self.assertEqual(report["evidence"], ["ETAS_ISOLATION_FAILURE errno=2"])
            self.assertEqual(report["commands"][0]["evidence"], report["evidence"])

    def test_enforcement_requires_its_own_activation_and_cleanup_evidence(self):
        activated = "ETAS_ISOLATION_EVIDENCE backend=Landlock cleanup=settled\n"
        cleaned = "ETAS_ISOLATION_CLEANUP cancelled=true reaped=true cleanup=settled\n"
        for text in ["", activated, cleaned]:
            with self.subTest(text=text), self.assertRaises(GateFailure):
                verify_enforcement_evidence(text)
        verify_enforcement_evidence(activated + cleaned)

    def test_ordinary_calibration_cannot_be_omitted_or_ignored(self):
        portable = f"test {PORTABLE} ... ok\n"
        for control in ["", f"test {CALIBRATION} ... ignored\n"]:
            with self.subTest(control=control), self.assertRaises(GateFailure):
                verify_execution(portable + control, {PORTABLE, CALIBRATION})
        verify_execution(portable + f"test {CALIBRATION} ... ok\n", {PORTABLE, CALIBRATION})


if __name__ == "__main__":
    unittest.main()
