"""Tests for tools/moveqsurvey.py's site recognisers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import moveqsurvey  # noqa: E402


def flow(texts, start=0x100):
    """A Flow over TEXTS at 2-byte steps (lengths do not matter here)."""
    return moveqsurvey.Flow([(start + 2 * i, 2, t) for i, t in enumerate(texts)])


class MoveqTest(unittest.TestCase):
    def test_one_store(self):
        # DISK_$PV_MOUNT 0xE6C9F4
        fl = flow(['moveq #18,%d0', 'movel %d0,%fp@(-20)', 'bsrw 0x200', 'unlk %fp', 'rts'])
        self.assertEqual(moveqsurvey.moveq_sites(fl),
                         [(0x100, 18, 'd0', ['store.l frame'], set(), 'd0', 'straight',
                           'alone')])

    def test_one_register_three_stores(self):
        # NETWORK_$INIT 0xE2F7C2..0xE2F7DE
        fl = flow(['moveq #1,%d0', 'moveal #14829820,%a1', 'movel %d0,%a1@(632)',
                   'movel %d0,%a1@(636)', 'movel %d0,%a1@(640)', 'moveq #60,%d1',
                   'addl 0xe2b0d4,%d1', 'movel %d1,%a1@(644)', 'unlk %fp', 'rts'])
        s = moveqsurvey.moveq_sites(fl)
        self.assertEqual(s[0][:5], (0x100, 1, 'd0', ['store.l areg'] * 3, {'rts'}))
        self.assertEqual(s[0][6:], ('straight', 'same-lower/same-lower+same-upper/same-upper'))
        # the second constant is read by the `add' first: not a store's alone
        self.assertEqual(s[1][3], ['other addl'])

    def test_other_use_and_clobber(self):
        # NETWORK_$INIT 0xE2F782: the constant kept for `ec + 1'
        fl = flow(['moveq #1,%d0', 'movel %d0,%a1@(656)', 'clrl %a1@(660)',
                   'addl 0xe248e8,%d0', 'movel %d0,%a1@(664)', 'rts'])
        self.assertEqual(moveqsurvey.moveq_sites(fl)[0][3], ['store.l areg', 'other addl'])
        # a call writes D0 before the store reads it: no use
        fl = flow(['moveq #1,%d0', 'jsr 0xe20000', 'movel %d0,%a0@', 'rts'])
        self.assertEqual(moveqsurvey.moveq_sites(fl), [])
        # D2 survives the call
        fl = flow(['moveq #1,%d2', 'jsr 0xe20000', 'movel %d2,%a0@', 'rts'])
        self.assertEqual(moveqsurvey.moveq_sites(fl)[0][3], ['store.l areg'])

    def test_join(self):
        # TERM_$CONTROL 0xE66C24: tail-merged arms share the store (a
        # branch target between each constant and it)
        fl = flow(['moveq #1,%d0', 'bras 0x106', 'moveq #2,%d0', 'movel %d0,%fp@(-28)',
                   'rts'])
        s = moveqsurvey.moveq_sites(fl)
        self.assertEqual([(x[0], x[6]) for x in s], [(0x100, 'join'), (0x104, 'join')])
        fl = flow(['moveq #2,%d0', 'movel %d0,%fp@(-28)', 'bras 0x100'])
        self.assertEqual(moveqsurvey.moveq_sites(fl)[0][6], 'straight')

    def test_join_backward(self):
        # TERM_$CONTROL 0xE66C24 `moveq #2,D0; bra.s 0xE66C10': the shared
        # store comes before the constant
        fl = flow(['movel %d0,%fp@(-28)', 'rts', 'moveq #2,%d0', 'bras 0x100'])
        self.assertEqual(moveqsurvey.moveq_sites(fl)[0][6], 'join')

    def test_previous_statement_store(self):
        # EC2_$INIT_S 0xE309C6..0xE309CE: the word above stored first, then
        # `moveq #1,D0; move.l D0,(3736,A1)' (the moveq between the two)
        fl = flow(['clrw %a1@(2692)', 'movew #1,%a1@(3740)', 'moveq #1,%d0',
                   'movel %d0,%a1@(3736)', 'rts'])
        self.assertEqual(moveqsurvey.moveq_sites(fl)[0][7], 'desc')
        # a constant just below, before the moveq: `upper'
        fl = flow(['clrl %a1@(3732)', 'moveq #1,%d0', 'movel %d0,%a1@(3736)', 'rts'])
        self.assertEqual(moveqsurvey.moveq_sites(fl)[0][7], 'upper')
        # a load into a data register between: another statement
        fl = flow(['clrl %a1@(3732)', 'movel %a0@,%d1', 'moveq #1,%d0',
                   'movel %d0,%a1@(3736)', 'rts'])
        self.assertEqual(moveqsurvey.moveq_sites(fl)[0][7], 'alone')

    def test_index_and_push(self):
        fl = flow(['moveq #1,%d0', 'movel %d1,%a5@(56,%d0:l)', 'movel %d0,%sp@-',
                   'jsr 0xe20000', 'rts'])
        self.assertEqual(moveqsurvey.moveq_sites(fl)[0][3], ['other index', 'push.l'])


class ImmTest(unittest.TestCase):
    def test_runs(self):
        # MAC_OS_$INIT 0xE2F55E, MST_$INIT 0xE30B98, a word alone, a large one
        fl = flow(['movel #1,%a1@(2204)', 'clrw %a1@(2208)', 'clrl 0xe24384',
                   'movel #1,0xe24388', 'movew #-1,%a0@(4)', 'movel #65536,%fp@(-4)',
                   'rts'])
        got = [(a, v, k, r) for a, v, k, r, _free in moveqsurvey.imm_sites(fl, 'l')]
        self.assertEqual(got, [(0x100, 1, 'areg', 'lower'), (0x106, 1, 'abs', 'upper'),
                               (0x10a, 65536, 'frame', 'alone')])
        self.assertEqual([s[:3] for s in moveqsurvey.imm_sites(fl, 'w')],
                         [(0x108, -1, 'areg')])

    def test_descending_and_pushes(self):
        # TTY_$I_INIT 0xE3326E..0xE33274: the constant above, stored first
        fl = flow(['movel #35,%a0@(28)', 'clrl %a0@(24)', 'movel #7,%sp@-', 'rts'])
        self.assertEqual([s[3] for s in moveqsurvey.imm_sites(fl, 'l')], ['desc'])

    def test_free_registers(self):
        fl = flow(['movel %fp@(8),%d1', 'movel #3,%a0@', 'movel %d1,%a0@(4)', 'rts'])
        free = moveqsurvey.imm_sites(fl, 'l')[0][4]
        self.assertNotIn('d1', free)
        self.assertIn('d0', free)


class ChainTest(unittest.TestCase):
    def test_tty_run(self):
        # TTY_$I_INIT 0xE33262..0xE33280
        fl = flow(['moveq #41,%d0', 'movel %d0,%a0@(20)', 'moveq #2,%d1',
                   'movel %d1,%a0@(12)', 'moveq #35,%d2', 'movel %d2,%a0@(28)',
                   'clrl %a0@(24)', 'moveq #17,%d0', 'clrw %d1', 'addqw #1,%d1',
                   'dbf %d0,0x112', 'rts'])
        got = [(a, r, v, pa, pr) for a, r, v, _u, pa, pr, _pv in moveqsurvey.chain_pairs(fl)]
        self.assertEqual(got, [(0x104, 'd1', 2, 0x100, 'd0'), (0x108, 'd2', 35, 0x104, 'd1'),
                               (0x10e, 'd0', 17, 0x108, 'd2')])

    def test_broken_by_a_branch(self):
        fl = flow(['moveq #1,%d0', 'movel %d0,%a0@', 'beqs 0x10a', 'moveq #2,%d1',
                   'movel %d1,%a0@(4)', 'rts'])
        self.assertEqual(moveqsurvey.chain_pairs(fl), [])


class HelpersTest(unittest.TestCase):
    def test_signed_and_base(self):
        self.assertEqual(moveqsurvey.signed(0xffff, 'w'), -1)
        self.assertEqual(moveqsurvey.signed(255, 'b'), -1)
        self.assertEqual(moveqsurvey.signed(7, 'l'), 7)
        self.assertEqual(moveqsurvey.base_off('%a1@(2204)'), ('%a1@', 2204))
        self.assertEqual(moveqsurvey.base_off('%a3@'), ('%a3@', 0))
        self.assertEqual(moveqsurvey.base_off('0xe24388'), ('abs', 0xe24388))
        self.assertIsNone(moveqsurvey.base_off('%a5@(56,%d0:l)'))


if __name__ == '__main__':
    unittest.main()
