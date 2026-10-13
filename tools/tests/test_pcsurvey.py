"""Tests for tools/pcsurvey.py's pure parts (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import pcsurvey  # noqa: E402


class OperandTest(unittest.TestCase):
    def test_pc_operand(self):
        # TERM_$SEND_KBD_STRING 0xE1AB0C
        self.assertEqual(pcsurvey.pc_operand('pea %pc@(0xe1ab26)'),
                         (0xE1AB26, 'pea %pc@(T)'))
        self.assertEqual(pcsurvey.pc_operand('lea %pc@(0xe4cd84),%a0'),
                         (0xE4CD84, 'lea %pc@(T),%a0'))
        # a `case' table index is not a constant
        self.assertIsNone(pcsurvey.pc_operand('jmp %pc@(0xe5c6d6,%d0:w)'))
        self.assertIsNone(pcsurvey.pc_operand('pea %fp@(-4)'))


class ClassTest(unittest.TestCase):
    def test_classes(self):
        # routine 0x100..0x130, code up to 0x12a
        k = lambda t, code=(): pcsurvey.site_class(0x100, 0x130, 0x12a, t, set(code))
        self.assertEqual(k(0x12a), 'own')
        self.assertEqual(k(0x80), 'back')
        self.assertEqual(k(0x130), 'fwd')
        self.assertEqual(k(0x80, [0x80]), 'code')


class LayoutTest(unittest.TestCase):
    def test_timer_init(self):
        # TIMER_$INIT 0xE1639C: used 9C, A4, A2, A0, 9E; laid out in
        # address order
        fu = [0x9C, 0xA4, 0xA2, 0xA0, 0x9E]
        size = {t: 2 for t in fu}
        self.assertEqual(pcsurvey.layout(fu, size), sorted(fu))

    def test_words_before_longs(self):
        # LOG_$SHUTDN 0xE17604: the longword used first, laid out after
        # the word; DIR_$OLD_FIX_DIR 0xE56094: each group in its order
        self.assertEqual(pcsurvey.layout([8, 4], {8: 4, 4: 2}), [4, 8])
        fu = [0, 4, 2, 6, 0xE, 0xA]
        size = {0: 2, 2: 2, 4: 2, 6: 4, 0xA: 4, 0xE: 4}
        self.assertEqual(pcsurvey.layout(fu, size), [0, 2, 4, 6, 0xA, 0xE])


class CutTest(unittest.TestCase):
    def test_filler_and_tail(self):
        # LOG_$SHUTDN 0xE17604: 0004 2048 0000 0000, the next routine at
        # 0xE1760C
        words = {0x604: 0x0004, 0x606: 0x2048, 0x608: 0, 0x60A: 0}
        self.assertEqual(pcsurvey.cut(0x604, 0x60C, [0x604, 0x608], words.get),
                         [(0x604, 2), (0x608, 4)])
        # a trailing filler is not part of the last cell
        words = {0x600: 0x0001, 0x602: 0x2048}
        self.assertEqual(pcsurvey.cut(0x600, 0x604, [0x600], words.get), [(0x600, 2)])


if __name__ == '__main__':
    unittest.main()
