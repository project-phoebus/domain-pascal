"""Tests for tools/a5survey.py's `switch' and `classify_insns' (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import a5survey  # noqa: E402


def rows(letters):
    """[(start, letter)] for routines at 0x100, 0x200, ..."""
    return [(0x100 * (i + 1), l) for i, l in enumerate(letters)]


class SwitchTest(unittest.TestCase):
    def test_switch(self):
        # SMD 0xE1D750 shape: n, then s s -> switch between them
        self.assertEqual(a5survey.switch(rows('nss')), (0x100, 0x200, []))
        # S fits both modes; last n before the first s bounds the switch
        self.assertEqual(a5survey.switch(rows('SnSSsS')), (0x200, 0x500, []))
        # no s: load iff used throughout
        self.assertEqual(a5survey.switch(rows('nSnS')), (0x300, None, []))
        # s from the start (MMAP_ shape has S first: no n at all)
        self.assertEqual(a5survey.switch(rows('SSsS')), (None, 0x300, []))

    def test_late_n_is_reported(self):
        # XPD 0xE5AF38 shape: n..n s..s n..n s..s
        last, first, late = a5survey.switch(rows('nnssnns'))
        self.assertEqual((last, first), (0x200, 0x300))
        self.assertEqual(late, [0x500, 0x600])


class ClassifyTest(unittest.TestCase):
    def test_save_load_restore_is_not_a_use(self):
        # SMD_$BUSY_WAIT 0xE1D89E
        ins = [(0, 4, 'linkw %fp,#0'), (4, 2, 'pea %a5@'), (6, 6, 'lea 0xe2e060,%a5'),
               (12, 4, 'moveal %fp@(12),%a0'), (16, 2, 'clrl %a0@'),
               (18, 4, 'moveal %fp@(-4),%a5'), (22, 2, 'unlk %fp'), (24, 2, 'rts')]
        sets, uses, leas, _, _, save = a5survey.classify_insns(ins)
        self.assertEqual((sets, uses, leas, save), (True, False, [0xE2E060], 'pea'))

    def test_a5_operand_is_a_use(self):
        # PROC1_$GET_LOADAV 0xE14BBA: `lea (A5),A0'; movem form elsewhere
        ins = [(0, 4, 'linkw %fp,#0'), (4, 4, 'moveml %d2/%a5,%sp@-'),
               (8, 6, 'lea 0xe254e8,%a5'), (14, 2, 'lea %a5@,%a0')]
        sets, uses, _, _, _, save = a5survey.classify_insns(ins)
        self.assertEqual((sets, uses, save), (True, True, 'movem'))

    def test_pea_a5_in_body_is_a_use(self):
        # `pea (A5)' not followed by the load passes the data's address
        ins = [(0, 2, 'pea %a5@'), (2, 6, 'jsr 0xe20b12')]
        self.assertTrue(a5survey.classify_insns(ins)[1])

    def test_bsr_targets(self):
        ins = [(0, 4, 'bsrw 0xe00eb0'), (4, 2, 'bsrs 0xe00f00')]
        self.assertEqual(a5survey.classify_insns(ins)[4], [0xE00EB0, 0xE00F00])


class TableTest(unittest.TestCase):
    def setUp(self):
        self._word = a5survey.loopalign.word

    def tearDown(self):
        a5survey.loopalign.word = self._word

    def test_bound_and_backward_targets(self):
        # ASKNODE_$INTERNET_INFO 0xE64FA6 shape: 4 entries, the last two
        # go back before the table
        mem = {0x1000: 0x0008, 0x1002: 0x0008, 0x1004: 0xFA7E, 0x1006: 0xFA7E}
        a5survey.loopalign.word = lambda a: mem[a]
        prev = ['cmpil #4,%d0', 'bccw 0x2000', 'addw %d0,%d0',
                'movew %pc@(0x1000,%d0:w),%d0']
        self.assertEqual(a5survey.table_entries(0x1000, prev, 0x0800, 0x1100),
                         [0x1008, 0x1008, 0x0A7E, 0x0A7E])

    def test_no_bound_stops_at_lowest_target(self):
        mem = {0x1000: 0x0006, 0x1002: 0x0004, 0x1004: 0x4E75}
        a5survey.loopalign.word = lambda a: mem[a]
        self.assertEqual(a5survey.table_entries(0x1000, [], 0x0800, 0x1100),
                         [0x1006, 0x1004])


if __name__ == '__main__':
    unittest.main()
