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


class GuardClassTest(unittest.TestCase):
    """`guards' (codegen-patterns 7.4, "Run-time bounds")."""

    def test_subq_copied(self):
        # DTTY_$WRITE_STRING 0xE1D6AA..0xE1D6B2
        self.assertEqual(loopalign.guard_class(
            'bmi', ['moveal %fp@(12),%a0', 'movew %a0@,%d0', 'subqw #1,%d0'],
            ['movew %d0,%d3', 'moveq #1,%d2'], '%d3'),
            ('bmi subq #1', 'copied', '1'))

    def test_subq_in_place_from_zero(self):
        # KBD's module-local 0xE1CA8C at 0xE1CAA8..0xE1CAAE
        self.assertEqual(loopalign.guard_class(
            'bmi', ['movew %d0,%d1', 'subqw #1,%d1'], ['clrw %d2'], '%d1'),
            ('bmi subq #1', 'in place', '0'))

    def test_tst_self_copy(self):
        # XNS_IDP_$REGISTER_ADDR 0xE19020..0xE1902A
        self.assertEqual(loopalign.guard_class(
            'bmi', ['movew %a5@(1336),%d0', 'tstw %d0'],
            ['movew %d0,%d0', 'clrw %d1', 'lea %a5@,%a0'], '%d0'),
            ('bmi tst', 'self-copy', '0'))

    def test_unsigned_reloaded(self):
        # 0xE10BD4's preheader: `tst.w (-478,A2); beq; move.w (-478,A2),D3'
        self.assertEqual(loopalign.guard_class(
            'beq', ['tstw %a2@(-478)'],
            ['movew %a2@(-478),%d3', 'subqw #1,%d3', 'moveq #1,%d4'], '%d3'),
            ('beq tst mem', 'reloaded', '1'))

    def test_exit_chain(self):
        # a guard may jump past the exit's `bra' (DISK_$AS_XFER_MULTI
        # 0xE6BA46 `bmi.w 0xe6bb8c')
        ins = [(0x100, 4, 'dbf %d2,0x0f0'), (0x104, 2, 'bras 0x110'),
               (0x106, 2, 'clrw %d0'), (0x110, 2, 'rts')]
        idx = {a: i for i, (a, n, x) in enumerate(ins)}
        self.assertEqual(loopalign.exit_chain(ins, idx, 0), {0x104, 0x110})

    def test_run_time_lower_bound(self):
        # 0xE02BFE..0xE02C0A
        self.assertEqual(loopalign.guard_class(
            'bmi', ['movew %d2,%d1', 'movew %d3,%d0', 'addqw #1,%d1', 'subw %d1,%d0'],
            ['movew %d0,%d3', 'movew %d1,%d0'], '%d3'),
            ('bmi sub', 'copied', '?'))


def routine(texts):
    """A tools/regalloc.py Routine from instruction texts (2 bytes each,
    from 0x1000), without the image (as tools/tests/test_regalloc.py)."""
    import regalloc
    rt = regalloc.Routine.__new__(regalloc.Routine)
    rt.ins = [regalloc.Insn(0x1000 + 2 * k, 2, t) for k, t in enumerate(texts)]
    rt.r0, rt.r1 = 0x1000, 0x1000 + 2 * len(texts)
    rt.at = {x.addr: i for i, x in enumerate(rt.ins)}
    rt.tails = {}
    word = loopalign.word
    loopalign.word = lambda a: 0
    try:
        rt.cfg()
        rt.epilogue()
        rt.zext()
        rt.liveness()
        rt.result()
        rt.webs()
    finally:
        loopalign.word = word
    return rt


class CounterSiteTest(unittest.TestCase):
    """`counters' (bead domain-pascal-fbah; codegen-patterns 7.4, "The
    count's register")."""

    def test_in_place(self):
        # PROC2_$GET_INFO 0xE40912..0xE40928: the counter in T's D0, the
        # index in D1 (the bound's dead register)
        rt = routine([
            'linkw %fp,#-8', 'movew %fp@(8),%d1', 'moveal %fp@(12),%a0',
            'movew %d1,%d0', 'subqw #1,%d0', 'bmis 0x1018', 'moveq #1,%d1',
            'moveb %a0@(-1,%d1:w),%fp@(-2)', 'addqw #1,%d1', 'dbf %d0,0x100e',
            'nop', 'nop', 'unlk %fp', 'rts'])
        s = loopalign.counter_site(rt, 0x100e, '%d0')
        self.assertEqual((s['where'], s['T'], s['C'], s['I'], s['L'], s['rel']),
                         ('in place', 'd0', 'd0', 'd1', 'd0', 'L=T'))
        self.assertEqual((s['bound'], s['first'], s['iuse']), ('var dies', 'C', 1))

    def test_copied_index_first(self):
        # PROC2_$INFO 0xE40840..0xE40858: the index in T's D0, the counter
        # in D1 above it
        rt = routine([
            'linkw %fp,#-8', 'movew %fp@(8),%d1', 'moveal %fp@(12),%a0',
            'movew %d1,%d0', 'subqw #1,%d0', 'bmis 0x101a', 'movew %d0,%d1',
            'moveq #1,%d0', 'moveb %a0@(-1,%d0:w),%fp@(-2)', 'addqw #1,%d0',
            'dbf %d1,0x1010', 'nop', 'nop', 'unlk %fp', 'rts'])
        s = loopalign.counter_site(rt, 0x1010, '%d1')
        self.assertEqual((s['where'], s['T'], s['C'], s['I'], s['L'], s['rel'], s['holder']),
                         ('copied', 'd0', 'd1', 'd0', 'd1', 'L>T', 'index'))
        self.assertEqual(s['first'], 'I')

    def test_copied_below_t(self):
        # the MAC_OS_ module-local 0xE0B202 at 0xE0B208..0xE0B218: the
        # bound's D1 dies at the copy into T (R2 keeps T out of it), the
        # counter takes it back
        rt = routine([
            'linkw %fp,#-16', 'movel %fp@(8),%d0', 'movew %fp@(16),%d1',
            'movew %d1,%d2', 'subqw #1,%d2', 'bmis 0x101c', 'movew %d2,%d1',
            'clrw %d2', 'moveal %fp@(12),%a0', 'cmpl %a0@(0,%d2:w),%d0',
            'addqw #1,%d2', 'dbf %d1,0x1012', 'nop', 'nop', 'unlk %fp', 'rts'])
        s = loopalign.counter_site(rt, 0x1012, '%d1')
        self.assertEqual((s['where'], s['T'], s['C'], s['I'], s['L'], s['rel']),
                         ('copied', 'd2', 'd1', 'd2', 'd1', 'L<T'))
        self.assertEqual((s['bound'], s['across']), ('var dies', '0'))


if __name__ == '__main__':
    unittest.main()
