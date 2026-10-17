"""Tests for tools/basesurvey.py's model (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import basesurvey  # noqa: E402
from test_csesurvey import FakeRoutine  # noqa: E402

K = 14820258        # FIM_$TRACE_STS, 0xE223A2


def isdata(k):
    return k >= 0xE00000


def refs(texts):
    rt = FakeRoutine(texts)
    sy = basesurvey.BaseSym(rt, isdata)
    return rt, sy, [(b['cls'], b['i'], basesurvey.strongest(b['ev']), b['loop'])
                    for b in sy.brefs]


class BaseSymTest(unittest.TestCase):
    def test_base_kept_for_a_later_statement(self):
        # module-local 0xE5B704 at 0xE5B744..0xE5B756: two elements of
        # FIM_$TRACE_STS cleared, one base.
        rt, sy, bs = refs([
            'movew %fp@(14),%d2',
            'moveal #14820258,%a1',
            'lslw #2,%d2',
            'clrl %a1@(0,%d2:w)',
            'movew %fp@(8),%d3',
            'lslw #2,%d3',
            'clrl %a1@(0,%d3:w)',
            'rts',
        ])
        self.assertEqual(bs, [('first', 1, '-', False), ('use', 3, '-', False),
                              ('kept', 6, '-', False)])
        c, _ = basesurvey.pairs([(rt, sy)])
        self.assertEqual(dict(c), {('two', 'opndX', 'opndX', '-', '-', 'kept'): 1})

    def test_reload_after_the_lea_into_the_base(self):
        # MMAP_$SET_WS_INDEX+0xAC 0xE0D2E0..0xE0D2F2: the element's
        # address replaces the base, the next statement loads it again
        # (a store through the address between: `ptr').
        rt, sy, bs = refs([
            'moveal #15419392,%a0',
            'lsll #4,%d7',
            'lea %a0@(0,%d7:l),%a0',
            'movew %d2,%a0@(-8182)',
            'moveal #15419392,%a0',
            'lsll #4,%d6',
            'lea %a0@(0,%d6:l),%a0',
            'rts',
        ])
        self.assertIn(('lost', 4, 'ptr', False), bs)
        lost = [b for b in sy.brefs if b['cls'] == 'lost'][0]
        self.assertEqual(lost['killer'], 2)
        c, _ = basesurvey.pairs([(rt, sy)])
        self.assertEqual(dict(c), {('two', 'leaX-self', 'leaX', 'ptr', '-', 'reload'): 1})

    def test_two_bases_in_one_statement(self):
        # FLP_$INT 0xE19F78..0xE19F82: both operands' bases loaded, the
        # second while the first holds it.
        _, _, bs = refs([
            'moveal #15183604,%a1',
            'moveal #15183604,%a2',
            'movel %a1@(236),%a2@(300)',
            'rts',
        ])
        self.assertEqual([b[0] for b in bs], ['first', 'held', 'use', 'use'])

    def test_after_a_call_and_across_a_branch(self):
        # A call between: the scratch register's base lost; a conditional
        # branch between: kept (KBD_$GET_DESC 0xE1AA62..0xE1AA74).
        _, _, bs = refs([
            'moveal #14867600,%a1',
            'tstl %a1@(44,%d3:w)',
            'bnes 0x4',
            'rts',
            'cmpiw #1,%a1@(52,%d3:w)',
            'jsr 0xe1ab2a',
            'moveal #14867600,%a1',
            'movel %a1@(44,%d3:w),%fp@(-8)',
            'rts',
        ])
        self.assertIn(('kept', 4, 'br', False), bs)
        self.assertIn(('lost', 6, 'call', False), bs)

    def test_held_in_a_data_register_before_a_loop(self):
        # TTY_$K_DRAIN_OUTPUT 0xE67AF0, 0xE67B20: the base in D7 from
        # before the loop, copied to A1 in it.
        rt, sy, bs = refs([
            'movel #14820026,%d7',
            'moveal %d7,%a1',
            'movel %a1@(0,%d2:w),%d2',
            'dbf %d1,0x1',
            'rts',
        ])
        self.assertEqual(bs[0][:2], ('first', 0))
        self.assertEqual(bs[1][0], 'copy')
        self.assertTrue(bs[1][3])
        rows = basesurvey.loopbases([(rt, sy)])
        self.assertEqual([(r[4], r[5], r[6]) for r in rows], [('H', 'loaded-for-loop', True)])

    def test_loaded_in_the_loop(self):
        # KBD_$FETCH_KEY 0xE1CBAA: the base loaded in the loop body.
        rt, sy, _ = refs([
            'moveal #15419392,%a0',
            'tstb %a0@(0,%d0:w)',
            'dbf %d1,0x0',
            'rts',
        ])
        rows = basesurvey.loopbases([(rt, sy)])
        self.assertEqual([(r[4], r[6]) for r in rows], [('L', True)])

    def test_statement_boundaries(self):
        rt = FakeRoutine([
            'movew %a0@(-2,%d0:w),%d1',      # 0 x[i] := x[i] + 1 (AUDIT_$SUSPEND)
            'addqw #1,%d1',
            'movew %d1,%a0@(-2,%d0:w)',      # 2
            'movel %a1@+,%a0@(10,%d0:w)',    # 3 a copy's halves (PROC2_$FORK)
            'movel %a1@+,%a0@(14,%d0:w)',    # 4
            'movew %fp@(8),%d0',             # 5 a new statement's index
            'lslw #2,%d0',
            'clrl %a0@(0,%d0:w)',            # 7
            'cmpiw #8,%a0@(-2,%d0:w)',       # 8 one condition (0xE0526C)
            'seq %d2',
            'cmpiw #9,%a0@(-2,%d0:w)',       # 10
            'rts',
        ])
        same = basesurvey.same_statement
        self.assertTrue(same(rt, 0, 2))
        self.assertTrue(same(rt, 3, 4))
        self.assertFalse(same(rt, 4, 7))
        self.assertTrue(same(rt, 8, 10))
        self.assertFalse(same(rt, 2, 3))

    def test_address_register_between(self):
        # SIO2681_$INIT 0xE3342C..0xE33468: the channel's pointer in the
        # base's register, the base loaded again.
        rt, sy, bs = refs([
            'moveal #14868352,%a4',
            'movel %a3,%a4@(-8,%d0:w)',
            'moveal %fp@(16),%a4',
            'movel %a1,%a4@',
            'moveal #14868352,%a3',
            'movel %d1,%a3@(-16,%d2:w)',
            'rts',
        ])
        c, _ = basesurvey.pairs([(rt, sy)])
        self.assertEqual(dict(c), {('two', 'opndX', 'opndX', 'ptr', 'adef', 'reload'): 1})

    def test_rkind(self):
        rt = FakeRoutine(['lea %a0@(0,%d1:w),%a1', 'lea %a2@(16),%a2', 'pea %a1@(0,%d1:w)',
                          'clrl %a1@(4)', 'moveal #14820258,%a1'])
        self.assertEqual([basesurvey.rkind(x) for x in rt.ins],
                         ['leaX-other', 'leaF-self', 'pea', 'opnd', 'load'])


if __name__ == '__main__':
    unittest.main()
