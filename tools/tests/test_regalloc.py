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


if __name__ == '__main__':
    unittest.main()
