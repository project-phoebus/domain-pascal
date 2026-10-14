"""Tests for tools/rhssurvey.py (bead domain-pascal-u6di; no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import regalloc  # noqa: E402
import rhssurvey  # noqa: E402


def routine(texts):
    """A Routine from instruction texts (2 bytes each, from 0x1000)."""
    rt = regalloc.Routine.__new__(regalloc.Routine)
    rt.ins = [regalloc.Insn(0x1000 + 2 * k, 2, t) for k, t in enumerate(texts)]
    rt.r0, rt.r1 = 0x1000, 0x1000 + 2 * len(texts)
    rt.at = {x.addr: i for i, x in enumerate(rt.ins)}
    rt.tails = {}
    word = regalloc.loopalign.word
    regalloc.loopalign.word = lambda a: 0
    try:
        rt.cfg()
        rt.epilogue()
        rt.zext()
        rt.liveness()
        rt.result()
        rt.webs()
    finally:
        regalloc.loopalign.word = word
    return rt


def one_pair(rt):
    ps = rhssurvey.pairs(rt)
    assert len(ps) == 1, [(P.reg, L.reg) for P, L, _ in ps]
    return ps[0]


# KBD_$PUT 0xE1CEAC: the status (0x18)->A3 pushed for KBD_$GET_DESC, the
# result kept in A2, the status tested and dead
KBD_PUT = ['linkw %fp,#-12', 'moveal %fp@(24),%a3', 'pea %a3@', 'jsr 0xe1aa26',
           'addqw #8,%sp', 'moveal %a0,%a2', 'tstl %a3@', 'bnes 0x1014',
           'pea %a2@', 'bsrw 0xe1ca8a', 'unlk %fp', 'rts']

# CAL_$SHUTDOWN 0xE3E6E4 (shortened): the same, but the status pushed again
# by the second call
CAL_SHUTDOWN = ['linkw %fp,#-4', 'moveal %fp@(8),%a2', 'subql #2,%sp', 'pea %a2@',
                'clrl %sp@-', 'jsr 0xe3a5b0', 'lea %sp@(24),%sp', 'moveal %a0,%a3',
                'tstl %a2@', 'bnes 0x101e', 'movel %a3@(176),%a3@(230)',
                'subql #2,%sp', 'pea %a2@', 'pea %a3@', 'jsr 0xe3a8b6',
                'unlk %fp', 'rts']


class ClassifyTest(unittest.TestCase):
    def test_push(self):
        ins = regalloc.Insn
        self.assertTrue(rhssurvey.is_push(ins(0, 2, 'pea %a3@')))
        self.assertTrue(rhssurvey.is_push(ins(0, 2, 'movel %d5,%sp@-')))
        self.assertTrue(rhssurvey.is_push(ins(0, 2, 'subql #2,%sp')))
        self.assertFalse(rhssurvey.is_push(ins(0, 2, 'addqw #8,%sp')))
        self.assertTrue(rhssurvey.is_pop(ins(0, 2, 'addqw #8,%sp')))
        self.assertTrue(rhssurvey.is_pop(ins(0, 2, 'lea %sp@(24),%sp')))

    def test_kbd_put_rhs_dies(self):
        rt = routine(KBD_PUT)
        P, L, sw = one_pair(rt)
        self.assertEqual((P.reg, L.reg, sw), ('a3', 'a2', True))
        self.assertEqual(rhssurvey.lkind(rt, L), ('call', 3))
        for mode in ('wide', 'push'):
            self.assertEqual(rhssurvey.classify(rt, P, L, mode), 'rhs')
        self.assertEqual(rhssurvey.fate(rt, P, L), 'dies')

    def test_cal_shutdown_rhs_lives(self):
        rt = routine(CAL_SHUTDOWN)
        P, L, sw = one_pair(rt)
        self.assertEqual((P.reg, L.reg), ('a2', 'a3'))
        self.assertEqual(rhssurvey.stmt_start(rt, 5), 0)       # the block start
        self.assertEqual(rhssurvey.stmt_start(rt, 5, push=True), 2)   # the pad
        self.assertEqual(rhssurvey.classify(rt, P, L, 'wide'), 'rhs')
        self.assertEqual(rhssurvey.fate(rt, P, L), 'lives')

    def test_before_and_after(self):
        before = ['linkw %fp,#-4', 'moveal %fp@(8),%a3', 'tstl %a3@', 'beqs 0x1016',
                  'pea %a3@', 'jsr 0x2000', 'addqw #4,%sp', 'moveal %a0,%a2',
                  'tstl %a3@', 'pea %a2@', 'jsr 0x2000', 'unlk %fp', 'rts']
        rt = routine(before)
        P, L, _ = one_pair(rt)
        self.assertEqual(rhssurvey.classify(rt, P, L, 'wide'), 'before')
        # AUDIT_ 0xE712BA's (8): loaded at the entry, first read after the
        # assignment
        after = ['linkw %fp,#-4', 'moveal %fp@(8),%a3', 'clrl %sp@-', 'jsr 0x2000',
                 'addqw #4,%sp', 'moveal %a0,%a2', 'pea %a3@', 'pea %a2@',
                 'jsr 0x2000', 'unlk %fp', 'rts']
        rt = routine(after)
        P, L, _ = one_pair(rt)
        self.assertEqual(rhssurvey.classify(rt, P, L, 'wide'), 'after')

    def test_previous_call_belongs_to_its_statement(self):
        rt = routine(['linkw %fp,#-4', 'moveal %fp@(8),%a3', 'jsr 0x2000',
                      'addqw #4,%sp', 'moveal %a0,%a2', 'pea %a3@', 'jsr 0x2000',
                      'pea %a2@', 'jsr 0x2000', 'unlk %fp', 'rts'])
        self.assertEqual(rhssurvey.stmt_start(rt, 6), 5)

    def test_noncall_rhs(self):
        # L := P^.field right after the entry load (0xE17D4E/0xE17D52)
        rt = routine(['linkw %fp,#-4', 'moveal %fp@(8),%a2', 'moveal %a2@(32),%a3',
                      'pea %a3@', 'jsr 0x2000', 'tstl %a2@', 'unlk %fp', 'rts'])
        P, L, _ = one_pair(rt)
        self.assertEqual(rhssurvey.lkind(rt, L), ('load', None))
        self.assertEqual(rhssurvey.classify(rt, P, L, 'def'), 'rhs')
        self.assertEqual(rhssurvey.classify(rt, P, L, 'wide'), 'rhs')
        self.assertEqual(rhssurvey.derivation(rt, P, L), 'derived')

    def test_indep(self):
        rt = routine(KBD_PUT)
        P, L, _ = one_pair(rt)
        self.assertEqual(rhssurvey.derivation(rt, P, L), 'indep')

    def test_binom(self):
        self.assertAlmostEqual(rhssurvey.binom_tail(0, 5, 0.3), 1.0)
        self.assertAlmostEqual(rhssurvey.binom_tail(2, 2, 0.5), 0.25)


if __name__ == '__main__':
    unittest.main()
