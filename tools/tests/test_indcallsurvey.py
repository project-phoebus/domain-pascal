"""Tests for tools/indcallsurvey.py's pure parts (no image needed)."""

import os
import re
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import indcallsurvey  # noqa: E402

# TTY_$I_ERR 0xE1BE08..0xE1BE26
TTY_I_ERR = [
    (0xe1be08, 4, 'linkw %fp,#-4'),
    (0xe1be0c, 4, 'moveml %d2-%d3,%sp@-'),
    (0xe1be10, 2, 'subql #2,%sp'),
    (0xe1be12, 2, 'st %sp@-'),
    (0xe1be14, 4, 'moveal %fp@(8),%a0'),
    (0xe1be18, 2, 'movel %a0@,%sp@-'),
    (0xe1be1a, 4, 'moveal %a0@(704),%a1'),
    (0xe1be1e, 2, 'jsr %a1@'),
    (0xe1be20, 2, 'addqw #8,%sp'),
    (0xe1be22, 4, 'movel %d0,%fp@(-4)'),
    (0xe1be26, 2, 'clrw %d1'),
]


class SiteTest(unittest.TestCase):
    def test_tty_i_err(self):
        i = 7
        j = indcallsurvey._JSR.match(TTY_I_ERR[i][2])
        s = indcallsurvey.site('TTY', False, 'TTY_$I_ERR', 0xe1be08, TTY_I_ERR, i,
                               set(), j)
        self.assertEqual(s['reg'], '%a1')
        self.assertEqual(s['where'], 'after-pushes')
        self.assertEqual(s['args'], ['%a0'])
        self.assertEqual(s['load'], ['moveal %a0@(704),%a1'])
        self.assertEqual(s['pop'], 'addqw #8,%sp')
        self.assertEqual(s['reads'], ['%d0'])
        self.assertFalse(s['a5'])
        self.assertEqual(indcallsurvey.load_form(s), 'moveal An@(d),An')

    def test_dest(self):
        self.assertEqual(indcallsurvey.dest('moveal %d2,%a1'), '%a1')
        self.assertEqual(indcallsurvey.dest('clrw %d0'), '%d0')
        self.assertIsNone(indcallsurvey.dest('jsr %a0@'))
        self.assertTrue(indcallsurvey.is_push('subql #2,%sp'))
        self.assertTrue(indcallsurvey.is_push('pea %a2@'))
        self.assertFalse(indcallsurvey.is_push('moveal %a2@,%a0'))

    def test_jsr_forms(self):
        self.assertTrue(indcallsurvey._JSR.match('jsr %a0@'))
        self.assertTrue(indcallsurvey._JSR.match('jsr %a1@(4)'))
        self.assertIsNone(indcallsurvey._JSR.match('jsr 0xe20b12'))
        self.assertIsNone(re.match(indcallsurvey._JSR, 'jsr %a0@(0,%d0:w)'))


if __name__ == '__main__':
    unittest.main()
