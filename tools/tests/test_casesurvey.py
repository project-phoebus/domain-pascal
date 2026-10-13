"""Tests for tools/casesurvey.py's chain recogniser (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import casesurvey  # noqa: E402


def insns(texts):
    """[(addr, len, text)] at 2-byte steps (lengths do not matter here)."""
    return [(0x100 + 2 * i, 2, t) for i, t in enumerate(texts)]


class ChainTest(unittest.TestCase):
    def test_label_zero_on_the_flags_of_the_load(self):
        # SYSBUS_$INIT 0xE0AB64
        ins = insns(['movew %a0@(4),%d0', 'beqs 0xe0ab78', 'cmpiw #1,%d0', 'beqs 0xe0ab8a',
                     'cmpiw #2,%d0', 'beqs 0xe0ab9c', 'bras 0xe0abaa'])
        run, j = casesurvey.chain_at(ins, 0)
        self.assertEqual([k for k, _ in run], [0, 1, 2])
        self.assertEqual(ins[j][2], 'bras 0xe0abaa')

    def test_tst_and_register_change(self):
        # TIME_$GET_EC 0xE1675C: label 0 as `tst.w'; a compare on another
        # register ends the chain
        ins = insns(['cmpiw #1,%d0', 'beqs 0xe16768', 'tstw %d0', 'beqs 0xe16772',
                     'cmpiw #3,%d1', 'beqs 0xe16700'])
        run, j = casesurvey.chain_at(ins, 0)
        self.assertEqual(run, [(1, 0xE16768), (0, 0xE16772)])
        self.assertEqual(j, 4)

    def test_label_zero_on_a_copy_compared_in_its_source(self):
        # MSG_$SARI 0xE59EF6: `move.w D0,D1; beq', then compares on D0
        ins = insns(['movew %d0,%d1', 'beqs 0xe59f08', 'cmpiw #1,%d0', 'beqs 0xe59f6e',
                     'cmpiw #2,%d0', 'beqs 0xe59f76', 'bras 0xe59ec0'])
        run, j = casesurvey.chain_at(ins, 0)
        self.assertEqual([k for k, _ in run], [0, 1, 2])
        self.assertEqual(j, 6)

    def test_single_compare_is_not_a_chain(self):
        ins = insns(['cmpiw #1,%d0', 'beqs 0xe16768', 'bras 0xe16770'])
        self.assertIsNone(casesurvey.chain_at(ins, 0))


if __name__ == '__main__':
    unittest.main()
