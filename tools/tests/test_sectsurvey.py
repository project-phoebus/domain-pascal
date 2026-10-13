"""Tests for tools/sectsurvey.py (the map is needed; the image only for
`calls', which is not run here)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import sau2map  # noqa: E402
import sectsurvey  # noqa: E402


class CallKindTest(unittest.TestCase):
    def test_kinds(self):
        self.assertEqual(sectsurvey.call_kind('jsr 0xe14b12'), ('jsr', 0xE14B12))
        self.assertEqual(sectsurvey.call_kind('bsrw 0xe5bb1e'), ('bsr', 0xE5BB1E))
        self.assertEqual(sectsurvey.call_kind('bsrs 0xe00eb0'), ('bsr', 0xE00EB0))
        self.assertIsNone(sectsurvey.call_kind('jsr %a1@'))


@unittest.skipUnless(os.path.exists(sau2map.DEFAULT_MAP), 'no SAU2 map')
class ModulesTest(unittest.TestCase):
    def test_scsi_and_proc1(self):
        mods = sectsurvey.modules(sau2map.load())
        self.assertEqual(mods['SCSI_'], [('I', 'OS_INIT_PROC', 0xE34F04, 0x10),
                                         ('I', 'SCSI_WIRED_PROC', 0xE88800, 0x4),
                                         ('D', 'SCSI_WIRED_DATA', 0xE88804, 0x4)])
        self.assertEqual([x[1] for x in mods['PROC1_']],
                         ['WIRED_PROC', 'WIRED_DATA', 'OS_INIT_PROC'])
        self.assertEqual(len(mods), 52)
