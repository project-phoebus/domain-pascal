"""Tests for tools/nilsurvey.py's site recognisers (no image needed)."""

import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import nilsurvey  # noqa: E402


def insns(texts, start=0x100):
    """[(addr, len, text)] at 2-byte steps (lengths do not matter here)."""
    return [(start + 2 * i, 2, t) for i, t in enumerate(texts)]


class StoreTest(unittest.TestCase):
    def test_kinds(self):
        # BAT_$ALLOC_VTOCE 0xE3AEDA, BAT_$ALLOCATE 0xE3B2EE, HINT_$INIT
        # 0xE3139A, DBUF_$GET_BLOCK 0xE3A678, a copy loop's `(An)+'
        ins = insns(['clrl %fp@(-20)', 'clrl %a5@(3384)', 'clrl 0xe2459c',
                     'clrl %a4@(4)', 'clrl %a3@', 'clrl %a0@+',
                     'clrl %a0@(0,%d0:w)', 'movel #0,%a1@(8)'])
        self.assertEqual(nilsurvey.store_sites(ins),
                         [(0x100, 'clr.l', 'frame'), (0x102, 'clr.l', 'unit'),
                          (0x104, 'clr.l', 'abs'), (0x106, 'clr.l', 'areg'),
                          (0x108, 'clr.l', 'areg'), (0x10a, 'clr.l', 'postinc'),
                          (0x10c, 'clr.l', 'indexed'), (0x10e, 'move.l #0', 'areg')])

    def test_registers_and_pushes_are_not_stores(self):
        ins = insns(['clrl %d0', 'clrl %sp@-', 'movel #0,%sp@-', 'clrw %a0@'])
        self.assertEqual(nilsurvey.store_sites(ins), [])


class PushTest(unittest.TestCase):
    def test_forms_and_callee(self):
        # FLP's EXCS 0xE3E2A0..0xE3E2C0: the integer 0 by `clr.l', the
        # address of a symbol at 0 by `move.l #0' with its immediate a
        # RELOC cell (EC_$WAIT 0xE20610)
        ins = insns(['clrl %sp@-', 'movel %d1,%sp@-', 'movel #0,%sp@-',
                     'pea %a5@(96)', 'jsr 0xe20610', 'lea %sp@(24),%sp',
                     'clrl %sp@-', 'bsrw 0xe0e238', 'movel #0,%sp@-'])
        names = {0xe20610: 'EC_$WAIT'}
        self.assertEqual(nilsurvey.push_sites(ins, lambda x: names.get(x, '0x%X' % x),
                                              frozenset([0x106])),
                         [(0x100, 'clr.l', 'EC_$WAIT'),
                          (0x104, 'move.l #0 [reloc]', 'EC_$WAIT'),
                          (0x10c, 'clr.l', '0xE0E238'), (0x110, 'move.l #0', '-')])


class AregTest(unittest.TestCase):
    def test_readers(self):
        # DXM_$HELPER_COMMON 0xE171F2: a relocated 0 held in A2, pushed
        # twice
        ins = insns(['moveal #0,%a2', 'lea %a3@(12),%a4', 'clrl %sp@-',
                     'pea %a2@', 'pea %a2@'])
        self.assertEqual(nilsurvey.areg_sites(ins), [(0x100, 'movea.l #0', 'pea')])
        self.assertEqual(nilsurvey.areg_sites(ins, frozenset([0x102])),
                         [(0x100, 'movea.l #0 [reloc]', 'pea')])
        # ROUTE_$FIND_PORTP 0xE15B84: a pointer result, then the epilogue
        ins = insns(['subal %a0,%a0', 'moveml %fp@(-24),%d2-%d3/%a5', 'unlk %fp', 'rts'])
        self.assertEqual(nilsurvey.areg_sites(ins), [(0x100, 'suba.l', 'rts')])
        ins = insns(['subal %a2,%a2', 'bras 0x200'])
        self.assertEqual(nilsurvey.areg_sites(ins), [(0x100, 'suba.l', 'branch')])
        ins = insns(['subal %a2,%a2', 'moveal %a2,%a0', 'rts'])
        self.assertEqual(nilsurvey.areg_sites(ins), [(0x100, 'suba.l', 'result')])
        ins = insns(['subal %a2,%a2', 'movew %a2@(18),%d0'])
        self.assertEqual(nilsurvey.areg_sites(ins), [(0x100, 'suba.l', 'base')])
        ins = insns(['moveal #0,%a0', 'moveal %a0,%a2'])
        self.assertEqual(nilsurvey.areg_sites(ins), [(0x100, 'movea.l #0', 'copy')])
        ins = insns(['subal %a2,%a2', 'moveal %fp@(8),%a2'])
        self.assertEqual(nilsurvey.areg_sites(ins), [(0x100, 'suba.l', 'overwritten')])


class RelocTest(unittest.TestCase):
    def test_cells(self):
        # a word count, then load addresses: one in the kernel (0x102000
        # is VA 0xE00800), one below it (COLD), left out
        img = bytearray(0x20)
        table = struct.pack('>HII', 2, 0x102000 + 0x12, 0x101410)
        off = nilsurvey.RELOC_VA - 0x10
        img[0x10:0x10 + len(table)] = table
        self.assertEqual(nilsurvey.reloc_cells(bytes(img), off=off), frozenset([0xE00812]))


if __name__ == '__main__':
    unittest.main()
