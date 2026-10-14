"""Tests for tools/boolsurvey.py's site recognisers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import boolsurvey  # noqa: E402


def insns(texts, start=0x100):
    """[(addr, len, text)] at 2-byte steps (lengths do not matter here)."""
    return [(start + 2 * i, 2, t) for i, t in enumerate(texts)]


class AndOrTest(unittest.TestCase):
    def test_two_compares(self):
        # TTY_$I_ERR 0xE1BE48..0xE1BE60
        ins = insns(['cmpil #3538955,%d0', 'seq %d1', 'moveal %fp@(8),%a0',
                     'movel %a0@(20),%d2', 'btst #10,%d2', 'sne %d3',
                     'andb %d3,%d1', 'bmis 0x200'])
        self.assertEqual(boolsurvey.andor_sites(ins),
                         [(0x10c, 'and', 'scc', 'scc', 'bmi')])

    def test_chain_and_memory_operand(self):
        # 0xE4BC2A..0xE4BC4A, then a boolean variable in memory
        ins = insns(['cmpil #1,%fp@(8)', 'seq %d0', 'cmpil #2,%fp@(8)', 'seq %d1',
                     'orb %d1,%d0', 'orb %a5@(844),%d0', 'bpls 0x200'])
        self.assertEqual(boolsurvey.andor_sites(ins),
                         [(0x108, 'or', 'scc', 'scc', 'chain'),
                          (0x10a, 'or', 'chain', 'mem', 'bpl')])

    def test_not_and_store(self):
        # 0xE0AED6 `or.b; not.b; bmi'; 0xE38630 `and.b; move.b Dn,<mem>'
        ins = insns(['sne %d0', 'sne %d1', 'orb %d1,%d0', 'notb %d0', 'bmiw 0x200'])
        self.assertEqual(boolsurvey.andor_sites(ins),
                         [(0x104, 'or', 'scc', 'scc', 'not,bmi')])
        ins = insns(['sne %d1', 'sls %d4', 'andb %d4,%d1', 'moveb %d1,%a3@(631)',
                     'tstw %a4@'])
        self.assertEqual(boolsurvey.andor_sites(ins),
                         [(0x104, 'and', 'scc', 'scc', 'store')])

    def test_held_copy_and_not_load(self):
        # 0xE01A20 `move.b D2,D0; and.b D3,D0; bmi'; 0xE1009C
        ins = insns(['moveb %d2,%d0', 'andb %d3,%d0', 'bmis 0x200'])
        self.assertEqual(boolsurvey.andor_sites(ins),
                         [(0x102, 'and', 'copy', 'none', 'bmi')])
        ins = insns(['seq %d0', 'moveb %fp@(26),%d3', 'notb %d3', 'andb %d3,%d0',
                     'bplw 0x200'])
        self.assertEqual(boolsurvey.andor_sites(ins),
                         [(0x106, 'and', 'scc', 'not+load', 'bpl')])

    def test_byte_arithmetic_is_not_a_site(self):
        ins = insns(['moveb %a0@,%d0', 'moveb %a1@,%d1', 'andb %d1,%d0',
                     'moveb %d0,%a2@'])
        self.assertEqual(boolsurvey.andor_sites(ins), [])


class Bit0Test(unittest.TestCase):
    def test_forms(self):
        # FIM_$BUILD_DF 0xE0A6F6; AST_$GET_LOCATION 0xE0472E; a byte in memory
        ins = insns(['btst #0,%d5', 'beqs 0x200', 'btst #0,%d2', 'sne %d0',
                     'btst #0,%a2@(23)', 'bnes 0x200'])
        self.assertEqual(boolsurvey.bit0_sites(ins),
                         [(0x100, 'reg', 'beq'), (0x104, 'reg', 'sne'),
                          (0x108, 'mem-odd', 'bne')])


class ElemTest(unittest.TestCase):
    def test_netlog_fields(self):
        # NETLOG_$LOG_IT 0xE71BCA..0xE71BD2
        ins = insns(['moveal %a5@(116),%a0', 'lea %a0@(0,%d1:w),%a0',
                     'moveb %d2,%a0@(-26)', 'moveb 0xe20609,%a0@(-25)',
                     'movel %fp@(-14),%a0@(-24)', 'jsr 0x300'])
        self.assertEqual(boolsurvey.elem_sites(ins),
                         [(0x102, 'w', 3, (-26, -25, -24))])


if __name__ == '__main__':
    unittest.main()
