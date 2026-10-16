"""Tests for tools/optsurvey.py (no image needed)."""

import os
import random
import sys
import unittest
from collections import Counter

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import regalloc  # noqa: E402
import optsurvey  # noqa: E402


def routine(texts, start=0x100):
    """A regalloc.Routine built from instruction texts, 2 bytes each (branch
    targets are written as addresses: start + 2 * index)."""
    rt = regalloc.Routine.__new__(regalloc.Routine)
    rt.r0, rt.r1 = start, start + 2 * len(texts)
    rt.ins = [regalloc.Insn(start + 2 * i, 2, t) for i, t in enumerate(texts)]
    rt.at = {x.addr: i for i, x in enumerate(rt.ins)}
    rt.tails = {}
    rt.cfg()
    rt.liveness()
    return rt


def a(i, start=0x100):
    return '0x%x' % (start + 2 * i)


class HelpersTest(unittest.TestCase):
    def test_base_and_size(self):
        self.assertEqual(optsurvey.base_of('movew'), 'move')
        self.assertEqual(optsurvey.base_of('moveal'), 'movea')
        self.assertEqual(optsurvey.base_of('muluw'), 'mulu')
        self.assertEqual(optsurvey.base_of('spl'), 'spl')       # Scc, not `sp' + l
        self.assertEqual(optsurvey.base_of('lea'), 'lea')
        self.assertEqual(optsurvey.size_of('clrb'), 1)
        self.assertEqual(optsurvey.size_of('sne'), 1)
        self.assertEqual(optsurvey.size_of('moveq'), 4)
        self.assertEqual(optsurvey.size_of('bset'), 1)

    def test_fixed_loc(self):
        self.assertEqual(optsurvey.fixed_loc('%fp@(-4)'), ('fp', -4))
        self.assertEqual(optsurvey.fixed_loc('%a5@(12)'), ('a5', 12))
        self.assertEqual(optsurvey.fixed_loc('%a5@'), ('a5', 0))
        self.assertEqual(optsurvey.fixed_loc('0xe2060a'), ('abs', 0xE2060A))
        self.assertIsNone(optsurvey.fixed_loc('%a0@(4)'))

    def test_access(self):
        x = regalloc.Insn(0, 2, 'addqw #1,%fp@(-2)')
        self.assertEqual(optsurvey.access(x), [('%fp@(-2)', 'rw')])
        x = regalloc.Insn(0, 2, 'movew %fp@(8),%a0@(4)')
        self.assertEqual(optsurvey.access(x), [('%fp@(8)', 'r'), ('%a0@(4)', 'w')])
        x = regalloc.Insn(0, 2, 'pea %fp@(-8)')
        self.assertEqual(optsurvey.access(x), [('%fp@(-8)', 'a')])
        x = regalloc.Insn(0, 2, 'movew %d0,%sp@-')
        self.assertEqual(optsurvey.access(x), [])          # a push

    def test_lin(self):
        x = ('m', 'w', 'FP+8')
        k = lambda n: ('k', n)                              # noqa: E731
        shl = ('op', 'shl', 'l', x, k(2))                   # x*4
        self.assertEqual(optsurvey.lin(shl), (x, 4))
        # x*12 as at FIM_$ACKNOWLEDGE 0xE0A972: (x*4)*2 + x*4
        e = ('op', 'add', 'l', ('op', 'shl', 'l', shl, k(1)), shl)
        self.assertEqual(optsurvey.lin(e), (x, 12))
        self.assertIsNone(optsurvey.lin(('op', 'add', 'l', x, ('m', 'w', 'FP+10'))))

    def test_is_step(self):
        ins = lambda t: regalloc.Insn(0, 2, t)              # noqa: E731
        self.assertTrue(optsurvey.is_step(ins('addql #4,%a2')))
        self.assertFalse(optsurvey.is_step(ins('addql #1,%a2')))   # a byte array
        self.assertTrue(optsurvey.is_step(ins('lea %a1@(12),%a1')))
        self.assertFalse(optsurvey.is_step(ins('lea %a1@(12),%a0')))
        self.assertTrue(optsurvey.is_step(ins('addql #4,%fp@(-120)')))  # PMAP_$FLUSH 0xE13984
        self.assertTrue(optsurvey.is_step(ins('movel %a1@+,%d0')))
        self.assertFalse(optsurvey.is_step(ins('moveb %a1@+,%d0')))

    def test_is_abs_addr(self):
        ins = lambda t: regalloc.Insn(0, 2, t)              # noqa: E731
        self.assertTrue(optsurvey.is_abs_addr(ins('moveal #15419392,%a0')))   # 0xEB4800
        self.assertTrue(optsurvey.is_abs_addr(ins('lea 0xe2451c,%a0')))
        self.assertFalse(optsurvey.is_abs_addr(ins('movel #544,%d6')))

    def test_movem_count(self):
        self.assertEqual(optsurvey.movem_count('%d2-%d5/%a2-%a3'), 6)
        self.assertEqual(optsurvey.movem_count('%d2/%a5'), 2)


