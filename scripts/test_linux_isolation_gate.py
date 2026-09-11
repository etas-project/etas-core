import unittest
import hashlib
from pathlib import Path
import tempfile

from linux_isolation_gate import GateFailure, REQUIRED, test_inventory, test_invocation, verify_execution


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


if __name__ == "__main__":
    unittest.main()
