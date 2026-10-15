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


class OpFlagsTest(unittest.TestCase):
    def test_branch_on_the_operation(self):
        # SOCK_$ALLOCATE 0xE15EF8..0xE15EFC `move.w D3,D0; or.w D4,D0;
        # beq.s'; tty_$i_put_chars 0xE1B046 `subq.w #1,D1; bpl.s'
        ins = insns(['movew %d3,%d0', 'orw %d4,%d0', 'beqs 0x200',
                     'subqw #1,%d1', 'bpls 0x200'])
        self.assertEqual(cmpsurvey.opflags_sites(ins),
                         [(0x102, 'or', 'w', 'flags', 'w', 'eq', 'b', '-'),
                          (0x106, 'sub', 'w', 'flags', 'w', 'pl', 'b', '-')])

    def test_tst_between(self):
        # SMD_$SIGNAL 0xE6F25A..0xE6F260 `moveq #39,D1; sub.l D0,D1; tst.l
        # D1; bgt.s'; 0xE0A542 `moveq #7,D1; and.b (-64,A6),D1; tst.w D1'
        ins = insns(['moveq #39,%d1', 'subl %d0,%d1', 'tstl %d1', 'bgts 0x200',
                     'moveq #7,%d1', 'andb %fp@(-64),%d1', 'tstw %d1', 'bnes 0x200'])
        self.assertEqual(cmpsurvey.opflags_sites(ins),
                         [(0x102, 'sub', 'l', 'tst', 'l', 'gt', 'b', '-'),
                          (0x10A, 'and', 'b', 'tst', 'w', 'ne', 'b', '-')])
        self.assertEqual(cmpsurvey.opflags_class(cmpsurvey.opflags_sites(ins)[1]),
                         ('and', 'eq/ne', 'tst', 'size', '-'))

    def test_s_cc_store_and_join(self):
        # SOCK_$PUT_INT-like `seq' after a subtraction; a store between
        # (0xE3BCB4 `sub.w D1,D0; move.w D0,(-92,A6); bne.s'); a `tst' that
        # a branch reaches (a join, 0xE08F80)
        ins = insns(['subw %d1,%d0', 'seq %d2',
                     'subw %d1,%d0', 'movew %d0,%fp@(-92)', 'bnes 0x200',
                     'subl %d1,%d3', 'tstl %d3', 'beqs 0x200', 'bras 0x10c'])
        self.assertEqual(cmpsurvey.opflags_sites(ins),
                         [(0x100, 'sub', 'w', 'flags', 'w', 'eq', 's', '-'),
                          (0x104, 'sub', 'w', 'store', 'w', 'ne', 'b', '-'),
                          (0x10A, 'sub', 'l', 'tst', 'l', 'eq', 'b', 'join')])

    def test_not_sites(self):
        # a compare, a load into an address register, a gap in memory
        ins = insns(['cmpw %d1,%d0', 'beqs 0x200', 'moveal %a0@,%a1', 'beqs 0x200'])
        ins += [(0x120, 2, 'addw %d1,%d0'), (0x130, 2, 'beqs 0x200')]
        self.assertEqual(cmpsurvey.opflags_sites(ins), [])


class SelectTest(unittest.TestCase):
    def test_memory_operand(self):
        # MMAP_$ALLOC_PURE 0xE0D7CE `cmp.l (0x30,A2),D6; bls.b; move.l
        # (0x30,A2),D6'
        ins = insns(['cmpl %a2@(48),%d6', 'blss 0x106', 'movel %a2@(48),%d6',
                     'subql #2,%sp'])
        self.assertEqual(cmpsurvey.select_sites(ins), [(0x100, 'cmp', 'l', 'ls', 'mem')])

    def test_immediate_and_moveq(self):
        # PKT_$DAT_COPY 0xE12852; 0xE08B50 (`moveq')
        ins = insns(['cmpiw #1024,%d5', 'bles 0x106', 'movew #1024,%d5', 'nop',
                     'cmpiw #1,%d5', 'bles 0x10e', 'moveq #1,%d5', 'nop'])
        self.assertEqual(cmpsurvey.select_sites(ins), [(0x100, 'cmp', 'w', 'le', 'imm'),
                                                       (0x108, 'cmp', 'w', 'le', 'imm')])

    def test_constant_in_a_register(self):
        # OS_INIT_START 0xE2F08A
        ins = insns(['movew #504,%d0', 'cmpw %d0,%d2', 'bles 0x108', 'movew %d0,%d2',
                     'nop'])
        self.assertEqual(cmpsurvey.select_sites(ins), [(0x102, 'cmp', 'w', 'le', 'constreg')])

    def test_clamp_at_zero(self):
        # NETBUF_$DEL_PAGES 0xE0EB64; 0xE4BD78
        ins = insns(['subl %d4,%d1', 'bpls 0x106', 'clrl %d1', 'nop',
                     'tstw %d2', 'bpls 0x10e', 'clrw %d2', 'nop'])
        self.assertEqual(cmpsurvey.select_sites(ins), [(0x100, 'zero', 'l', 'pl', 'flags'),
                                                       (0x108, 'zero', 'w', 'pl', 'tst')])

    def test_not_sites(self):
        # Another register moved, another target, a move of another value.
        ins = insns(['cmpw %d1,%d0', 'bles 0x106', 'movew %d1,%d2', 'nop',
                     'cmpw %d1,%d0', 'bles 0x110', 'movew %d1,%d0', 'nop', 'nop',
                     'cmpw %d1,%d0', 'bles 0x118', 'movew %d3,%d0', 'nop'])
        self.assertEqual(cmpsurvey.select_sites(ins), [])
