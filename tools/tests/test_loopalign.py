"""Tests for tools/loopalign.py's `switch' (no image needed)."""

import os
import sys
import unittest
from collections import Counter

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import loopalign  # noqa: E402


class Seg:
    def __init__(self, name):
        self.name = name


def pr(seg, classes):
    """per_routine()-style rows for routines at 0x100, 0x200, ... with
    loop-head counters giving the one-letter classes in `classes'."""
    c = {'F': Counter(F=1), 'X': Counter(F=1, U=1), 'u': Counter(U=1),
         'a': Counter(A=1), '.': Counter()}
    return [(seg, 0x100 * (i + 1), 0x100 * (i + 2), c[k]) for i, k in enumerate(classes)]


class SwitchTest(unittest.TestCase):
    def test_switch(self):
        s, other = Seg('S'), Seg('T')
        rows = pr(other, 'uuu') + pr(s, 'aFX.au.u')
        # last F/X routine 0x300, first later u routine 0x600
        self.assertEqual(loopalign.switch(rows, s), (0x300, 0x600))
        self.assertEqual(loopalign.switch(pr(s, '..u'), s), (None, 0x300))
        self.assertEqual(loopalign.switch(pr(s, 'F..'), s), (0x100, None))
        self.assertEqual(loopalign.switch(pr(s, '...'), s), (None, None))

    def test_switching_back_on_is_reported(self):
        s = Seg('S')
        with self.assertRaisesRegex(ValueError, 'routine 0x100 does not align'):
            loopalign.switch(pr(s, 'uF'), s)


class Site:
    def __init__(self, seg, src, tgt, kind, text, cls):
        self.seg, self.src, self.tgt, self.kind, self.text, self.cls = \
            seg, src, tgt, kind, text, cls


class Map:
    def __init__(self, segs):
        self.segments = segs


class HeadRowsTest(unittest.TestCase):
    def test_while_and_dbf_heads(self):
        s = Seg('S')
        # a `while' (bra to the bottom test, filler, head) and a `dbf' loop
        loopalign.INSNS[0x100] = [
            (0x100, 2, 'bras 0x10c'),
            (0x102, 2, 'moveal %a0,%a0'),
            (0x104, 2, 'addqw #1,%d0'),
            (0x106, 2, 'nop'),
            (0x108, 2, 'nop'),
            (0x10A, 2, 'nop'),
            (0x10C, 2, 'tstw %d0'),
            (0x10E, 2, 'bnes 0x104'),
            (0x110, 2, 'moveq #3,%d1'),
            (0x112, 2, 'clrw %d2'),
            (0x114, 2, 'addqw #1,%d2'),
            (0x116, 4, 'dbf %d1,0x114'),
            (0x11A, 2, 'rts'),
        ]
        sites = [Site(s, 0x100, 0x10C, 'fwd', 'bras 0x10c', 'A'),
                 Site(s, 0x10E, 0x104, 'bwd', 'bnes 0x104', 'F'),
                 Site(s, 0x116, 0x114, 'dbf', 'dbf %d1,0x114', 'U')]
        rows = loopalign.head_rows(Map([s]), sites, [(s, 0x100, 0x200)])
        self.assertEqual([r[:5] for r in rows],
                         [(0x104, 'F', 'bcc', 'bra-in', 'al'),
                          (0x114, 'U', 'dbf', 'fall', 'al')])


if __name__ == '__main__':
    unittest.main()
