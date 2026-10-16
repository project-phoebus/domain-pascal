"""Tests for tools/elemupdate.py's site recognisers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import elemupdate  # noqa: E402


def insns(rows):
    return [(a, 2, t) for a, t in rows]


# AUDIT_$SUSPEND 0xE70DB6..0xE70DD6
SUSPEND = insns([
    (0xe70db6, 'linkw %fp,#0'),
    (0xe70dba, 'movew 0xe20608,%d0'),
    (0xe70dc0, 'lea 0xe854d8,%a0'),
    (0xe70dc6, 'addw %d0,%d0'),
    (0xe70dc8, 'movew %a0@(fffffffffffffffe,%d0:w),%d1'),
    (0xe70dcc, 'addqw #1,%d1'),
    (0xe70dce, 'movew %d1,%a0@(fffffffffffffffe,%d0:w)'),
    (0xe70dd2, 'unlk %fp'),
    (0xe70dd4, 'rts'),
])

# ACL_$ENTER_SUPER 0xE46F90..0xE46FB4
ENTER = insns([
    (0xe46f90, 'linkw %fp,#0'),
    (0xe46f94, 'pea %a5@'),
    (0xe46f96, 'lea 0xe7cf54,%a5'),
    (0xe46f9c, 'movew 0xe20608,%d0'),
    (0xe46fa2, 'addw %d0,%d0'),
    (0xe46fa4, 'lea %a5@(0,%d0:w),%a0'),
    (0xe46fa8, 'addqw #1,%a0@(2934)'),
    (0xe46fac, 'moveal %fp@(-4),%a5'),
    (0xe46fb0, 'unlk %fp'),
    (0xe46fb2, 'rts'),
])

# PMAP_ 0xE13146..0xE13154
STATS = insns([
    (0xe13146, 'movew 0xe20608,%d0'),
    (0xe1314c, 'moveal #14834976,%a0'),
    (0xe13152, 'lslw #4,%d0'),
    (0xe13154, 'addql #1,%a0@(fffffffffffffffc,%d0:w)'),
    (0xe13158, 'rts'),
])


class Sites(unittest.TestCase):
    def test_load_change_store(self):
        [s] = elemupdate.sites(SUSPEND)
        self.assertEqual((s['addr'], s['kind'], s['form'], s['size'], s['c'], s['base'],
                          s['index'], s['width']),
                         (0xe70dc8, 'lms', 'idx', 'w', 1, 'abs', 'absw', 'w'))

    def test_lea_in_place(self):
        [s] = elemupdate.sites(ENTER)
        self.assertEqual((s['addr'], s['kind'], s['form'], s['op'], s['base'], s['index']),
                         (0xe46fa8, 'inplace', 'lea', 'addq', 'a5', 'absw'))

    def test_indexed_in_place(self):
        [s] = elemupdate.sites(STATS)
        self.assertEqual((s['kind'], s['form'], s['size'], s['base'], s['index']),
                         ('inplace', 'idx', 'l', 'sym', 'absw'))

    def test_plain(self):
        ins = insns([(0x10, 'movel %a1@(48),%d1'), (0x14, 'addql #1,%d1'),
                     (0x16, 'movel %d1,%a1@(48)'), (0x1a, 'rts')])
        self.assertEqual(len(elemupdate.plain_sites(ins)), 1)
        self.assertEqual(elemupdate.plain_sites(SUSPEND), [])


if __name__ == '__main__':
    unittest.main()
