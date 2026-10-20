"""Tests for tools/blocksizes.py: dpc's data blocks against the map's D
segments (beads domain-pascal-kplu, domain-pascal-2u4o)."""

import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import blocksizes  # noqa: E402

# Map lines of the SAU2 map's shape (domain_os.10.2.map 833, 1082, 2223,
# 2248, 2284): sections with a number, segments without; DISK_ as three
# units of one section.
MAP = """\
Build ID test
D24  E1DC00  WIRED_DATA         loaded at 11F400, size = 104A0
D    E24304  MST_WIRED          size = 164
     E24304  MST_$DISKLESS_MSG
D    E2C048  DIR_WIRED          size = 1BC
D21  E78400  .DATA              loaded at 179C00, size = D308
D39  E78400  OS_DATA            loaded at 179C00, size = 0
D    E7A1CC  DISK_              size = 40
D    E7AD5C  DISK_              size = 10
D    E7AEF4  FLP_               size = 13C
D    E7DBF8  DIR                size = 212C
D    E826C4  DISK_              size = 10
"""


class BlockSizes(unittest.TestCase):
    def setUp(self):
        fd, self.path = tempfile.mkstemp(suffix='.map')
        with os.fdopen(fd, 'w') as f:
            f.write(MAP)
        self.segs = blocksizes.map_segments(self.path)

    def tearDown(self):
        os.unlink(self.path)

    def test_segments_by_module_and_section(self):
        self.assertEqual(self.segs[('MST_WIRED', 'WIRED_DATA')], [(0xE24304, 0x164)])
        # the zero-size OS_DATA section does not take .DATA's segments
        self.assertEqual(self.segs[('DIR', '.DATA')], [(0xE7DBF8, 0x212C)])
        self.assertEqual(len(self.segs[('DISK_', '.DATA')]), 3)

    def test_compare(self):
        c = blocksizes.compare
        self.assertEqual(c(self.segs, '.moddata.DIR', 0x212C), ('=', 0xE7DBF8, 0x212C))
        self.assertEqual(c(self.segs, '.moddata.DIR', 0x2124), ('-8', 0xE7DBF8, 0x212C))
        self.assertEqual(c(self.segs, '.moddata.FLP_', 0x140), ('+4', 0xE7AEF4, 0x13C))
        self.assertEqual(c(self.segs, '.bss.moddata.MST_WIRED.WIRED_DATA', 0x164),
                         ('=', 0xE24304, 0x164))
        # one module, several units: the segment of the block's size
        self.assertEqual(c(self.segs, '.moddata.DISK_', 0x40), ('=', 0xE7A1CC, 0x40))
        self.assertEqual(c(self.segs, '.moddata.DISK_', 0x10)[0], '?')
        self.assertEqual(c(self.segs, '.moddata.NONE', 4), ('?', None, None))


if __name__ == '__main__':
    unittest.main()