class FrameTest(unittest.TestCase):
    def test_dead_store(self):
        # FIM_$GET_USER_PC 0xE0AAA6: a call's result stored to a local that
        # nothing reads, the value used from D0
        rt = routine(['linkw %fp,#-8', 'jsr 0xe20f0c', 'movel %d0,%fp@(-4)',
                      'moveal %d0,%a0', 'movel %a0@,%d0', 'unlk %fp', 'rts'])
        self.assertEqual(optsurvey.dead_stores(rt), [0x104])

    def test_no_dead_store_when_read_or_escaped(self):
        rt = routine(['linkw %fp,#-8', 'movel %d0,%fp@(-4)', 'movew %fp@(-2),%d1',
                      'unlk %fp', 'rts'])
        self.assertEqual(optsurvey.dead_stores(rt), [])     # a part read
        rt = routine(['linkw %fp,#-8', 'movel %d0,%fp@(-4)', 'pea %fp@(-8)',
                      'jsr 0xe20f0c', 'unlk %fp', 'rts'])
        self.assertEqual(optsurvey.dead_stores(rt), [])     # the frame escapes
        rt = routine(['linkw %fp,#-8', 'movel %d0,%fp@(-4)', 'moveal %fp,%a1',
                      'bsrw 0x200', 'unlk %fp', 'rts'])
        self.assertEqual(optsurvey.dead_stores(rt), [])     # a static link

    def test_overwritten_store(self):
        rt = routine(['linkw %fp,#-8', 'movew #1,%fp@(-2)', 'movew %d0,%fp@(-2)',
                      'movew %fp@(-2),%sp@-', 'unlk %fp', 'rts'])
        self.assertEqual(optsurvey.overwritten_stores(rt), [0x102])
        rt = routine(['linkw %fp,#-8', 'movew #1,%fp@(-2)', 'tstw %fp@(-2)',
                      'movew %d0,%fp@(-2)', 'unlk %fp', 'rts'])
        self.assertEqual(optsurvey.overwritten_stores(rt), [])

    def test_block_reloads(self):
        # AREA_ 0xE085DA..0xE085E8: a store, then a reload of the slot
        # while D1 still holds the value
        rt = routine(['linkw %fp,#-200', 'movew %d1,%fp@(-166)', 'movel %fp@(14),%d2',
                      'movew %fp@(-166),%d3', 'unlk %fp', 'rts'])
        c, s = optsurvey.block_reloads(rt)
        self.assertEqual(c['streload_fp'], 1)
        self.assertEqual(s['streload_fp'], [0x106])
        # AST_$ACTIVATE_AND_WIRE 0xE02FEE: a cleared local pushed from its slot
        rt = routine(['linkw %fp,#-4', 'clrl %fp@(-4)', 'subql #2,%sp', 'clrw %sp@-',
                      'movel %fp@(-4),%sp@-', 'unlk %fp', 'rts'])
        c, _ = optsurvey.block_reloads(rt)
        self.assertEqual(c['kreread_fp'], 1)
        # a call in between: the scratch register no longer holds it
        rt = routine(['linkw %fp,#-4', 'movew %d0,%fp@(-2)', 'jsr 0xe20f0c',
                      'movew %fp@(-2),%d1', 'unlk %fp', 'rts'])
        c, _ = optsurvey.block_reloads(rt)
        self.assertEqual(c['streload_fp'], 0)
        # a store through a pointer forgets the unit's data, not the frame
        rt = routine(['linkw %fp,#-4', 'movew %a5@(8),%d2', 'movew %d3,%a0@',
                      'movew %a5@(8),%d4', 'unlk %fp', 'rts'])
        c, _ = optsurvey.block_reloads(rt)
        self.assertEqual(c['ldreload_a5'], 0)


