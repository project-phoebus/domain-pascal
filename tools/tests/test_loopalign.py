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


class SteppedPointersTest(unittest.TestCase):
    """`ptrs' (bead domain-pascal-xm5w): the stepped pointers of a `dbf'
    loop and how the preheader forms them."""

    def ins(self, texts, at=0x100):
        out, a = [], at
        for t in texts:
            out.append((a, 2, t))
            a += 2
        return out

    def test_roots(self):
        self.assertEqual(loopalign.root_class('lea %a5@(36),%a2'), 'lea A5')
        self.assertEqual(loopalign.root_class('lea %a5@,%a1'), 'lea A5')
        self.assertEqual(loopalign.root_class('moveal %a5,%a0'), 'movea A5')
        self.assertEqual(loopalign.root_class('lea %fp@(4),%a0'), 'lea A6')
        self.assertEqual(loopalign.root_class('moveal %fp,%a0'), 'movea A6')
        self.assertEqual(loopalign.root_class('moveal #14848408,%a2'), 'movea abs')
        self.assertEqual(loopalign.root_class('lea %a1@(2),%a0'), 'lea An')
        self.assertEqual(loopalign.root_class('moveal %fp@(8),%a2'), 'load frame')
        self.assertEqual(loopalign.root_class('moveal %d3,%a1'), 'from Dn')

    def test_in_place_and_new(self):
        # EC2_$REGISTER_EC1 0xE42968: `movea.l A5,A0; moveq #1,D1;
        # addq.l #4,A0', the head, `cmpa.l (2700,A0),A1', the step.
        ins = self.ins([
            'subqw #1,%d0',
            'bmis 0x200',
            'moveal %a5,%a0',
            'moveq #1,%d1',
            'addql #4,%a0',
            'cmpal %a0@(2700),%a1',       # head, index 5
            'addqw #1,%d1',
            'addql #4,%a0',
            'dbf %d0,0x10a',              # index 8
        ])
        (p,) = loopalign.stepped_pointers(ins, 5, 8)
        self.assertEqual(p['reg'], '%a0')
        self.assertEqual((p['kind'], p['step'], p['root']), ('step', 4, 'movea A5'))
        self.assertEqual([t for _, t in p['chain']], ['moveal %a5,%a0', 'addql #4,%a0'])
        self.assertEqual((p['offset'], p['disps'], p['count']), (4, [2700], 'run'))
        # DBUF_$INVALIDATE 0xE3AA0E: `lea (36,A5),A2', the filler skipped,
        # a `with' copy at the head, a constant count.
        ins = self.ins([
            'moveq #7,%d4',
            'lea %a5@(36),%a2',
            'moveal %a0,%a0',
            'moveal %a2,%a3',             # head, index 3
            'tstl %a3@(-8)',
            'lea %a2@(36),%a2',
            'dbf %d4,0x106',              # index 6
        ])
        (p,) = loopalign.stepped_pointers(ins, 3, 6, lambda a: a == 0x104)
        self.assertEqual((p['root'], p['offset'], p['step'], p['count']),
                         ('lea A5', 36, 36, 'const'))
        self.assertEqual(p['disps'], [])

    def test_lea_from_pointer_is_an_access(self):
        # DIR_$DO_OP 0xE4CC16: `lea (5460,A0),A3' reaches an element
        # through the stepped A0 (it is not A0's step).
        ins = self.ins([
            'moveal %a5,%a0',
            'moveq #1,%d1',
            'addql #8,%a0',
            'lea %a0@(5460),%a3',         # head, index 3
            'cmpml %a3@+,%a4@+',
            'addqw #1,%d1',
            'addql #8,%a0',
            'dbf %d0,0x106',              # index 7
        ])
        ps = loopalign.stepped_pointers(ins, 3, 7)
        (p,) = [x for x in ps if x['reg'] == '%a0']
        self.assertEqual((p['root'], p['offset'], p['disps']), ('movea A5', 8, [5460]))


if __name__ == '__main__':
    unittest.main()
