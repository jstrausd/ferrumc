#!/usr/bin/env python3
"""Negative provenance/hash/producer checks, including actual helper refusals.

Usage: test_normal_crafting_producer.py ORIGINAL_CACHE JAVA_HOME NORMAL_EXPORT
"""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile

sys.dont_write_bytecode = True
SOURCE = Path(__file__).with_name("generate_normal_crafting.py")
SPEC = importlib.util.spec_from_file_location("crafting_producer", SOURCE)
GEN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GEN)
GEN.require(len(sys.argv) == 4, "expected ORIGINAL_CACHE JAVA_HOME NORMAL_EXPORT")
CACHE, JAVA_HOME, EXPORT = (Path(v).resolve() for v in sys.argv[1:])
sys.argv = sys.argv[:1]
FIXTURE = SOURCE.resolve().parents[5] / "docs/fixtures/minecraft-1.21.8-original/recipes.json"


class ProducerGuards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.records = [json.loads(line) for line in (EXPORT / "normal_crafting_1_21_8.jsonl").read_text().splitlines()]
        cls.bundle = CACHE / "server.jar"
        cls.inner = CACHE / "versions/1.21.8/server-1.21.8.jar"
        GEN.verify(cls.bundle, GEN.PINS["bundle"], "official bundle")
        GEN.verify(cls.inner, GEN.PINS["inner"], "official inner")
        GEN.verify(FIXTURE, GEN.PINS["recipes"], "original recipes")
        classpath = [cls.inner]
        with zipfile.ZipFile(cls.bundle) as archive:
            for line in archive.read("META-INF/libraries.list").decode().splitlines():
                digest, coordinate, relative = line.split("\t")
                path = CACHE / "libraries" / relative
                GEN.verify(path, digest, coordinate); classpath.append(path)
        GEN.require(len(classpath) == 44, "original classpath mismatch")
        GEN.reject_shadow(classpath)
        cls.cp = os.pathsep.join(str(p) for p in classpath)
        cls.classes = tempfile.TemporaryDirectory(prefix="crafting-negative-classes-", dir=EXPORT)
        subprocess.run([str(JAVA_HOME / "bin/javac"), "-proc:none", "-encoding", "UTF-8", "-cp", cls.cp, "-d", cls.classes.name, str(SOURCE.with_name("normal_crafting_oracle.java"))], check=True)

    @classmethod
    def tearDownClass(cls):
        cls.classes.cleanup()

    def mutate(self, kind, **updates):
        records = list(self.records)
        index = next(i for i, record in enumerate(records) if record["kind"] == kind)
        records[index] = dict(records[index], **updates)
        return records

    def test_actual_projection_matches_generated_artifacts(self):
        rust, tsv, census = GEN.producer(self.records, FIXTURE)
        self.assertEqual(rust, (EXPORT / "normal_recipes_generated.rs").read_bytes())
        self.assertEqual(tsv, (EXPORT / "normal_crafting_1_21_8.tsv").read_bytes())
        self.assertEqual(census["recipes"], 968)

    def test_hash_changes_and_symlink_inputs_are_refused(self):
        with tempfile.TemporaryDirectory(dir=EXPORT) as directory:
            path = Path(directory) / "wrong-input"
            path.write_bytes(b"wrong original")
            for label, digest in GEN.PINS.items():
                with self.subTest(label=label), self.assertRaisesRegex(ValueError, "SHA256 mismatch"):
                    GEN.verify(path, digest, label)
            link = Path(directory) / "linked-input"; link.symlink_to(FIXTURE)
            with self.assertRaisesRegex(ValueError, "invalid .* file"):
                GEN.verify(link, GEN.PINS["recipes"], "recipes")

    def test_helper_classpath_shadow_is_refused(self):
        with tempfile.TemporaryDirectory(dir=EXPORT) as directory:
            shadow = Path(directory) / "shadow.jar"
            with zipfile.ZipFile(shadow, "w") as archive:
                archive.writestr(GEN.HELPER, b"not a trusted helper")
            with self.assertRaisesRegex(ValueError, "helper classpath shadow"):
                GEN.reject_shadow([shadow])

    def test_missing_recipe_invalid_grid_and_duplicate_case_are_refused(self):
        records = list(self.records); records.remove(next(r for r in records if r["kind"] == "recipe"))
        with self.assertRaisesRegex(ValueError, "recipe census"):
            GEN.producer(records, FIXTURE)
        with self.assertRaisesRegex(ValueError, "invalid original case grid"):
            GEN.producer(self.mutate("case", side=4), FIXTURE)
        records = list(self.records); records.append(next(r for r in records if r["kind"] == "case"))
        with self.assertRaisesRegex(ValueError, "duplicate differential case"):
            GEN.producer(records, FIXTURE)

    def test_invalid_pattern_and_result_components_are_refused(self):
        with self.assertRaisesRegex(ValueError, "invalid original pattern"):
            GEN.producer(self.mutate("recipe", width=4), FIXTURE)
        result = {"item": "minecraft:stone", "count": 1, "added": [{"id": 0, "name": "minecraft:custom_data", "payload": "00"}], "removed": []}
        with self.assertRaisesRegex(ValueError, "unexpected normal output component"):
            GEN.producer(self.mutate("recipe", result=result), FIXTURE)

    def helper_refusal(self, inner, classes, expected):
        args = [str(JAVA_HOME / "bin/java"), "-cp", self.cp + os.pathsep + self.classes.name, "ferrumc.reference.NormalCraftingOracle", str(FIXTURE), str(EXPORT / "negative-unused.jsonl"), str(inner), str(classes)]
        result = subprocess.run(args, capture_output=True, text=True, cwd=EXPORT)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(expected, result.stdout + result.stderr)
        self.assertFalse((EXPORT / "negative-unused.jsonl").exists())

    def test_actual_helper_checks_original_codesource(self):
        self.helper_refusal(self.bundle, self.classes.name, "wrong original CodeSource: ac")

    def test_actual_helper_checks_fresh_helper_codesource(self):
        self.helper_refusal(self.inner, EXPORT, "wrong helper CodeSource")

    def test_java_option_injection_is_refused(self):
        for variable in ("JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS"):
            with self.subTest(variable=variable), tempfile.TemporaryDirectory(dir=EXPORT) as directory:
                # The pinned mappings must precede the option guard, exactly as
                # in a real export. Nothing starts Java when the guard refuses.
                Path(directory, "server-mappings.txt").write_bytes((EXPORT / "server-mappings.txt").read_bytes())
                env = dict(os.environ); env[variable] = "-Dcrafting.injected=true"
                result = subprocess.run([sys.executable, "-O", str(SOURCE.resolve()), str(CACHE), str(JAVA_HOME), directory], env=env, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("unsupported Java classpath/agent injection: " + variable, result.stderr)


if __name__ == "__main__":
    unittest.main()
