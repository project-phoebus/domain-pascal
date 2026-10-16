"""Tests for tools/pushsurvey.py's recognisers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import pushsurvey  # noqa: E402


def insns(items, start=0x100):
    """[(addr, len, text)] from [(len, text)], laid end to end."""
    out, a = [], start
    for n, t in items:
        out.append((a, n, t))
        a += n
    return out


class FormTest(unittest.TestCase):
    def test_forms(self):
        f = pushsurvey.push_form
        self.assertEqual(f('subql #2,%sp', 2), (2, 'pad', None))
        self.assertEqual(f('pea 0x1', 4), (4, 'lc', 1))
        self.assertEqual(f('pea 0xe2060a', 6)[1], 'x')      # pea abs.l
        self.assertEqual(f('clrl %sp@-', 2), (4, 'lc', 0))
        self.assertEqual(f('clrw %sp@-', 2), (2, 'wc', 0))
        self.assertEqual(f('st %sp@-', 2), (2, 'st', 0xff))
        self.assertEqual(f('movel #262144,%sp@-', 6), (4, 'lc', 0x40000))
        self.assertEqual(f('movew #-1,%sp@-', 4), (2, 'wc', 0xffff))
        self.assertEqual(f('movew %d2,%sp@-', 2)[:2], (2, 'x'))
        self.assertIsNone(f('movew %d2,%d3', 2))


class RunTest(unittest.TestCase):
    def test_netlog_three_zero_words(self):
        # 0xE135CE..0xE135F2: the pad, three zero words as `clr.l; clr.w',
        # then NETLOG_$LOG_IT's other arguments
        ins = insns([(2, 'subql #2,%sp'), (2, 'clrl %sp@-'), (2, 'clrw %sp@-'),
                     (2, 'movew %d2,%sp@-'), (4, 'movew %fp@(16),%sp@-'),
                     (4, 'moveal %a2@(8),%a0'), (4, 'movew %a0@(12),%sp@-'),
                     (4, 'pea %a3@(16)'), (4, 'movew #5,%sp@-'),
                     (6, 'jsr 0xe71b38')])
        slots = pushsurvey.call_slots(ins, len(ins) - 1)
        self.assertEqual([(s[0], s[3]) for s in slots],
                         [(8, 'wc'), (10, 'x'), (14, 'x'), (16, 'x'), (18, 'x'),
                          (20, 'wc'), (22, 'lc'), (26, 'pad')])
        reads = {8: {'w'}, 10: {'l'}, 14: {'w'}, 16: {'w'}, 18: {'w'},
                 20: {'w'}, 22: {'w'}, 24: {'w'}}
        runs = pushsurvey.runs(pushsurvey.expand(slots, reads))
        self.assertEqual([pushsurvey.shape(r) for r in runs], ['W', 'L W'])
        self.assertEqual([e[0] for e in runs[1]], [20, 22, 24])

    def test_true_splits_and_longword_args(self):
        # 0xE51510: `pea (0x4).w; st; clr.l' for FILE_$PRIV_LOCK's side,
        # lock_mode | local_only | flags, key; the next `clr.l' a
        # longword argument (read `.l')
        ins = insns([(2, 'clrl %sp@-'), (2, 'clrl %sp@-'), (2, 'st %sp@-'),
                     (4, 'pea 0x4'), (6, 'movew 0xe2060a,%sp@-'),
                     (4, 'pea %fp@(-88)'), (6, 'jsr 0xe5f0ee')])
        slots = pushsurvey.call_slots(ins, len(ins) - 1)
        reads = {8: {'l'}, 12: {'w'}, 14: {'w'}, 16: {'w'}, 18: {'b'},
                 20: {'w'}, 22: {'w'}, 24: {'l'}}
        runs = pushsurvey.runs(pushsurvey.expand(slots, reads))
        self.assertEqual([pushsurvey.shape(r) for r in runs], ['L', 'L'])


if __name__ == '__main__':
    unittest.main()
