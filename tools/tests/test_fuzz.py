"""Tests for tools/fuzz.py: generation is deterministic, and generated
modules parse (with dpc --parse-only, skipped when dpc is not built)."""

import os
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import fuzz  # noqa: E402


class GenerationTest(unittest.TestCase):
    def test_deterministic(self):
        self.assertEqual(fuzz.Gen(5).module(), fuzz.Gen(5).module())
        self.assertNotEqual(fuzz.Gen(5).module(), fuzz.Gen(6).module())
        self.assertEqual(fuzz.soup(3), fuzz.soup(3))
        files = fuzz.corpus()
        self.assertTrue(files)
        self.assertEqual(fuzz.mutate(9, files), fuzz.mutate(9, files))

    def test_first_error(self):
        err = "a.pas:1:2: error: undefined identifier `x` at 12\n"
        self.assertEqual(fuzz.first_error(err), "undefined identifier _ at _")
        self.assertIsNone(fuzz.first_error(""))


@unittest.skipUnless(os.path.exists(fuzz.DEFAULT_DPC), "dpc not built")
class ParseTest(unittest.TestCase):
    def test_generated_modules_parse(self):
        with tempfile.TemporaryDirectory() as d:
            for seed in range(5):
                path = os.path.join(d, f"f{seed}.pas")
                with open(path, "w") as f:
                    f.write(fuzz.Gen(seed).module())
                p = subprocess.run([fuzz.DEFAULT_DPC, "--parse-only", path],
                                   capture_output=True, text=True, timeout=5)
                self.assertEqual(p.returncode, 0, p.stderr)

    def test_run_flags_only_crashes(self):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "bad.pas")
            with open(path, "w") as f:
                f.write("module m; procedure p; begin x := 1 end;\n")
            why, code, _ = fuzz.run(fuzz.DEFAULT_DPC, path, ["--check-only"])
            self.assertIsNone(why)
            self.assertEqual(code, 1)

    def test_run_flags_internal_errors(self):
        # The driver's test-only panic trigger: the backstop's report
        # (exit 3, `internal error`) is a failure.
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "m.pas")
            with open(path, "w") as f:
                f.write("module m;\n")
            old = os.environ.get("DPC_INTERNAL_PANIC_FOR_TEST")
            os.environ["DPC_INTERNAL_PANIC_FOR_TEST"] = "0"
            try:
                why, code, err = fuzz.run(fuzz.DEFAULT_DPC, path, [])
            finally:
                if old is None:
                    del os.environ["DPC_INTERNAL_PANIC_FOR_TEST"]
                else:
                    os.environ["DPC_INTERNAL_PANIC_FOR_TEST"] = old
            self.assertEqual(why, "exit 3")
            self.assertIn("internal error", err)


if __name__ == '__main__':
    unittest.main()
