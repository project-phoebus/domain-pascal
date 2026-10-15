"""Tests for tools/cmpsurvey.py's site recognisers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import cmpsurvey  # noqa: E402


def insns(texts, start=0x100):
    """[(addr, len, text)] at 2-byte steps (lengths do not matter here)."""
    return [(start + 2 * i, 2, t) for i, t in enumerate(texts)]


class ByteIndexTest(unittest.TestCase):
    def test_held_char_scaled(self):
        # tty_$i_enable_crash_func 0xE672AA..0xE672B4
        ins = insns(['clrw %d2', 'moveb %d0,%d2', 'addw %d2,%d2',
                     'lea %a0@(0,%d2:w),%a1', 'movew #17,%a1@(164)'])
        self.assertEqual(cmpsurvey.bidx_sites(ins), [(0x106, 'clr.w', 'w', 2, 'reg')])

    def test_clear_earlier_unscaled(self):
        # The TTY module-local 0xE1CB8A..0xE1CB92
        ins = insns(['clrw %d0', 'moveal %a4@(52),%a1', 'moveb %a2@,%d0',
                     'moveb %a1@(0,%d0:w),%a2@'])
        self.assertEqual(cmpsurvey.bidx_sites(ins), [(0x106, 'clr.w(gap)', 'w', 1, 'mem')])

    def test_word_index_is_not_a_site(self):
        ins = insns(['movew %a0@,%d0', 'addw %d0,%d0', 'movew %a5@(4,%d0:w),%d1'])
        self.assertEqual(cmpsurvey.bidx_sites(ins), [])


class TwoVariablesTest(unittest.TestCase):
    def test_left_loaded(self):
        # TTY_$K_SET_FLAG 0xE67462..0xE6746A
        ins = insns(['movew %a2@(716),%d0', 'cmpw %a2@(714),%d0', 'beqs 0x200'])
        self.assertEqual(cmpsurvey.mem2_sites(ins), ([(0x100, 'w', 'beq')], []))

    def test_both_loaded(self):
        ins = insns(['movew %a0@,%d0', 'movew %a1@(2),%d1', 'cmpw %d1,%d0', 'bnes 0x200'])
        self.assertEqual(cmpsurvey.mem2_sites(ins), ([], [(0x100, 'w', 'bne')]))

    def test_held_register_is_not_a_site(self):
        # TTY_$K_GET 0xE1C44E `cmp.w (0x2ca,A2),D2', D2 held
        ins = insns(['tstb %d5', 'cmpw %a2@(714),%d2', 'beqw 0x200'])
        self.assertEqual(cmpsurvey.mem2_sites(ins), ([], []))


class WidenTest(unittest.TestCase):
    def test_signed_against_unsigned(self):
        # TTY_$K_PUT 0xE67C42..0xE67C4C
        ins = insns(['movew %d2,%d0', 'clrl %d1', 'extl %d0', 'movew %a4@,%d1',
                     'cmpl %d1,%d0', 'bges 0x200'])
        self.assertEqual(cmpsurvey.widen_sites(ins), ([(0x108, 'sign', 'zero', 'bge')], []))

    def test_word_against_long(self):
        ins = insns(['movew %a0@,%d0', 'extl %d0', 'cmpl %a5@(8),%d0', 'bles 0x200'])
        self.assertEqual(cmpsurvey.widen_sites(ins), ([], [(0x104, 'sign', 'mem', 'ble')]))


class CmpmTest(unittest.TestCase):
    def test_unrolled_pair(self):
        # TTY_$K_SIMULATE_TERMINAL_INPUT 0xE1C1AC..0xE1C1BC
        ins = insns(['lea %fp@(-8),%a0', 'lea %a2@(76),%a1', 'moveq #1,%d0',
                     'cmpml %a1@+,%a0@+', 'bnes 0x200', 'cmpml %a1@+,%a0@+', 'beqs 0x300'])
        self.assertEqual(cmpsurvey.cmpm_sites(ins), [(0x106, 'l', 2, '#1', False, 'bne/beq')])

    def test_loop(self):
        # DISK_IO 0xE3D84E..0xE3D856
        ins = insns(['moveq #7,%d1', 'cmpml %a1@+,%a0@+', 'dbne %d1,0x102', 'beqs 0x300'])
        self.assertEqual(cmpsurvey.cmpm_sites(ins), [(0x102, 'l', 1, '#7', True, 'dbne')])

    def test_value(self):
        # PACCT_$ON 0xE5A9B8..0xE5A9C0: the run ends in `sne'
        ins = insns(['moveq #1,%d0', 'cmpml %a1@+,%a0@+', 'bnes 0x108', 'cmpml %a1@+,%a0@+',
                     'sne %d0'])
        self.assertEqual(cmpsurvey.cmpm_sites(ins), [(0x102, 'l', 2, '#1', False, 'bne')])


class ByteCompareTest(unittest.TestCase):
    def test_forms(self):
        ins = insns(['cmpib #-1,%d0', 'cmpib #32,%a0@', 'cmpb %d1,%d0', 'cmpb %a0@,%d0'])
        self.assertEqual([f for _, f in cmpsurvey.cmpb_sites(ins)],
                         ['cmpi.b #-1', 'cmpi.b', 'cmp.b reg', 'cmp.b mem'])


if __name__ == '__main__':
    unittest.main()


class OrderedBranchTest(unittest.TestCase):
    def test_unsigned_longwords(self):
        # PMAP_$PURIFIER_L 0xE13EB0..0xE13EB8, EC2_$INIT 0xE42C68..0xE42C6E
        ins = insns(['movel %a1@(4),%d7', 'cmpl %a1@(32),%d7', 'bhis 0x200',
                     'cmpil #1000,%d0', 'blss 0x200'])
        self.assertEqual(cmpsurvey.ord_sites(ins),
                         [(0x102, 'cmpl', 'mem', 'U'), (0x106, 'cmpil', 'imm', 'U')])

    def test_signed_and_equality(self):
        ins = insns(['cmpw %d1,%d0', 'bges 0x200', 'tstl %d0', 'beqs 0x200'])
        self.assertEqual(cmpsurvey.ord_sites(ins), [(0x100, 'cmpw', 'reg', 'S')])


class CallCompareTest(unittest.TestCase):
    def test_tpad_boolean_tested(self):
        # TPAD_$SET_UNIT_MODE 0xE6980E..0xE6981A
        ins = insns(['jsr 0xe70024', 'cmpw %a4@,%d0', 'slt %d2', 'tstb %d2', 'bpls 0x200'])
        self.assertEqual(cmpsurvey.callcmp_sites(ins),
                         [(0x102, 'cmp.w', 'dst', 'mem-An', 's<cc>>tst', 'd2')])

    def test_static_and_pop(self):
        # TPAD_$INQUIRE_UNIT 0xE69984..0xE69990; a pop skipped
        ins = insns(['jsr 0xe70024', 'addqw #4,%sp', 'cmpw %a5@(380),%d0', 'slt %d0',
                     'tstb %d0', 'bpls 0x200'])
        self.assertEqual(cmpsurvey.callcmp_sites(ins),
                         [(0x104, 'cmp.w', 'dst', 'mem-A5', 's<cc>>tst', 'd0')])

    def test_direct_branches(self):
        # 0xE0FD5A `cmp.l (d,A6),D0; ble'; 0xE10AA2 `cmp.l D0,Dn; ble';
        # `tst.w D0; beq' (0xE0F7BE)
        ins = insns(['bsrw 0x300', 'cmpl %fp@(-8),%d0', 'bles 0x200',
                     'jsr 0x400', 'lea %sp@(12),%sp', 'cmpl %d0,%d3', 'bles 0x200',
                     'jsr 0x500', 'tstw %d0', 'beqs 0x200'])
        self.assertEqual(cmpsurvey.callcmp_sites(ins),
                         [(0x102, 'cmp.l', 'dst', 'mem-A6', 'ble', '-'),
                          (0x10A, 'cmp.l', 'src', 'reg', 'ble', '-'),
                          (0x110, 'tst.w', 'dst', 'zero', 'beq', '-')])

    def test_m_arith_calls_left_out(self):
        # NETWORK_$READ_AHEAD 0xE0FD52..0xE0FD5E: M$DIU$LLW computes a
        # `div', not a function's result
        ins = insns(['jsr 0xe0ac68', 'addqw #8,%sp', 'cmpl %fp@(-364),%d0', 'bles 0x200'])
        self.assertEqual(cmpsurvey.callcmp_sites(ins), [])

    def test_value_stored_and_no_compare(self):
        # SOCK_$PUT_INT 0xE161EA `tst.w D0; seq D0' (the function's
        # result); a call whose result is stored is not a site
        ins = insns(['bsrs 0x300', 'tstw %d0', 'seq %d0', 'jsr 0x400', 'movew %d0,%a0@'])
        self.assertEqual(cmpsurvey.callcmp_sites(ins),
                         [(0x102, 'tst.w', 'dst', 'zero', 's<cc>', 'd0')])


class AndOrTest(unittest.TestCase):
    def test_right_operand_without_call(self):
        # 0xE072A2..0xE072B6: the call is the left operand's (its D0)
        ins = insns(['jsr 0xe148ca', 'addqw #4,%sp', 'orb %fp@(-18),%d0',
                     'cmpil #512,%d3', 'scs %d1', 'orb %d1,%d0'])
        self.assertEqual(cmpsurvey.andor_sites(ins),
                         [(0x104, 'or', '-'), (0x10A, 'or', '-')])

    def test_right_operand_with_call(self):
        ins = insns(['tstw %a2@', 'sgt %d2', 'jsr 0x300', 'cmpw %a2@,%d0', 'sge %d0',
                     'andb %d0,%d2'])
        self.assertEqual(cmpsurvey.andor_sites(ins), [(0x10A, 'and', 'call')])

    def test_mask_is_not_a_site(self):
        ins = insns(['andb #7,%d0'])
        self.assertEqual(cmpsurvey.andor_sites(ins), [])