class FlowTest(unittest.TestCase):
    def test_dead_bra(self):
        rt = routine(['linkw %fp,#-4', 'beqs ' + a(4), 'clrw %d0', 'bras ' + a(5),
                      'bras ' + a(5), 'unlk %fp', 'rts'])
        # index 4 is reached from the beq: not dead
        self.assertEqual(optsurvey.dead_bras(rt), [])
        rt = routine(['linkw %fp,#-4', 'beqs ' + a(3), 'bras ' + a(4),
                      'bras ' + a(4), 'unlk %fp', 'rts'])
        self.assertEqual(optsurvey.dead_bras(rt), [])
        rt = routine(['linkw %fp,#-4', 'clrw %d0', 'bras ' + a(4),
                      'bras ' + a(4), 'unlk %fp', 'rts'])
        self.assertEqual(optsurvey.dead_bras(rt), [0x106])

    def test_join_tails(self):
        # TERM_$GET_REAL_LINE 0xE1A9F0..0xE1AA06: the same load before a
        # `bra' to the join and before falling into it
        rt = routine(['linkw %fp,#-4', 'tstw %d1', 'beqs ' + a(5),
                      'movew 0xe2e00e,%d0', 'bras ' + a(6),
                      'movew 0xe2e00e,%d0', 'unlk %fp', 'rts'])
        dup, xj = optsurvey.join_tails(rt)
        self.assertEqual(dup, [0x10A])
        self.assertEqual(xj, [])
        # 0xE06156: a call's pop shared at a join (`bsr; bra' to the pop)
        rt = routine(['linkw %fp,#-4', 'tstw %d1', 'beqs ' + a(6), 'movel %d0,%sp@-',
                      'bsrw 0x300', 'bras ' + a(8), 'movel %d1,%sp@-', 'bsrw 0x320',
                      'addqw #4,%sp', 'unlk %fp', 'rts'])
        dup, xj = optsurvey.join_tails(rt)
        self.assertEqual(xj, [0x10A])


