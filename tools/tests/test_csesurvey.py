"""Tests for tools/csesurvey.py's symbolic model (no image needed)."""

import os
import sys
import unittest
from collections import defaultdict

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import csesurvey  # noqa: E402
import regalloc  # noqa: E402


class FakeRoutine:
    """A regalloc.Routine stand-in: instructions at 2-byte steps, branch
    targets given as instruction indices (written as 0x<index>)."""

    def __init__(self, texts):
        self.ins = []
        for k, t in enumerate(texts):
            self.ins.append(regalloc.Insn(k, 2, t))
        self.reach = list(range(len(self.ins)))
        self.succ, self.pred = {}, defaultdict(list)
        for i, x in enumerate(self.ins):
            out = []
            if x.kind not in ('ret', 'jmp'):
                if x.target is not None and not x.call:
                    out.append(x.target)
                if x.kind != 'br' and i + 1 < len(self.ins):
                    out.append(i + 1)
            self.succ[i] = out
            for s in out:
                self.pred[s].append(i)


def events(texts):
    sy = csesurvey.Sym(FakeRoutine(texts))
    return sorted((t, i, csesurvey.kind(e), tuple(sorted(es)))
                  for t, i, e, es, r in sy.final)


class SymTest(unittest.TestCase):
    def test_zero_extension_reused_across_a_join(self):
        # 0xE177A6..0xE177D0: `offset' zero-extended, the `if', then the
        # copy at the join reads it.
        ev = events([
            'movew %fp@(12),%d0',
            'moveal %fp@(16),%a0',
            'clrl %d1',
            'movew %d0,%d1',
            'cmpiw #1024,%d0',
            'blss 0x7',
            'clrw %a0@',
            'movel %d1,%d3',
            'rts',
        ])
        self.assertIn(('reuse', 7, 'zx', ('br', 'join', 'ptr')), ev)

    def test_scaled_index_reused_past_other_stores(self):
        # FIM_$ACKNOWLEDGE: AS_ID * 12 in D2 at 0xE0A9B0, past a store to
        # another array at a constant address.
        ev = events([
            'movew 0xe2060a,%d2',
            'lslw #2,%d2',
            'movew %d2,%d0',
            'addw %d0,%d0',
            'addw %d0,%d2',
            'movew 0xe2060a,%d0',
            'moveal #14820490,%a1',
            'clrb %a1@(0,%d0:w)',
            'moveal #14820548,%a1',
            'pea %a1@(0,%d2:w)',
            'rts',
        ])
        self.assertIn(('reuse', 9, 'mulsh', ('st',)), ev)

    def test_recomputed_after_a_store_to_its_operand(self):
        ev = events([
            'movew %a5@(124),%d0',
            'lslw #2,%d0',
            'movel %d1,%a5@(0,%d0:w)',
            'movew %d2,%a5@(124)',
            'movew %a5@(124),%d0',
            'lslw #2,%d0',
            'rts',
        ])
        self.assertIn(('lost', 5, 'shl', ('kill',)), ev)

    def test_stored_value_is_not_a_cse(self):
        ev = events([
            'clrl %d1',
            'movew %fp@(12),%d1',
            'movel %d1,%fp@(-4)',
            'movel %d1,%a5@(8)',
            'rts',
        ])
        self.assertIn(('reuse', 3, 'zx', ('st', 'sto')), ev)

    def test_aliasing(self):
        self.assertTrue(csesurvey.may_alias('A5+124', 'A5+124'))
        self.assertTrue(csesurvey.may_alias('A5+0[x:w]', 'A5+124'))
        self.assertFalse(csesurvey.may_alias('A5+8', 'A5+124'))
        self.assertFalse(csesurvey.may_alias('abs:e2060a', "('k', 1)+0[x:w]"))


if __name__ == '__main__':
    unittest.main()
