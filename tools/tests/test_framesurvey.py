"""Tests for tools/framesurvey.py's helpers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import framesurvey  # noqa: E402


def ins(*texts):
    return [(0x100 + 2 * i, 2, t) for i, t in enumerate(texts)]


class FrameSurveyTest(unittest.TestCase):
    def test_access_sizes(self):
        self.assertEqual(framesurvey.access_size('pea'), 'a')
        self.assertEqual(framesurvey.access_size('sne'), 'b')
        self.assertEqual(framesurvey.access_size('st'), 'b')
        self.assertEqual(framesurvey.access_size('movew'), 'w')
        self.assertEqual(framesurvey.access_size('tstl'), 'l')
        self.assertEqual(framesurvey.access_size('bsrw'), '?')

    def test_lone_byte_at_minus_2(self):
        # FILE_$REMOVE_WHEN_UNLOCKED 0xE5FC8E
        r = ins('linkw %fp,#-4', 'pea %a5@', 'pea %fp@(-2)', 'bsrw 0xe5e8e0',
                'moveb %fp@(-2),%d0', 'moveal %fp@(-8),%a5')
        offs = framesurvey.offsets(r, -4)
        self.assertEqual(dict(offs), {-2: {'a', 'b'}})
        self.assertTrue(framesurvey.lone(offs, -2, 'b'))
        self.assertEqual(framesurvey.align_counts([(r, -4)]), {('b', '-', 2): 1})

    def test_byte_inside_a_longword_is_not_lone(self):
        # VTOCE_$READ 0xE394EC: a bit field in the low byte of (-4,A6)
        r = ins('linkw %fp,#-20', 'movel %d0,%fp@(-4)', 'andib #-16,%fp@(-1)')
        offs = framesurvey.offsets(r, -20)
        self.assertFalse(framesurvey.lone(offs, -1, 'b'))
        self.assertFalse(framesurvey.lone(offs, -4, 'l'))

    def test_byte_pairs(self):
        # PROC2_$WAIT 0xE3FDD0: booleans one to a word
        r = ins('linkw %fp,#-164', 'clrb %fp@(-150)', 'clrb %fp@(-148)',
                'tstb %fp@(-150)', 'movew #-1,%fp@(-138)')
        self.assertEqual(framesurvey.byte_pairs(r, -164), [-148])
        r = ins('linkw %fp,#-8', 'clrb %fp@(-2)', 'clrb %fp@(-6)')
        self.assertEqual(framesurvey.byte_pairs(r, -8), [])

    def test_size_order(self):
        # PKT_$NEXT_ID-like: a word at the top, a longword below it
        r = ins('linkw %fp,#-8', 'movew %d0,%fp@(-2)', 'movel %d1,%fp@(-8)')
        self.assertEqual(framesurvey.size_order(r, -8), 'small-above-long')
        r = ins('linkw %fp,#-8', 'movel %d0,%fp@(-4)', 'movew %d1,%fp@(-6)')
        self.assertEqual(framesurvey.size_order(r, -8), 'longs-above')
        self.assertIsNone(framesurvey.size_order(ins('movew %d0,%fp@(-2)'), -4))

    def test_entry_copies(self):
        # FILE_$INVALIDATE 0xE75158: A0 used up by (A0)+, then A0, A1, A2
        r = ins('linkw %fp,#-24', 'moveml %a2/%a5,%sp@-', 'lea 0xe86068,%a5',
                'moveal %fp@(8),%a0', 'movel %a0@+,%fp@(-8)', 'movel %a0@+,%fp@(-4)',
                'moveal %fp@(12),%a0', 'movel %a0@,%fp@(-20)',
                'moveal %fp@(16),%a1', 'movel %a1@,%fp@(-16)',
                'moveal %fp@(20),%a2', 'moveb %a2@,%fp@(-22)',
                'pea %fp@(-12)', 'jsr 0xe46a00')
        got = framesurvey.entry_copies(r)
        self.assertEqual([(s, a, ok) for s, a, _, ok in got],
                         [(8, 0, True), (12, 0, True), (16, 1, True), (20, 2, True)])
        self.assertEqual(got[0][2], [('l', '+', -8), ('l', '+', -4)])
        # DIR_$GET_ENTRYU 0xE4D500: the status slot loaded at entry first
        r = ins('linkw %fp,#-8', 'moveml %a2/%a5,%sp@-', 'lea 0xe7dc00,%a5',
                'moveal %fp@(24),%a2', 'moveal %fp@(8),%a0',
                'movel %a0@+,%fp@(-8)', 'movel %a0@+,%fp@(-4)', 'bras 0x0')
        self.assertEqual([(s, a, ok) for s, a, _, ok in framesurvey.entry_copies(r)],
                         [(8, 0, True)])
        # TONE_$TIME 0xE17300: 6 bytes, a longword and a displaced word
        r = ins('linkw %fp,#-12', 'moveal %fp@(8),%a0', 'movel %a0@,%fp@(-8)',
                'movew %a0@(4),%fp@(-4)')
        self.assertEqual(framesurvey.slot_copy(r, 1)[2],
                         [('l', '0', -8), ('w', 'd', -4)])

    def test_outside_the_link_area(self):
        r = ins('linkw %fp,#-4', 'movew %fp@(-6),%d0', 'movew %fp@(8),%d1')
        self.assertEqual(framesurvey.accesses(r, -4), [])


if __name__ == '__main__':
    unittest.main()