class LoopTest(unittest.TestCase):
    def test_while_loop_head_is_the_test(self):
        # SOCK_$OPEN 0xE15E00..0xE15E0C: entered by a `bra' to the test
        rt = routine(['linkw %fp,#-4', 'lea %a5@,%a0', 'bras ' + a(4),
                      'moveal %a0@(12),%a0', 'cmpal %a0@(12),%a2', 'bnes ' + a(3),
                      'unlk %fp', 'rts'])
        loops = optsurvey.natural_loops(rt)
        self.assertEqual([(rt.ins[h].addr, len(b)) for h, _, b in loops], [(0x108, 3)])

    def test_strength_reduction_missed_for_offset_index(self):
        # FILE_$FW_PAGES 0xE5E7AA: the source stepped, the destination
        # indexed by (i - lo) * 4 every iteration
        rt = routine(['linkw %fp,#-200', 'moveq #9,%d2', 'moveq #1,%d3',
                      'movew %d3,%d4', 'subw %d6,%d4', 'lslw #2,%d4',
                      'movel %a1@(-4),%fp@(-128,%d4:w)', 'addqw #1,%d3', 'addql #4,%a1',
                      'dbf %d2,' + a(3), 'unlk %fp', 'rts'])
        c, s = optsurvey.loop_features(rt)
        self.assertEqual((c['loops'], c['srmiss'], c['srstep']), (1, 1, 1))

    def test_redefined_register_is_not_scaled_index(self):
        # AST_$TOUCH 0xE03464/0xE03486: D0 copies the index, then is loaded
        # with another value before the shift
        ind = {'d5'}
        ins = [regalloc.Insn(0, 2, t) for t in (
            'moveb %d5,%d0', 'movel %d4,%d0', 'lsll #2,%d0', 'movew %a1@(2,%d0:l),%d1',
            'addqw #1,%d5')]
        self.assertFalse(optsurvey.scaled_index_use(ins, ind))
        ins = [regalloc.Insn(0, 2, t) for t in (
            'movew %d5,%d0', 'addw %d0,%d0', 'movew %a1@(0,%d0:w),%d1')]
        self.assertTrue(optsurvey.scaled_index_use(ins, ind))

    def test_wrapping_index_is_no_induction(self):
        # BAT_$ALLOC_FM 0xE3ADBA..0xE3ADCA: the index wraps (`clr.w D1')
        rt = routine(['linkw %fp,#-4', 'moveq #9,%d2', 'clrw %d1', 'movew %d1,%d0',
                      'lsll #3,%d0', 'tstb %a1@(0,%d0:l)', 'addqw #1,%d1',
                      'cmpw %d3,%d1', 'blts ' + a(10), 'clrw %d1', 'dbf %d2,' + a(3),
                      'unlk %fp', 'rts'])
        (h, _, body), = optsurvey.natural_loops(rt)
        ins = [rt.ins[i] for i in sorted(body)]
        self.assertEqual(optsurvey.induction_regs(rt, h, ins), {'d2'})

    def test_pure_steps(self):
        ins = lambda ts: [regalloc.Insn(0, 2, t) for t in ts]   # noqa: E731
        # a pointer stepped and defined in no other way
        self.assertEqual(optsurvey.pure_steps(ins(['movel %a1@+,%d0', 'addql #4,%a2',
                                                   'movew %d0,%a2@'])), {'a1', 'a2'})
        # a pointer the source also assigns in the loop (`p := p^.next')
        self.assertEqual(optsurvey.pure_steps(ins(['addql #4,%a2', 'moveal %a2@(8),%a2'])),
                         set())
        # a pointer held in the frame (PMAP_$FLUSH 0xE13984)
        self.assertEqual(optsurvey.pure_steps(ins(['addql #4,%fp@(-120)'])), {'%fp@(-120)'})
        self.assertEqual(optsurvey.pure_steps(ins(['addql #4,%fp@(-120)',
                                                   'movel %a0,%fp@(-120)'])), set())

    def test_invaddr_call_free(self):
        rt = routine(['linkw %fp,#-8', 'moveq #9,%d1', 'moveal #15419392,%a0',
                      'clrw %a0@(0,%d1:w)', 'dbf %d1,' + a(2), 'unlk %fp', 'rts'])
        c, _ = optsurvey.loop_features(rt)
        self.assertEqual((c['invaddr'], c['invaddr_nc']), (1, 1))
        rt = routine(['linkw %fp,#-8', 'moveq #9,%d1', 'moveal #15419392,%a0',
                      'jsr 0xe20f0c', 'dbf %d1,' + a(2), 'unlk %fp', 'rts'])
        c, _ = optsurvey.loop_features(rt)
        self.assertEqual((c['invaddr'], c['invaddr_nc']), (1, 0))

    def test_invariant_load(self):
        # EC2_$REGISTER_EC1 0xE4296E: a parameter slot loaded in the loop
        rt = routine(['linkw %fp,#-8', 'moveq #9,%d1', 'addql #4,%a0',
                      'moveal %fp@(8),%a1', 'cmpal %a0@(2700),%a1', 'dbeq %d1,' + a(2),
                      'unlk %fp', 'rts'])
        c, s = optsurvey.loop_features(rt)
        self.assertEqual((c['invld'], c['invld_nc']), (1, 1))
        self.assertEqual(s['invld'], [0x106])
        # written in the loop: not invariant
        rt = routine(['linkw %fp,#-8', 'moveq #9,%d1', 'moveal %fp@(-4),%a1',
                      'addql #4,%fp@(-4)', 'tstw %a1@', 'dbeq %d1,' + a(2),
                      'unlk %fp', 'rts'])
        c, _ = optsurvey.loop_features(rt)
        self.assertEqual((c['invld'], c['lfrm'], c['srstep']), (0, 1, 1))


