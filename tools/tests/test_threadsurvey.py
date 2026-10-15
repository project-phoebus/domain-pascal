"""Tests for tools/threadsurvey.py's site recognisers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import threadsurvey  # noqa: E402


def insns(rows):
    """[(addr, len, text)] from [(addr, text)] (lengths from successive
    addresses, the last one 2) or [(addr, len, text)] as given."""
    out = []
    for i, row in enumerate(rows):
        if len(row) == 3:
            out.append(row)
            continue
        a, t = row
        n = rows[i + 1][0] - a if i + 1 < len(rows) else 2
        out.append((a, n, t))
    return out


# TTY_$K_SET_FLAG 0xE67454..0xE6748A
SET_FLAG = insns([
    (0xe67454, 'moveal %fp@(16),%a1'),
    (0xe67458, 'tstb %a1@'),
    (0xe6745a, 'bpls 0xe6747c'),
    (0xe6745c, 'bset #4,%a2@(9)'),
    (0xe67462, 'movew %a2@(716),%d0'),
    (0xe67466, 'cmpw %a2@(714),%d0'),
    (0xe6746a, 'beqs 0xe67482'),
    (0xe6746c, 'subql #2,%sp'),
    (0xe6746e, 'movew #26,%sp@-'),
    (0xe67472, 'pea %a2@'),
    (0xe67474, 'jsr 0xe1b824'),
    (0xe6747a, 'bras 0xe67482'),
    (0xe6747c, 'bclr #4,%a2@(9)'),
    (0xe67482, 'moveml %fp@(-20),%a2-%a3/%a5'),
    (0xe67488, 'unlk %fp'),
    (0xe6748a, 'rts'),
])


class ThreadTest(unittest.TestCase):
    def test_set_flag(self):
        self.assertEqual(threadsurvey.thread_sites(SET_FLAG),
                         [(0xe6746a, 'bcc', 's', True, 'epi')])

    def test_plain_if_else_is_not_a_site(self):
        ins = insns([(0x100, 'tstw %d0'), (0x102, 'beqs 0x108'),
                     (0x104, 'clrw %d1'), (0x106, 'bras 0x10a'),
                     (0x108, 'moveq #1,%d1'), (0x10a, 'rts')])
        self.assertEqual(threadsurvey.thread_sites(ins), [])

    def test_no_branch_to_a_bra(self):
        self.assertEqual(threadsurvey.tobra_sites(SET_FLAG), [])

    def test_branch_to_a_bra(self):
        ins = insns([(0x100, 'beqs 0x104'), (0x102, 'clrw %d1'),
                     (0x104, 'bras 0x108'), (0x106, 'moveq #1,%d1'),
                     (0x108, 'rts')])
        self.assertEqual(threadsurvey.tobra_sites(ins), [(0x100, 'beqs', 0x104)])


class DeadTest(unittest.TestCase):
    def test_unreached_bra(self):
        ins = insns([(0x100, 2, 'bras 0x108'), (0x104, 2, 'rts')])
        words = {0x102: 0x6004}
        self.assertEqual(threadsurvey.dead_sites(ins, lambda a: words.get(a, 0)), [(0x102, 'bras')])

    def test_reached_bra(self):
        ins = insns([(0x100, 'bras 0x108'), (0x102, 'bras 0x10a'), (0x104, 'rts')])
        self.assertEqual(threadsurvey.dead_sites(ins, lambda a: 0x6004 if a == 0x102 else 0), [])


class LoopTest(unittest.TestCase):
    def test_threaded_head_with_filler(self):
        # The module-local 0xE4AFA8, 0xE4B0EE..0xE4B100
        ins = insns([
            (0xe4b0ee, 'bnes 0xe4b0fc'),
            (0xe4b0f0, 'movel %a3,%a2@(8)'),
            (0xe4b0f4, 2, 'bras 0xe4b106'),
            (0xe4b0f8, 'moveal %a0@(48),%a0'),
            (0xe4b0fc, 'tstl %a0@(48)'),
            (0xe4b100, 'bnes 0xe4b0f8'),
            (0xe4b102, 'movel %a3,%a0@(48)'),
            (0xe4b106, 'rts'),
        ])
        words = {0xe4b0f6: 0x2048}
        self.assertEqual(threadsurvey.loop_sites(ins, lambda a: words.get(a, 0)),
                         [(0xe4b100, 0xe4b0f8, 'threaded', 'F')])

    def test_while_entered_by_bra(self):
        ins = insns([(0x100, 'bras 0x106'), (0x102, 'clrw %d1'),
                     (0x104, 'addqw #1,%d0'), (0x106, 'tstw %d0'),
                     (0x108, 'bnes 0x102'), (0x10a, 'rts')])
        self.assertEqual(threadsurvey.loop_sites(ins, lambda a: 0),
                         [(0x108, 0x102, 'bra', 'U')])



# DXM_$HELPER_COMMON 0xE171FC..0xE1721C: a loop with no way out ends the
# routine (bead domain-pascal-pkle).
HELPER_COMMON = insns([
    (0xe171e4, 'linkw %fp,#-4'),
    (0xe171e8, 'moveml %d2/%a2-%a4,%sp@-'),
    (0xe171ec, 'moveal %fp@(8),%a3'),
    (0xe171f0, 'moveq #1,%d2'),
    (0xe171f2, 'moveal #0,%a2'),
    (0xe171f8, 'lea %a3@(12),%a4'),
    (0xe171fc, 'clrl %sp@-'),
    (0xe171fe, 'clrl %sp@-'),
    (0xe17200, 'movel %d2,%sp@-'),
    (0xe17202, 'pea %a2@'),
    (0xe17204, 'pea %a2@'),
    (0xe17206, 'pea %a4@'),
    (0xe17208, 'jsr 0xe20610'),
    (0xe1720e, 'lea %sp@(24),%sp'),
    (0xe17212, 'pea %a3@'),
    (0xe17214, 'bsrw 0xe17168'),
    (0xe17218, 'addqw #4,%sp'),
    (0xe1721a, 'addql #1,%d2'),
    (0xe1721c, 'bras 0xe171fc'),
])


class NoTestLoopTest(unittest.TestCase):
    def test_no_epilogue(self):
        self.assertEqual(threadsurvey.noepi_sites(HELPER_COMMON),
                         [(0xe1721c, 'bras 0xe171fc')])

    def test_epilogue(self):
        self.assertEqual(threadsurvey.noepi_sites(SET_FLAG), [])

    def test_branch_to_next(self):
        ins = insns([(0x100, 'beqs 0x102'), (0x102, 'braw 0x106'),
                     (0x106, 'rts')])
        self.assertEqual(threadsurvey.next_sites(ins), [(0x100, 'beqs'), (0x102, 'braw')])
        self.assertEqual(threadsurvey.next_sites(SET_FLAG), [])

    def test_conditional_branch_over_a_bra(self):
        ins = insns([(0x100, 'beqs 0x104'), (0x102, 'bras 0x100'),
                     (0x104, 'rts')])
        self.assertEqual(threadsurvey.over_sites(ins), [(0x100, 'beqs')])
        self.assertEqual(threadsurvey.over_sites(SET_FLAG), [])


if __name__ == '__main__':
    unittest.main()
