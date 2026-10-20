"""Tests for tools/dpcflags.py: the DPC_FLAGS of ../source's Makefile."""

import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import dpcflags  # noqa: E402
import srcsurvey  # noqa: E402


class FromTextTest(unittest.TestCase):
    def test_forms(self):
        t = dpcflags.from_text
        self.assertEqual(t('CC := gcc\n'), [])
        self.assertEqual(t('DPC_FLAGS := -config sau2\n'), ['-config', 'sau2'])
        self.assertEqual(t('DPC_FLAGS = -config sau2 # the SAU2 kernel\n'),
                         ['-config', 'sau2'])
        self.assertEqual(t('DPC_FLAGS ?= -cond\nDPC_FLAGS ?= -nwarn\n'), ['-cond'])
        self.assertEqual(t('DPC_FLAGS := -config sau2\nDPC_FLAGS += -cond\n'),
                         ['-config', 'sau2', '-cond'])
        self.assertEqual(t('DPC_FLAGS := -config \\\n   sau2 a\n'), ['-config', 'sau2', 'a'])
        self.assertEqual(t('override DPC_FLAGS ::= -natural\n'), ['-natural'])
        self.assertEqual(t('DPC_FLAGS :=\n'), [])
        # recipe lines and other variables are not assignments of it
        self.assertEqual(t('x:\n\tDPC_FLAGS=-cond foo\nMY_DPC_FLAGS := -cond\n'), [])
        # a reference cannot be read from the text
        self.assertIsNone(t('DPC_FLAGS := -config $(CONFIG)\n'))


class MakefileTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def makefile(self, text):
        with open(os.path.join(self.tmp, 'Makefile'), 'w') as fh:
            fh.write(text)

    def test_no_makefile(self):
        self.assertEqual(dpcflags.dpc_flags(self.tmp), ([], 'none'))

    @unittest.skipUnless(shutil.which('make'), 'needs make')
    def test_make_evaluates(self):
        self.makefile('CONFIG := sau2\nDPC_FLAGS := -config $(CONFIG)\n'
                      'all:\n\t@echo built > built.txt\n')
        self.assertEqual(dpcflags.dpc_flags(self.tmp), (['-config', 'sau2'], 'make'))
        # make's command-line variables count
        self.assertEqual(dpcflags.from_make(self.tmp, ['CONFIG=other']), ['-config', 'other'])
        # nothing of the Makefile's own is built
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'built.txt')))
        self.makefile('all:\n\ttrue\n')
        self.assertEqual(dpcflags.dpc_flags(self.tmp), ([], 'make'))

    def test_text_when_make_fails(self):
        self.makefile('DPC_FLAGS := -config sau2\n')
        self.assertEqual(dpcflags.dpc_flags(self.tmp, make='no-such-make-here'),
                         (['-config', 'sau2'], 'Makefile text'))
        self.makefile('DPC_FLAGS := $(X)\n')
        self.assertEqual(dpcflags.dpc_flags(self.tmp, make='no-such-make-here'), ([], 'none'))

    def test_command_line_wins(self):
        self.makefile('DPC_FLAGS := -config sau2\n')
        self.assertEqual(dpcflags.dpc_flags(self.tmp, ['DPC_FLAGS=-cond -config x']),
                         (['-cond', '-config', 'x'], 'command line'))


class SrcsurveyTest(unittest.TestCase):
    def test_flags_come_first(self):
        # `dpc -config sau2 -I . ... MODULE': the list ends at `-I'.
        seen = []

        class P:
            returncode, stderr = 0, ''

        def fake_run(cmd, **_kw):
            seen.append(cmd)
            return P()
        old = srcsurvey.subprocess.run, list(srcsurvey.SOURCE_DPC_FLAGS)
        srcsurvey.subprocess.run = fake_run
        srcsurvey.SOURCE_DPC_FLAGS[:] = ['-config', 'sau2']
        try:
            srcsurvey.run_dpc('dpc', '.', 'm/m.pas', check_only=True)
        finally:
            srcsurvey.subprocess.run = old[0]
            srcsurvey.SOURCE_DPC_FLAGS[:] = old[1]
        self.assertEqual(seen[0][:5], ['dpc', '-config', 'sau2', '-I', '.'])
        self.assertEqual(seen[0][-1], 'm/m.pas')


if __name__ == '__main__':
    unittest.main()