class SymTest(unittest.TestCase):
    def test_reread_and_product(self):
        rt = routine(['linkw %fp,#-4', 'movew %fp@(8),%d0', 'movew %d0,%a5@(4)',
                      'movew %fp@(8),%d1', 'movew %d1,%d2', 'lslw #2,%d1',
                      'addw %d2,%d1', 'movew %d1,%a5@(6)', 'unlk %fp', 'rts'])
        sy = optsurvey.OptSym(rt, False)
        kinds = Counter(t for t, _, _ in sy.extra)
        self.assertEqual(kinds['reread'], 1)                 # (8,A6) still in D0
        self.assertEqual(optsurvey.product_sites(sy), [(0x10C, 5)])

    def test_constant_operator(self):
        # DISK_$PV_ASSIGN 0xE6C98E..0xE6C994: moveq #5; ori.w #2 on one path
        rt = routine(['linkw %fp,#-4', 'moveq #5,%d2', 'oriw #2,%d2', 'movew %d2,%a5@(4)',
                      'addil #0,%d3', 'unlk %fp', 'rts'])
        sy = optsurvey.OptSym(rt, False)
        self.assertEqual([rt.ins[i].addr for t, i, _ in sy.extra if t == 'kfold'], [0x104])


class StatsTest(unittest.TestCase):
    def test_fisher(self):
        self.assertAlmostEqual(optsurvey.fisher(3, 0, 0, 3), 0.1, places=6)
        self.assertAlmostEqual(optsurvey.fisher(1, 1, 1, 1), 1.0, places=6)

    def test_concentration_and_pearson(self):
        rng = random.Random(1)
        groups = [[True] * 5, [False] * 5, [True] * 5, [False] * 5]
        obs, mean, p = optsurvey.concentration(groups, rng, 200)
        self.assertEqual(obs, 0)
        self.assertGreater(mean, 2)
        self.assertLess(p, 0.05)
        self.assertAlmostEqual(optsurvey.pearson([0, 1, 2], [0, 2, 4]), 1.0)
        self.assertEqual(optsurvey.pearson([1, 1], [0, 1]), 0.0)

    def test_tell_class(self):
        self.assertEqual(optsurvey.tell_class(Counter(cse=1)), 'opt1+')
        self.assertEqual(optsurvey.tell_class(Counter(srstep=2, keep=3)), 'opt1+')
        self.assertEqual(optsurvey.tell_class(Counter(keep=3)), 'opt0')
        self.assertEqual(optsurvey.tell_class(Counter(xjpop=1)), 'opt0')
        self.assertEqual(optsurvey.tell_class(Counter()), 'none')

    def test_a5_table(self):
        def row(seg, segaddr, r0, name, a5, **c):
            r = optsurvey.Row(seg, segaddr, r0, r0 + 0x20, name)
            r.a5 = a5
            r.c.update(c)
            return r
        rows = [row('A', 0x100, 0x100, 'A_$X', 'S', merge=1),
                row('A', 0x100, 0x120, 'A_$Y', 'n'),
                row('B', 0x200, 0x200, 'B_$X', 's', apart=1),
                row('B', 0x200, 0x220, 'B_$Y', 'n'),
                row('C', 0x300, 0x300, 'C_$X', 'n', apart=2)]   # loads no A5
        t = optsurvey.a5_table(rows)
        self.assertEqual(t, Counter({('merge', 'used'): 1, ('apart', 'mixed'): 1,
                                     ('apart', '-'): 1}))

    def test_flags(self):
        s = {'c': Counter(merge=1, apart=2, loops=3, invld=1), 'n': 3, 'mod': 3,
             'align': Counter(al=1, na=2), 'a5': Counter(S=1, n=2),
             'a5named': Counter(S=1, n=2), 'a5seg': True}
        f = optsurvey.flags(s)
        self.assertEqual((f['merges'], f['aligns'], f['a5always'], f['invld'], f['srmiss']),
                         (True, True, False, True, False))
        f = optsurvey.flags({'c': Counter(), 'n': 1, 'mod': 0, 'align': Counter({'?': 1}),
                             'a5': Counter(n=1), 'a5named': Counter(), 'a5seg': False})
        self.assertEqual((f['merges'], f['aligns'], f['a5always'], f['deadst']),
                         (None, None, None, None))


if __name__ == '__main__':
    unittest.main()
