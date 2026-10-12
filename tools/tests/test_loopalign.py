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


if __name__ == '__main__':
    unittest.main()
