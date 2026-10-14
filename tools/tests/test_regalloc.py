"""Tests for tools/regalloc.py's instruction model and rule names (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import regalloc  # noqa: E402


def insn(text):
    return regalloc.Insn(0x1000, 2, text)


class InsnModelTest(unittest.TestCase):
    def test_moves(self):
        x = insn('movew %fp@(12),%d0')
        self.assertEqual((x.uses, x.defs, x.partial), ({'a6'}, {'d0'}, {'d0'}))
        x = insn('moveal %d3,%a1')
        self.assertEqual((x.uses, x.defs, x.partial), ({'d3'}, {'a1'}, set()))
        x = insn('movew %d1,%a5@(0,%d2:w)')
        self.assertEqual((x.uses, x.defs), ({'d1', 'a5', 'd2'}, set()))

    def test_autoincrement_writes_the_register(self):
        x = insn('movel %a0@+,%a1@+')
        self.assertEqual(x.uses, {'a0', 'a1'})
        self.assertEqual(x.defs, {'a0', 'a1'})

    def test_two_address_and_clear_idiom(self):
        x = insn('addl %d1,%d2')
        self.assertEqual((x.uses, x.defs), ({'d1', 'd2'}, {'d2'}))
        x = insn('subal %a0,%a0')            # a clear, not a read of A0
        self.assertEqual((x.uses, x.defs), (set(), {'a0'}))

    def test_control(self):
        x = insn('dbf %d0,0xe177e8')
        self.assertEqual((x.kind, x.target, x.uses, x.defs), ('db', 0xE177E8, {'d0'}, {'d0'}))
        x = insn('blss 0xe177cc')
        self.assertEqual((x.kind, x.target), ('bcc', 0xE177CC))
        x = insn('jsr 0xe20b12')
        self.assertTrue(x.call)
        x = insn('jmp %pc@(0x10,%d0:w)')
        self.assertEqual(x.kind, 'ret')       # computed jump: routine skipped
        self.assertTrue(x.bad)

    def test_sr_moves(self):
        x = insn('movew %sr,%d1')
        self.assertEqual((x.uses, x.defs), (set(), {'d1'}))
        x = insn('movew %d0,%sr')
        self.assertEqual((x.uses, x.defs), ({'d0'}, set()))


def routine(texts):
    """A Routine built from instruction texts (2 bytes each, from 0x1000),
    without the image."""
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


class UpdateTest(unittest.TestCase):
    def test_updates_vs_new_values(self):
        self.assertEqual(insn('addl %d1,%d2').upd, {'d2'})
        x = insn('movel %a0@+,%a1@+')
        self.assertEqual((x.upd, x.incdec), ({'a0', 'a1'}, {'a0', 'a1'}))
        self.assertEqual(insn('lea %a1@(16),%a1').upd, {'a1'})   # adda #16,A1
        self.assertEqual(insn('dbf %d0,0x1000').upd, {'d0'})
        # a load through A0 into A0 is a new value, not an update
        x = insn('moveal %a0@(4),%a0')
        self.assertEqual((x.uses, x.defs, x.upd), ({'a0'}, {'a0'}, set()))
        self.assertEqual(insn('movew %a0@(2,%d1:w),%d1').upd, set())

    def test_coalesced_load_is_its_own_value(self):
        rt = routine(['linkw %fp,#0', 'moveal %fp@(8),%a0', 'moveal %a0@(4),%a0',
                      'movew %a0@,%d0', 'unlk %fp', 'rts'])
        self.assertEqual([rt.ins[v.first].text for v in rt.values if v.reg == 'a0'],
                         ['moveal %fp@(8),%a0', 'moveal %a0@(4),%a0'])

    def test_dead_postincrement_is_not_a_result(self):
        # the last `(A0)+' of a block copy leaves A0 dead: not a result
        rt = routine(['linkw %fp,#0', 'lea %a5@,%a0', 'moveal %fp@(8),%a1',
                      'movel %a0@+,%a1@+', 'movel %a0@+,%a1@+', 'unlk %fp', 'rts'])
        self.assertEqual(rt.result_regs, set())
        rt = routine(['linkw %fp,#0', 'moveal %fp@(8),%a0', 'unlk %fp', 'rts'])
        self.assertEqual(rt.result_regs, {'a0'})

    def test_address_order_rule(self):
        # (8) in D1 and (12) in D0 overlap up to the add: colU (lowest
        # register free of every other value) accepts D1 for the first
        # load; colQU (greedy in address order) wants it in D0.
        rt = routine(['linkw %fp,#0', 'movel %fp@(8),%d1', 'movel %fp@(12),%d0',
                      'addl %d1,%d0', 'unlk %fp', 'rts'])
        res = {rt.ins[v.first].text: p for v, p, f in regalloc.analyse(rt)}
        p = res['movel %fp@(8),%d1']
        self.assertEqual((p['colU'], p['colQU']), ('d1', 'd0'))


class CallArgsTest(unittest.TestCase):
    def test_arg_call(self):
        # PROC1_$SET_ASID 0xE148F8: the parameter's last use is the push
        # of the call's argument; the call statement starts after the store.
        rt = routine(['linkw %fp,#0', 'movel %d2,%sp@-', 'movew %fp@(8),%d2',
                      'moveal 0xe1eac8,%a0', 'movew %d2,%a0@(70)', 'subql #2,%sp',
                      'movew %d2,%sp@-', 'jsr 0xe24204', 'movel %fp@(-4),%d2',
                      'unlk %fp', 'rts'])
        v = next(v for v in rt.values if v.reg == 'd2')
        self.assertEqual(regalloc.arg_call(rt, v), (7, 5))
        # A0's last use is the store: not an argument.
        a = next(v for v in rt.values if v.reg == 'a0')
        self.assertIsNone(regalloc.arg_call(rt, a))


class SlotRuleTest(unittest.TestCase):
    """12.4 R4(b) (bead domain-pascal-178): `slots'."""

    def preds(self, texts):
        rt = routine(texts)
        dirty = {k: regalloc.dirty_sets(rt, *a) for k, a in regalloc.DIRTY_VARIANTS.items()}
        written = regalloc.written_sets(rt)
        stack = regalloc.free_stack(rt)
        return rt, {rt.ins[v.first].text: regalloc.slot_predictions(rt, v, dirty, written, stack)
                    for v in regalloc.slot_values(rt)}

    def test_fresh_in_straight_code(self):
        # IIC_$RECEIVE 0xE70AC0..0xE70AD0: A0, A1, then A2
        rt, p = self.preds(['linkw %fp,#0', 'moveal %fp@(8),%a0', 'clrl %a0@',
                            'moveal %fp@(12),%a1', 'clrw %a1@',
                            'moveal %fp@(16),%a2', 'clrw %a2@', 'unlk %fp', 'rts'])
        q = p['moveal %fp@(16),%a2']
        self.assertEqual((q['colU'], q['yc'], q['lc']), ('a0', 'a2', 'a2'))
        self.assertEqual(q['dpc'], 'refused')      # A0 dirty: not dpc's shape

    def test_after_a_call(self):
        # A0 written, then a call: A0 and A1 clean, A1 never written: A0
        rt, p = self.preds(['linkw %fp,#0', 'moveal %fp@(8),%a0', 'movew %a0@,%sp@-',
                            'jsr 0xe20b12', 'moveal %fp@(12),%a0', 'clrl %a0@',
                            'unlk %fp', 'rts'])
        self.assertEqual(p['moveal %fp@(12),%a0']['dpc'], 'a0')
        # A1 written before the call: A0 or A1 in the image, refused
        rt, p = self.preds(['linkw %fp,#0', 'moveal %fp@(8),%a0', 'moveal %fp@(16),%a1',
                            'movew %a0@,%a1@', 'jsr 0xe20b12', 'moveal %fp@(12),%a0',
                            'clrl %a0@', 'unlk %fp', 'rts'])
        self.assertEqual(p['moveal %fp@(12),%a0']['dpc'], 'refused')

    def test_restarted_at_a_call(self):
        # PROC2_$PGUID_TO_UPGID 0xE410B2/0xE410B8: A0 then A1 after the call
        rt, p = self.preds(['linkw %fp,#0', 'movew #4,%sp@-', 'jsr 0xe20b62',
                            'moveal %fp@(12),%a0', 'movew %d2,%a0@',
                            'moveal %fp@(16),%a1', 'clrl %a1@', 'unlk %fp', 'rts'])
        q = p['moveal %fp@(16),%a1']
        self.assertEqual((q['colU'], q['dpc']), ('a0', 'a1'))
        self.assertEqual(regalloc.since_call(rt, rt.at[0x100a]), {'a0'})

    def test_dirty_on_one_path(self):
        # A0 written on one path only: dirty for 'any', clean for 'all'
        rt = routine(['linkw %fp,#0', 'tstw %d0', 'beqs 0x1008', 'moveal %fp@(8),%a0',
                      'clrl %a0@', 'moveal %fp@(12),%a1', 'clrl %a1@', 'unlk %fp', 'rts'])
        k = rt.at[0x100a]
        self.assertEqual(regalloc.dirty_sets(rt, 'any')[k], {'a0'})
        self.assertEqual(regalloc.dirty_sets(rt, 'all')[k], set())
        # a call cleans what it writes; `keep' does not
        rt = routine(['linkw %fp,#0', 'moveal %fp@(8),%a0', 'clrl %a0@', 'jsr 0xe20b12',
                      'unlk %fp', 'rts'])
        k = rt.at[0x1008]
        self.assertEqual(regalloc.dirty_sets(rt, 'any', 'clean')[k], set())
        self.assertEqual(regalloc.dirty_sets(rt, 'any', 'keep')[k], {'a0'})
        self.assertEqual(regalloc.written_sets(rt)[k], {'a0', 'call'})


