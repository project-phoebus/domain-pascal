"""Tests for tools/dispcheck.py: displacements in a gas listing checked
against their targets (bead domain-pascal-pxov)."""

import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import dispcheck  # noqa: E402

# `m68k-elf-as -m68010 -aln=` of a `bsr.w`, a `bra.w` and a `(d,PC)` over
# 40000 bytes (wrapped without a word), a `dbf` in reach, and a `bsr.w`
# of another unit's routine (left to ld: a relocation at its extension
# word).
WRAPPED = """\
   1               \t\t.text
   2               \tF:
   3 0000 6100 9C52 \t\tbsr.w\tG
   4 0004 6000 9C4A \t\tbra.w\t1f
   5 0008 41FA 9C48 \t\tlea\t(.Lc0,%pc),%a0
   6 000c 51C8 0002 \t\tdbf\t%d0,2f
   7               \t2:
   8 0010 0000 0000 \t\t.space\t40000
   8      0000 0000
   9               \t1:
  10 9c50 4E75      \t\trts
  11               \t.Lc0:
  12 9c52 1234      \t\t.short\t0x1234
  13               \tG:
  14 9c54 4E75      \t\trts
  16 9c56 6100 0000 \t\tbsr.w\tX
"""
RELOCS = """
RELOCATION RECORDS FOR [.text]:
OFFSET   TYPE              VALUE
00009c58 R_68K_PC16        X
"""

# In reach: a short branch, a word one, a `dbf` back, a table entry.
GOOD = """\
   1               \t\t.text
   2               \tF:
   3               \t1:
   4 0000 6702      \t\tbeq.s\t2f
   5 0002 4E71      \t\tnop
   6               \t2:
   7 0004 51C8 FFFA \t\tdbf\t%d0,1b
   8 0008 6000 FFF6 \t\tbra.w\t1b
   9               \t3:
  10 000c 0004      \t\t.short\t4f-3b
  11 000e 4E71      \t\tnop
  12               \t4:
  13 0010 4E75      \t\trts
"""

# A `dbf` gas lengthened (out of reach: `dbf; bra; jmp`), 12 bytes.
LONG_DBF = """\
   1               \t\t.text
   2               \t1:
   3 0000 4E71      \t\tnop
   4 0002 51C8 0004 \t\tdbf\t%d0,1b
   4      6006 4EF9
   4      0000 0000
"""


class CheckListing(unittest.TestCase):
    def test_wrapped_displacements_are_bad(self):
        checked, bad = dispcheck.check_listing(WRAPPED, dispcheck.parse_relocs(RELOCS))
        self.assertEqual(checked, 4)
        self.assertEqual(len(bad), 3, bad)
        self.assertTrue(any('bsr.w\tG' in b for b in bad))
        self.assertTrue(any('bra.w\t1f' in b for b in bad))
        self.assertTrue(any('(.Lc0,%pc)' in b for b in bad))

    def test_relocated_call_is_left_to_ld(self):
        _, bad = dispcheck.check_listing(WRAPPED, dispcheck.parse_relocs(RELOCS))
        self.assertFalse(any('bsr.w\tX' in b for b in bad))
        # Without the relocation it would be a call of an unknown label.
        _, bad = dispcheck.check_listing(WRAPPED)
        self.assertTrue(any('bsr.w\tX' in b for b in bad))

    def test_in_reach_is_good(self):
        checked, bad = dispcheck.check_listing(GOOD)
        self.assertEqual((checked, bad), (4, []))

    def test_lengthened_dbf_is_bad(self):
        checked, bad = dispcheck.check_listing(LONG_DBF)
        self.assertEqual(checked, 1)
        self.assertEqual(len(bad), 1)
        self.assertIn('12 bytes', bad[0])


@unittest.skipUnless(shutil.which(dispcheck.AS) and shutil.which(dispcheck.OBJDUMP),
                     'm68k-elf-as/objdump not installed')
class Assemble(unittest.TestCase):
    def run_asm(self, text):
        with tempfile.TemporaryDirectory() as d:
            src = os.path.join(d, 't.s')
            with open(src, 'w') as f:
                f.write(text)
            p, bad = dispcheck.assemble_checked(src, os.path.join(d, 't.o'))
            self.assertEqual(p.returncode, 0, p.stderr)
            return bad

    def test_far_branch_is_caught(self):
        bad = self.run_asm('\t.text\nF:\n\tbra.w\t1f\n\t.space\t40000\n1:\n\trts\n')
        self.assertEqual(len(bad), 1, bad)

    def test_near_branch_and_external_call(self):
        bad = self.run_asm('\t.text\nF:\n\tbsr.w\tX\n\tbra.w\t1f\n\t.space\t400\n1:\n\trts\n')
        self.assertEqual(bad, [])


if __name__ == '__main__':
    unittest.main()
