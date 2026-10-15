"""Tests for tools/datasurvey.py's helpers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import datasurvey  # noqa: E402


def ins(*texts):
    return [(0x100 + 2 * i, 2, t) for i, t in enumerate(texts)]


class DataSurveyTest(unittest.TestCase):
    def test_access_sizes(self):
        self.assertEqual(datasurvey.access_size('pea'), 'a')
        self.assertEqual(datasurvey.access_size('st'), 'b')
        self.assertEqual(datasurvey.access_size('bset'), 'b')
        self.assertEqual(datasurvey.access_size('movew'), 'w')
        self.assertEqual(datasurvey.access_size('tstl'), 'l')

    def test_operands(self):
        # SIO2681_$SET_BREAK 0xE1D13E (decimal plain displacement) and
        # SIO2681_$SET_LINE 0xE1D2FE (`3235 5050': an indexed operand's
        # displacement is printed in hex without `0x').
        a5 = 0xE2DEB8
        r = ins('moveml %d2/%a2/%a5,%sp@-', 'lea 0xe2deb8,%a5', 'pea %a5@',
                'moveb %a5@(74),%a0@(5)', 'movew %a5@(50,%d5:w),%d1',
                'tstb 0xe24c44', 'bsrw 0xe1d1da', 'moveal %fp@(-12),%a5')
        self.assertEqual(datasurvey.operands(r, a5), [
            (0x106, a5 + 0x4A, 'b'),
            (0x108, a5 + 0x50, 'x'),
            (0x10A, 0xE24C44, 'b'),
        ])
        # Without a known A5 only absolute operands are kept.
        self.assertEqual(datasurvey.operands(r, None), [(0x10A, 0xE24C44, 'b')])

    def test_lone_bytes(self):
        offs = {0x8: {'x'}, 0x48: {'b'}, 0x4A: {'b'}, 0x4C: {'l'},
                0x54: {'b', 'a'}, 0x60: {'b'}, 0x61: {'b'}, 0x70: {'w'},
                0x73: {'b'}}
        self.assertEqual(datasurvey.lone_bytes(offs),
                         {0x48: 'near-x', 0x4A: 'lone', 0x54: 'lone',
                          0x73: 'lone'})


if __name__ == '__main__':
    unittest.main()
