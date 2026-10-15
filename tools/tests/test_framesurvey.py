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

    # ---- hidden (bead domain-pascal-s9g0)

    def test_used_bytes(self):
        # sizes from the access; one byte for pea and indexed operands;
        # nothing outside the link area or from the movem restore
        r = ins('linkw %fp,#-16', 'movew %d0,%fp@(-2)', 'pea %fp@(-8)',
                'movel %fp@(-12,%d0:w),%d1', 'moveml %fp@(-24),%d2/%a2',
                'movel %fp@(8),%d2', 'movel %d1,%fp@(-20)')
        self.assertEqual(framesurvey.used_bytes(r, -16), {-2, -1, -8, -12})

    def test_counts(self):
        # MMAP_$AVAIL 0xE0CC80: one element address; KBD 0xE1CA62-like dbf
        r = ins('lea %a0@(0,%d0:l),%a2', 'lea %a5@(12),%a0', 'dbf %d0,0x100',
                'lea %a0@(12),%a0', 'addql #4,%a1')
        self.assertEqual(framesurvey.elem_leas(r), 1)
        self.assertEqual(framesurvey.dbf_loops(r), 1)
        self.assertEqual(framesurvey.stepped_regs(r), {'%a0', '%a1'})

    def test_saved_regs(self):
        r = ins('linkw %fp,#-8', 'moveml %d2-%d4/%a2-%a3/%a5,%sp@-')
        self.assertEqual(framesurvey.saved_regs(r), {'d2', 'd3', 'd4', 'a2', 'a3'})
        self.assertEqual(framesurvey.saved_regs(ins('linkw %fp,#-4', 'pea %a2@')), {'a2'})
        self.assertEqual(framesurvey.saved_regs(ins('linkw %fp,#-4', 'pea %a5@')), set())

    def test_slot_reads(self):
        # TTY_$K_SET_FLAG 0xE67422: the line read in place and pushed, the
        # flag read into D0; a slot copied into the frame is left out
        r = ins('linkw %fp,#-8', 'moveal %fp@(8),%a0', 'movew %a0@,%sp@-',
                'jsr 0xe66738', 'moveal %fp@(12),%a0', 'movew %a0@,%d0',
                'moveal %fp@(16),%a1', 'movel %a1@,%fp@(-4)')
        self.assertEqual(framesurvey.slot_reads(r), {8: 'w', 12: 'w'})

    def test_standin_bytes(self):
        decls = """
  tty: tty_$desc_ptr_t;                   { (-0x4,A6): A2 }
  unused_8: integer32;                    { (-0x8,A6): stand-in, see
                                            the TODO }
  unused_a: integer;                      { -0xA: the stand-in (TODO) }
  unused_30: array[1..3] of integer32;    { -0x30: the stand-in (TODO) }
  odd: array[1..3] of char;               { stand-in }
  pad: weird_t;                           { stand-in }
"""
        self.assertEqual(framesurvey.standin_bytes(decls), (4 + 2 + 12 + 4, ['pad']))

    def test_image_fit(self):
        f = lambda **k: dict(dict(L=4, free=4, untouched=True, reserved=0, func=False,
                                  elem=0, dbf=0, slots={}, saved=set(), stepped=set()), **k)
        fs = [f(elem=1), f(L=0, free=0, elem=1), f(L=8, free=8)]
        rows = {r[0].split(':')[0]: r[1:] for r in framesurvey.image_fit(fs)}
        # elem: two predict, one refuted (no room), one exact of three
        self.assertEqual(rows['elem'], (2, 1, 1, 2))
        self.assertEqual(rows['none'], (0, 0, 0, 2))

    def test_unit_link(self):
        text = ('\t.globl\tA\nA:\n\tlink.w\t%fp,#-0xc\n\trts\n'
                'B:\n\trts\nC:\n\tlink.w\t%fp,#0\n\trts\n')
        self.assertEqual(framesurvey.unit_link(text, 'a'), -12)
        self.assertIsNone(framesurvey.unit_link(text, 'b'))
        self.assertEqual(framesurvey.unit_link(text, 'c'), 0)
        self.assertIsNone(framesurvey.unit_link(text, 'd'))

    def test_compare_links(self):
        names = {'a': 1, 'b': 2, 'c': 3, 'd': 4}
        img = {1: -4, 2: -8, 3: -8, 4: 0}
        before = {'a': 0, 'b': -8, 'c': 0, 'd': 0}
        after = {'a': -4, 'b': -12, 'c': -4, 'd': 0}
        fixed, broke, closer, farther, n0, n1 = framesurvey.compare_links(
            names, before.get, after.get, img.get)
        self.assertEqual([x[0] for x in fixed], ['a'])
        self.assertEqual([x[0] for x in broke], ['b'])
        self.assertEqual([x[0] for x in closer], ['c'])
        self.assertEqual(farther, [])
        self.assertEqual((n0, n1), (2, 2))


if __name__ == '__main__':
    unittest.main()
