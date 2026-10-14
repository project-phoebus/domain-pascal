"""Tests for tools/setsurvey.py's site recognisers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import setsurvey  # noqa: E402


def insns(texts, start=0x100):
    """[(addr, len, text)] at 2-byte steps (lengths do not matter here)."""
    return [(start + 2 * i, 2, t) for i, t in enumerate(texts)]


class OneMemberTest(unittest.TestCase):
    def test_union_and_difference_in_place(self):
        # TTY_$K_SET_INPUT_FLAG 0xE6768E..0xE676A2
        ins = insns(['clrl %d1', 'movew %a2@,%d2', 'bset %d2,%d1', 'orl %d1,%a0@(20)',
                     'bras 0x200', 'clrl %d1', 'movew %a2@,%d2', 'bset %d2,%d1',
                     'notl %d1', 'andl %d1,%a0@(20)'])
        self.assertEqual(setsurvey.one_sites(ins),
                         [(0x104, 'l', 'or-mem'), (0x10e, 'l', 'not-and-mem')])

    def test_word_set_pushed(self):
        # DISK_$FORMAT 0xE3D4E2..0xE3D4E6
        ins = insns(['clrw %d1', 'bset %d3,%d1', 'movew %d1,%sp@-'])
        self.assertEqual(setsurvey.one_sites(ins), [(0x102, 'w', 'push')])

    def test_bset_without_clear_is_not_a_site(self):
        ins = insns(['movel %a0@,%d1', 'bset %d2,%d1', 'orl %d1,%a0@(4)'])
        self.assertEqual(setsurvey.one_sites(ins), [])


class MemberTest(unittest.TestCase):
    def test_set_of_char(self):
        # The TTY module-local 0xE1B740..0xE1B750
        ins = insns(['clrw %d1', 'movew #255,%d2', 'moveb %a0@(721),%d1', 'lea %a5@,%a0',
                     'subw %d1,%d2', 'lsrw #3,%d2', 'btst %d1,%a0@(0,%d2:w)'])
        self.assertEqual(setsurvey.memb_sites(ins), [(0x10c, 'tst', 256, False, False)])

    def test_guarded_constant_set(self):
        # NAME_$INIT 0xE31836..0xE31846
        ins = insns(['moveq #95,%d5', 'moveb %a5@(-1,%d0:w),%d4', 'subw %d4,%d5',
                     'bcss 0x200', 'lsrw #3,%d5', 'lea %pc@(0xe318cc),%a0',
                     'btst %d4,%a0@(0,%d5:w)'])
        self.assertEqual(setsurvey.memb_sites(ins), [(0x10c, 'tst', 96, True, True)])

    def test_packed_array_is_not_a_site(self):
        # PCHIST_$COUNT 0xE1A164..0xE1A16C: bit 7 - k mod 8 of byte k div 8
        ins = insns(['moveq #7,%d1', 'moveq #7,%d3', 'andb %d0,%d1', 'subb %d1,%d3',
                     'btst %d3,%a1@(24)'])
        self.assertEqual(setsurvey.memb_sites(ins), [])


class OtherFormsTest(unittest.TestCase):
    def test_difference_tested(self):
        # PROC2_$SIGPAUSE 0xE3FAC6..0xE3FAD0
        ins = insns(['movel %a4@(-108),%d1', 'notl %d1', 'andl %a4@(-100),%d1', 'beqs 0x200'])
        self.assertEqual(setsurvey.diff_sites(ins), [0x100])

    def test_loop(self):
        # MSG_$CLOSEI 0xE59478..0xE59480
        ins = insns(['movel %a3@+,%d0', 'notl %d0', 'andl %a1@+,%d0', 'movel %d0,%a4@+',
                     'dbf %d1,0x100'])
        self.assertEqual(setsurvey.loop_sites(ins), [0x100])

    def test_constant_through_a_register(self):
        # The TTY module-local 0xE1C006
        ins = insns(['moveq #-8,%d1', 'andw %d1,%a2@(8)', 'andiw #-225,%a2@(8)'])
        self.assertEqual(setsurvey.regmem_sites(ins), ([0x100], [0x104]))

    def test_and_of_a_held_value(self):
        # TTY_$I_RCV 0xE1BACE; 0xE177A0
        ins = insns(['movew %d4,%d1', 'andiw #128,%d1', 'moveq #-2,%d2', 'andw %d1,%d2'])
        self.assertEqual(setsurvey.andc_sites(ins),
                         [(0x100, 'copy+andi', False), (0x104, 'const-first', True)])


if __name__ == '__main__':
    unittest.main()