class RuleTest(unittest.TestCase):
    def test_rule_spec(self):
        self.assertEqual(regalloc.rule_spec('colU'), ('col', False, True, False, None, False))
        self.assertEqual(regalloc.rule_spec('colLU'), ('col', 'blk', True, False, None, False))
        self.assertEqual(regalloc.rule_spec('colUC'), ('col', False, True, False, 'cond', False))
        self.assertEqual(regalloc.rule_spec('lifoU')[0], 'lifo')

    def test_lifo_pick(self):
        class V:
            cls = regalloc.DREGS
        # most recently freed first; never-used registers lowest first
        self.assertEqual(regalloc.lifo_pick(V, ['d2', 'd3'], {'d2': (5, 1), 'd3': (5, 2)}), 'd3')
        self.assertEqual(regalloc.lifo_pick(V, ['d2', 'd3'], {'d2': (7, 1), 'd3': (5, 2)}), 'd2')
        self.assertEqual(regalloc.lifo_pick(V, ['d4', 'd5'], {}), 'd4')


class PointerCopiesTest(unittest.TestCase):
    # EC2_$INIT 0xE42C60: the slot in D0, compared, one copy for two
    # stores (codegen-patterns 12.2, "Pointers in data registers").
    EC2_INIT = ['linkw %fp,#-4', 'movel %fp@(8),%d0', 'cmpil #1000,%d0',
                'blss 0x100e', 'moveal %d0,%a0', 'clrl %a0@', 'clrw %a0@(4)',
                'unlk %fp', 'rts']

    def test_param_used_as_data(self):
        rt = routine(self.EC2_INIT)
        self.assertEqual([k for k, _ in regalloc.ptr_dregs(rt)],
                         [('param', 'cmp', '1copy', 'Afree')])

    def test_copies_while_held(self):
        # a second copy into the register that still holds the first
        rt = routine(['linkw %fp,#0', 'movel %fp@(8),%d2', 'moveal %d2,%a0',
                      'clrl %a0@', 'moveal %d2,%a0', 'clrw %a0@(4)',
                      'moveal %d2,%a1', 'clrl %a1@', 'jsr 0xe20b12',
                      'moveal %d2,%a0', 'clrl %a0@', 'unlk %fp', 'rts'])
        self.assertEqual([k for k, _ in regalloc.ptr_copies(rt)],
                         ['fresh', 'same', 'other', 'fresh'])


