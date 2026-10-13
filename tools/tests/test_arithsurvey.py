"""Tests for tools/arithsurvey.py's site recognisers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import arithsurvey  # noqa: E402


def insns(texts):
    """[(addr, len, text)] at 2-byte steps (lengths do not matter here)."""
    return [(0x100 + 2 * i, 2, t) for i, t in enumerate(texts)]


class CallSiteTest(unittest.TestCase):
    def test_word_argument_with_pad(self):
        # AREA_$FREE_FROM 0xE07FE6..0xE07FF8
        ins = insns(['subql #2,%sp', 'movew #11,%sp@-', 'movel %d2,%sp@-',
                     'jsr 0xe0acc0', 'addqw #8,%sp', 'clrl %d1', 'movew %d0,%d1'])
        c = arithsurvey.call_site(ins, 3)
        self.assertEqual(c['entry'], 'M$OIU$WLW')
        self.assertEqual(c['a'], 'movel %d2,%sp@-')
        self.assertEqual(c['b'], 'movew #11,%sp@-')
        self.assertEqual(c['pad'], 0x100)
        self.assertFalse(c['pad_inside'])
        self.assertEqual(c['pop'], 'addqw #8,%sp')
        self.assertEqual(c['use'], 'movew %d0,%d1')

    def test_pad_scheduled_into_the_operand(self):
        # CAL_$WEEKDAY 0xE686AE..0xE686C6
        ins = insns(['movew %a1@,%d0', 'subql #2,%sp', 'extl %d0', 'addil #9,%d0',
                     'movew #12,%sp@-', 'movel %d0,%sp@-', 'jsr 0xe0ad54', 'addqw #8,%sp',
                     'mulsw #153,%d0'])
        c = arithsurvey.call_site(ins, 6)
        self.assertEqual(c['entry'], 'M$OIS$WLW')
        self.assertTrue(c['pad_inside'])
        self.assertEqual(c['use'], 'mulsw #153,%d0')

    def test_unpopped_longword_call(self):
        # TIME_$GET_ADJUST 0xE16AD0..0xE16ADE
        ins = insns(['movel #250000,%sp@-', 'movel %d1,%sp@-', 'jsr 0xe0ad7c', 'lsll #2,%d0'])
        c = arithsurvey.call_site(ins, 2)
        self.assertEqual(c['entry'], 'M$OIS$LLL')
        self.assertIsNone(c['pad'])
        self.assertIsNone(c['pop'])

    def test_other_calls_are_not_sites(self):
        ins = insns(['subql #2,%sp', 'movew #14,%sp@-', 'jsr 0xe20b12'])
        self.assertIsNone(arithsurvey.call_site(ins, 2))


class InlineSiteTest(unittest.TestCase):
    def test_longword_index(self):
        ins = insns(['movew %d2,%d0', 'mulsw #228,%d0', 'leal %a0@(0,%d0:l),%a2'])
        s = arithsurvey.inline_site(ins, 1)
        self.assertEqual((s['op'], s['src'], s['cls']), ('mulsw', 'imm', 'index.l'))

    def test_product_used_as_a_longword(self):
        # PROC1_$LOADAV_CALLBACK 0xE14C18
        ins = insns(['mulsw #5240,%d2', 'addl %d2,%d0'])
        s = arithsurvey.inline_site(ins, 0)
        self.assertEqual((s['op'], s['cls']), ('mulsw', 'addl'))


if __name__ == '__main__':
    unittest.main()
