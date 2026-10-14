"""Tests for tools/storesurvey.py (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import regalloc  # noqa: E402
import storesurvey  # noqa: E402


class FakeRoutine:
    def __init__(self, texts, start=0x100):
        self.ins = [regalloc.Insn(start + 2 * i, 2, t) for i, t in enumerate(texts)]


def run(texts):
    rt = FakeRoutine(texts)
    return storesurvey.stores(rt, [list(range(len(rt.ins)))])


class StoresTest(unittest.TestCase):
    def test_set_delay(self):
        # TTY_$K_SET_DELAY 0xE67A40..0xE67A4A: the element (destination)
        # first, interleaved with the value's slot load
        res = run(['moveal %d0,%a0', 'movew %a2@,%d1',
                   'moveal %fp@(16),%a1', 'addw %d1,%d1',
                   'movew %a1@,%a0@(64,%d1:w)'])
        self.assertEqual(len(res), 1)
        addr, dsh, ssh, first, nd, ns, inter, _ = res[0]
        self.assertEqual((addr, dsh, ssh, first, nd, ns, inter),
                         (0x108, 'idx', 'ptr', 'dst', 3, 1, True))
        self.assertEqual(storesurvey.predicted(nd, ns), 'dst')

    def test_tie_value_first(self):
        # TTY_$K_GET 0xE1C4D6..0xE1C4DE
        res = run(['lea %a2@(0,%d2:w),%a0', 'moveal %fp@(-4),%a1',
                   'moveb %a0@(721),%a1@'])
        self.assertEqual([r[1:7] for r in res], [('ptr', 'ptr', 'src', 1, 1, False)])
        self.assertEqual(storesurvey.predicted(1, 1), 'src')

    def test_statement_boundary_and_held(self):
        # a register set up before the previous store is not this
        # statement's; a held register (A5/A6) sets nothing up
        res = run(['moveal %fp@(8),%a0', 'movew #1,%a0@', 'moveal %fp@(12),%a1',
                   'movew %a1@,%a5@(4)'])
        self.assertEqual(res, [])


if __name__ == '__main__':
    unittest.main()