class SimulationTest(unittest.TestCase):
    """Allocator orders over colU's graph (bead domain-pascal-jjh)."""
    # (8) in D1 and (12) in D0 overlap; the sum stays in D0
    TWO = ['linkw %fp,#0', 'movel %fp@(8),%d1', 'movel %fp@(12),%d0',
           'addl %d1,%d0', 'unlk %fp', 'rts']

    def graph(self, texts):
        rt = routine(texts)
        return rt, regalloc.Graph(rt)

    def index(self, rt, text):
        return next(k for k, v in enumerate(rt.values) if rt.ins[v.first].text == text)

    def test_overlap_edge_and_orders(self):
        rt, g = self.graph(self.TWO)
        a, b = self.index(rt, 'movel %fp@(8),%d1'), self.index(rt, 'movel %fp@(12),%d0')
        self.assertEqual(g.nb[a], {b})
        # order-free (colU): D1 is locally minimal; address order wants D0
        self.assertEqual(g.pick(a, {'d0'}), 'd1')
        run = regalloc.run_order(g, sorted(range(2), key=lambda k: rt.values[k].first))
        self.assertEqual(run[a], ('d0', 'd0'))
        # the second value: loo sees D1 (the original's), sim sees D0
        self.assertEqual(run[b], ('d0', 'd1'))

    def test_r2_edge_without_overlap(self):
        # 0xE177D0 shape: the sum is not computed in the copied register
        rt, g = self.graph(['linkw %fp,#0', 'movel %fp@(8),%d1', 'movel %d1,%d3',
                            'addl %a5@(20),%d3', 'movel %d3,%a5@(24)', 'unlk %fp', 'rts'])
        a, b = self.index(rt, 'movel %fp@(8),%d1'), self.index(rt, 'movel %d1,%d3')
        self.assertIn(b, g.nb[a])

    def test_call_forbids_scratch(self):
        rt, g = self.graph(['linkw %fp,#0', 'movel %fp@(8),%d2', 'jsr 0xe20b12',
                            'movel %d2,%a5@(4)', 'unlk %fp', 'rts'])
        k = self.index(rt, 'movel %fp@(8),%d2')
        self.assertEqual(g.forbid[k], {'d0', 'd1'})
        self.assertEqual(g.pick(k, set()), 'd2')

    def test_loop_depth(self):
        rt = routine(['linkw %fp,#0', 'moveq #3,%d0', 'clrl %a0@+', 'dbf %d0,0x1004',
                      'unlk %fp', 'rts'])
        d = regalloc.loop_depth(rt)
        self.assertEqual([d[i] for i in rt.reach], [0, 0, 1, 1, 0, 0])

    def test_simplify_and_priority(self):
        rt, g = self.graph(self.TWO)
        a, b = self.index(rt, 'movel %fp@(8),%d1'), self.index(rt, 'movel %fp@(12),%d0')
        # never blocked: the select order is the reverse of the removal
        # order, i.e. reverse address order, or address order with tie=-1
        self.assertEqual(regalloc.order_simplify(g, 1), ([b, a], set()))
        self.assertEqual(regalloc.order_simplify(g, -1), ([a, b], set()))
        # a priority order, ties in address order; with `defer' the
        # unconstrained values go last (here: all of them)
        self.assertEqual(regalloc.order_priority(g, lambda v: 1, 1), [a, b])
        self.assertEqual(regalloc.order_priority(g, lambda v: v.first, 1), [b, a])
        self.assertEqual(regalloc.order_priority(g, lambda v: 1, 1, defer=True), [a, b])

    def test_models_cover_every_value(self):
        rt, g = self.graph(self.TWO)
        ms = regalloc.models(rt, g)
        self.assertEqual(set(regalloc.MODELS) - set(ms), set())
        for name in regalloc.MODELS:
            self.assertEqual(sorted(ms[name][0]), [0, 1])


if __name__ == '__main__':
    unittest.main()
