"""Tests for tools/schedsurvey.py's dependence model and the necessary-
condition check of list rules (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import regalloc  # noqa: E402
import schedsurvey as ss  # noqa: E402
from test_regalloc import routine  # noqa: E402


def info(text):
    return ss.Info(regalloc.Insn(0x1000, 2, text))


class LocTest(unittest.TestCase):
    def test_classes(self):
        self.assertEqual(ss.loc('%fp@(12)', 2), ('f', 12, 14))
        self.assertEqual(ss.loc('%a5@', 4), ('5', 0, 4))
        self.assertEqual(ss.loc('%a5@(0,%d0:w)', 4), ('5', None, None))
        self.assertEqual(ss.loc('0xe2060a', 2), ('abs', 0xE2060A, 0xE2060C))
        self.assertEqual(ss.loc('%sp@-', 2)[0], 'sp')
        self.assertEqual(ss.loc('%a0@(4)', 4)[0], 'p')
        self.assertIsNone(ss.loc('#1024', 4))
        self.assertIsNone(ss.loc('%d0', 4))

    def test_alias(self):
        f = ss.loc
        self.assertFalse(ss.alias(f('%fp@(8)', 4), f('%fp@(12)', 4)))
        self.assertTrue(ss.alias(f('%fp@(8)', 4), f('%fp@(10)', 2)))
        self.assertTrue(ss.alias(f('%a0@', 4), f('%fp@(8)', 4)))
        self.assertTrue(ss.alias(f('%sp@-', 2), f('%fp@(8)', 4)))
        self.assertFalse(ss.alias(f('%fp@(8)', 4), f('%a5@(8)', 4)))


class InfoTest(unittest.TestCase):
    def test_kinds(self):
        x = info('movew %d2,%a0@')
        self.assertTrue(x.real_store and x.mem and x.cc_w)
        x = info('movew #20,%sp@-')
        self.assertTrue(x.push and not x.real_store and 'a7' in x.w)
        self.assertTrue(info('subql #2,%sp').pad)
        self.assertTrue(info('addqw #4,%sp').pop)
        self.assertFalse(info('moveal %fp@(8),%a0').cc_w)
        self.assertTrue(info('lea 0xe24304,%a5').barrier)
        self.assertTrue(info('oriw #1792,%sr').barrier)
        self.assertTrue(info('sne %d0').cc_r)


class DepsTest(unittest.TestCase):
    def test_store_models(self):
        run = [info('movel %d1,%fp@(-4)'), info('movel %fp@(-8),%d2'), info('moveq #1,%d3')]
        self.assertEqual(ss.deps(run, 'alias')[1], set())
        self.assertEqual(ss.deps(run, 'nodis')[1], {0})
        self.assertEqual(ss.deps(run, 'stores')[2], {0, 1})   # last cc setter too

    def test_pad_after_store(self):
        run = [info('movel %d0,%fp@(-8)'), info('subql #2,%sp')]
        self.assertEqual(ss.deps(run, 'alias')[1], {0})


class CheckTest(unittest.TestCase):
    """0xE00F2C-like: a copy, a shift reading it, then an independent load
    that was ready: top-down rules are contradicted there."""

    TEXTS = ['linkw %fp,#0', 'movel %d2,%d0', 'lsll #4,%d0', 'moveal %fp@(12),%a1',
             'movew %a1@,%d1', 'unlk %fp', 'rts']

    def test_top_down_contradicted(self):
        rt = routine(self.TEXTS)
        c, ex = ss.check([(None, rt)], 'stores', 'raw', False, show=5)
        self.assertEqual(c['avoid'], 1)
        self.assertIn('lsll #4,%d0', ex[0])
        self.assertIn('moveal %fp@(12),%a1', ex[0])

    def test_bottom_up(self):
        rt = routine(self.TEXTS)
        c, ex = ss.check([(None, rt)], 'stores', 'raw', True, show=5)
        # From the bottom: the `moveal' next to its use could have been
        # replaced by the shift (contradicted); the copy next to the shift
        # had no other candidate (forced).
        self.assertEqual((c['avoid'], c['forced']), (1, 1))
        self.assertIn('moveal %fp@(12),%a1', ex[0])


if __name__ == '__main__':
    unittest.main()
