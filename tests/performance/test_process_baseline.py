#!/usr/bin/env python3
"""Exercise cleanup failures with Docker doubles; never launch a service."""

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "process_baseline", Path(__file__).with_name("process-baseline.py")
)
baseline_module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(baseline_module)

OWNED_ID = "a" * 64
FOREIGN_ID = "f" * 64


class DockerDouble:
    """Simulate external Docker outcomes and keep foreign resources visible."""

    def __init__(self):
        self.containers = {FOREIGN_ID: "another-operation"}
        self.removal_attempts = []
        self.run_error = None
        self.log_error = None
        self.log_returncode = 0
        self.remove_failures = set()
        self.copy_error = None

    @staticmethod
    def result(stdout="", stderr="", returncode=0):
        return SimpleNamespace(stdout=stdout, stderr=stderr, returncode=returncode)

    def __call__(self, *args, **_kwargs):
        if args[:2] == ("image", "inspect"):
            return self.result(json.dumps([{"Id": "frozen-image", "Created": "frozen-date"}]))
        if args[0] in {"run", "create"}:
            owner = args[args.index("--label") + 1].split("=", 1)[1]
            self.containers[OWNED_ID] = owner
            if self.run_error is not None:
                raise self.run_error
            return self.result(OWNED_ID + "\n")
        if args[0] == "inspect":
            # Fail before any network/process/client sampling begins.
            raise RuntimeError("external setup failure")
        if args[0] == "cp":
            if self.copy_error is not None:
                raise self.copy_error
            raise AssertionError("unexpected successful extraction in a failure test")
        if args[0] == "logs":
            if self.log_error is not None:
                raise self.log_error
            return self.result("", "external log failure", self.log_returncode)
        if args[:2] == ("container", "ls"):
            prefix = f"label={baseline_module.OWNERSHIP_LABEL}="
            selector = args[args.index("--filter") + 1]
            if not selector.startswith(prefix):
                raise AssertionError("container discovery must select the operation's owner")
            owner = selector[len(prefix):]
            return self.result("\n".join(sorted(
                container_id for container_id, label in self.containers.items() if label == owner
            )))
        if args[0] == "rm":
            container_id = args[2]
            self.removal_attempts.append(container_id)
            if container_id in self.remove_failures:
                raise RuntimeError("external removal failure")
            del self.containers[container_id]
            return self.result(container_id)
        raise AssertionError(f"unexpected Docker operation: {args}")


class CleanupTests(unittest.TestCase):
    """Validate owned resource cleanup across concrete external failures."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="ferrumc-cleanup-test-")
        self.addCleanup(self.directory.cleanup)
        self.artifacts = Path(self.directory.name)
        self.docker = DockerDouble()
        self.baseline = baseline_module.Baseline.__new__(baseline_module.Baseline)
        self.baseline.artifacts = self.artifacts
        self.baseline.args = SimpleNamespace(view_distance=4)
        self.baseline.image = "frozen-image"
        self.baseline.result = {"runs": []}
        self.baseline.docker = self.docker

    def run_failing_setup(self, expected_error="external setup failure"):
        with self.assertRaisesRegex((RuntimeError, subprocess.TimeoutExpired), expected_error):
            self.baseline.one_run("native", 1)
        record = self.baseline.result["runs"][0]
        persisted = json.loads((self.artifacts / "results.json").read_text())
        self.assertEqual(persisted["runs"][0], record)
        return record

    def assert_only_foreign_container_remains(self):
        self.assertEqual(self.docker.containers, {FOREIGN_ID: "another-operation"})
        self.assertEqual(self.docker.removal_attempts, [OWNED_ID])

    def test_log_timeout_does_not_skip_owned_container_removal(self):
        self.docker.log_error = subprocess.TimeoutExpired("external logs", 30)
        record = self.run_failing_setup()
        self.assertIn("log_collection_error", record)
        self.assertTrue(record["cleanup_verified"])
        self.assert_only_foreign_container_remains()

    def test_log_failure_preserves_existing_diagnostics_and_removes_container(self):
        run_artifacts = self.artifacts / "round-1-native"
        run_artifacts.mkdir()
        log = run_artifacts / "server.log"
        log.write_text("previous diagnostic log")
        self.docker.log_returncode = 1
        record = self.run_failing_setup()
        self.assertIn("log_collection_error", record)
        self.assertEqual(log.read_text(), "previous diagnostic log")
        self.assertTrue(record["cleanup_verified"])
        self.assert_only_foreign_container_remains()

    def test_timed_out_run_recovers_owned_container_without_returned_id(self):
        self.docker.run_error = subprocess.TimeoutExpired("external run", 30)
        record = self.run_failing_setup("timed out")
        self.assertTrue(record["cleanup_verified"])
        self.assert_only_foreign_container_remains()

    def test_failed_removal_is_recorded_and_never_reports_verified_cleanup(self):
        self.docker.remove_failures.add(OWNED_ID)
        record = self.run_failing_setup("remains after cleanup")
        self.assertFalse(record["cleanup_verified"])
        self.assertIn("container_cleanup_error", record)
        self.assertIn(OWNED_ID, self.docker.containers)
        self.assertEqual(self.docker.removal_attempts, [OWNED_ID])
        self.assertEqual(record["error"], "external setup failure")

    def test_failed_removal_still_attempts_other_owned_containers(self):
        second_id = "b" * 64
        self.docker.containers.update({OWNED_ID: "our-operation", second_id: "our-operation"})
        self.docker.remove_failures.add(OWNED_ID)
        with self.assertRaisesRegex(RuntimeError, "remaining IDs"):
            self.baseline.remove_owned_containers("our-operation")
        self.assertEqual(self.docker.removal_attempts, [OWNED_ID, second_id])
        self.assertNotIn(second_id, self.docker.containers)
        self.assertEqual(self.docker.containers[FOREIGN_ID], "another-operation")

    def test_missing_owned_container_never_removes_foreign_resources(self):
        self.baseline.remove_owned_containers("missing-operation")
        self.assertEqual(self.docker.removal_attempts, [])
        self.assertEqual(self.docker.containers, {FOREIGN_ID: "another-operation"})

    def test_failed_artifact_extraction_removes_only_its_owned_container(self):
        fixture = self.artifacts / "fixture.jar"
        fixture.write_bytes(b"local test fixture")
        extraction = self.artifacts / "extraction"
        extraction.mkdir()
        args = SimpleNamespace(docker_command="docker", namespace_command="nsenter",
                               image="frozen-image", fixture=fixture)
        self.docker.copy_error = RuntimeError("external copy failure")
        with patch.object(baseline_module.tempfile, "mkdtemp", return_value=str(extraction)), \
                patch.object(baseline_module.Baseline, "docker", self.docker):
            with self.assertRaisesRegex(RuntimeError, "external copy failure"):
                baseline_module.Baseline(args)
        self.assert_only_foreign_container_remains()


if __name__ == "__main__":
    unittest.main()
