"""Tests for tools/sau2map.py (the SAU2 link map reader)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import sau2map  # noqa: E402

SAMPLE = """\
Build ID: test
I 4  E00800  WIRED_PROC         loaded at 102000, size = 1D0F4
I    E1471C  PROC1_             size = D88
     E1471C  PROC1_$TRY_TO_SUSPEND
     E148CA  PROC1_$TST_LOCK
     E148E6  PROC1_$GET_LOCKS
     E151FE  EC_$INIT                         MARKED
D    E1EAC8  PROC1_ASM          size = 24A4
     E1EAC8  PROC1_$CURRENT_PCB
"""


class SampleTest(unittest.TestCase):
    def setUp(self):
        self.m = sau2map.LinkMap.parse(SAMPLE)

    def test_lines(self):
        self.assertEqual(len(self.m.sections), 1)
        self.assertTrue(self.m.sections[0].is_section)
        self.assertEqual(self.m.sections[0].load_addr, 0x102000)
        seg = self.m.segment('PROC1_')
        self.assertEqual((seg.kind, seg.addr, seg.size), ('I', 0xE1471C, 0xD88))
        self.assertTrue(self.m.lookup('EC_$INIT').marked)
        self.assertEqual(self.m.lookup('proc1_$tst_lock').addr, 0xE148CA)

    def test_extent(self):
        self.assertEqual(self.m.routine_extent('PROC1_$TST_LOCK'), (0xE148CA, 0x1C))
        # last symbol of a segment runs to the segment end
        self.assertEqual(self.m.routine_extent('EC_$INIT'), (0xE151FE, 0xE154A4 - 0xE151FE))

    def test_bad_line(self):
        with self.assertRaises(ValueError):
            sau2map.LinkMap.parse('Build ID: x\nnonsense here\n')


@unittest.skipUnless(os.path.exists(sau2map.DEFAULT_MAP), 'SAU2 map not present')
class RealMapTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.m = sau2map.load()

    def test_symbols(self):
        self.assertEqual(self.m.addr_of('PROC1_$TST_LOCK'), 0xE148CA)
        self.assertEqual(self.m.addr_of('PROC1_$GET_LOCKS'), 0xE148E6)
        self.assertEqual(self.m.addr_of('PROC1_$CURRENT_PCB'), 0xE1EAC8)
        self.assertIsNone(self.m.lookup('NO_SUCH_SYMBOL'))

    def test_segment_line(self):
        # I    E1471C  PROC1_             size = D88
        seg = self.m.segment('PROC1_', 'I')
        self.assertEqual((seg.addr, seg.size, seg.is_section), (0xE1471C, 0xD88, False))
        self.assertIs(self.m.segment_containing(0xE148CA), seg)

    def test_sections(self):
        wp = [s for s in self.m.sections if s.name == 'WIRED_PROC'][0]
        self.assertEqual((wp.kind, wp.number, wp.addr, wp.size), ('I', 4, 0xE00800, 0x1D0F4))

    def test_extents(self):
        self.assertEqual(self.m.routine_extent('PROC1_$TST_LOCK'), (0xE148CA, 0x1C))
        self.assertEqual(self.m.routine_extent('AST_$PAGE_ZERO'), (0xE00EEC, 0x2A))
        # module-local worker ahead of AST_$PAGE_ZERO (CLAUDE.md 6.3)
        self.assertEqual(self.m.extent_at(0xE00EB0), (0xE00EB0, 0x3C))


if __name__ == '__main__':
    unittest.main()
