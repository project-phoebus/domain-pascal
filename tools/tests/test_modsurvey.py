"""Tests for tools/modsurvey.py (no image or build needed)."""

import os
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import linktest  # noqa: E402
import modsurvey  # noqa: E402
import sau2map  # noqa: E402

MAP = """\
I 4  E70000  PROC_SECT          loaded at 100000, size = 1000
I    E70000  CT                 size = 4
     E70000  CT_$PROC_START
     E70000  CT_$RELEASE
I    E70010  GPU                size = 14
     E70010  GPU_$INIT
     E70022  GPU_$TERM
I    E70030  NAME               size = 40
     E70030  NAME_$FREE_ASID
I    E70070  M_ASM              size = 10
     E70070  M_$HAND
D 9  E90000  DATA_SECT          loaded at 200000, size = 100
D    E90000  GPU                size = 10
I    E70100  NAME2              size = 40
     E70100  NAME_$FORK
I    E70140  CT_END_ADDR        size = 4
     E70142  CT_$PROC_END
"""

NM = """\
ct/release.o:00000000 T CT_$RELEASE
gpu/init.o:00000000 T GPU_$INIT
gpu/term.o:00000000 T GPU_$TERM
gpu/term.o:00000010 T gpu_$helper
name/asid.o:00000000 T NAME_$FREE_ASID
name/asid.o:00000040 T NAME_$FORK
"""


class SurveyTest(unittest.TestCase):
    def setUp(self):
        self.m = sau2map.LinkMap.parse(MAP)
        self.defs = linktest.parse_nm(NM)
        self.units = [['GPU_$INIT', 'GPU_$TERM'], ['CT_$RELEASE']]

    def row(self, name, local=()):
        seg = self.m.segment(name, 'I')
        return modsurvey.survey_row(seg, self.m, self.defs, self.units, local)

    def test_manifest_units(self):
        with tempfile.NamedTemporaryFile('w', suffix='.txt', delete=False) as f:
            f.write('# comment\n'
                    'a.pas  X_$A X_$B:1A  --section-addr .bss.moddata.X=0xE0  a5-always=X_$A\n'
                    'b.pas  0xE00EB0 Y_$C  module-routines=Y_$D\n')
        try:
            self.assertEqual(modsurvey.manifest_units(f.name),
                             [['X_$A', 'X_$B'], ['0xE00EB0', 'Y_$C']])
        finally:
            os.unlink(f.name)

    def test_whole_with_marker_name(self):
        # CT_$PROC_START shares CT_$RELEASE's address: one routine
        r = self.row('CT')
        self.assertTrue(r.whole)
        self.assertEqual(r.noc, {'CT_$PROC_START'})
        self.assertEqual(r.cobjs, ['ct/release.o'])

    def test_helpers_and_data(self):
        r = self.row('GPU')
        self.assertTrue(r.whole and r.data)
        self.assertEqual(r.helpers, {'gpu_$helper'})
        self.assertFalse(r.blocked)

    def test_local_routine_not_whole(self):
        self.assertFalse(self.row('GPU', local=[0xE7001A]).whole)

    def test_out_blocks(self):
        r = self.row('NAME')
        self.assertEqual(r.out, {'NAME_$FORK'})
        self.assertTrue(r.blocked)
        self.assertIn('OUT:NAME_$FORK', r.format())

    def test_asm_blocks(self):
        r = modsurvey.survey_row(self.m.segment('GPU', 'I'), self.m, self.defs, [],
                                 asm_objs={'gpu/init.o'})
        self.assertEqual(r.asm, {'GPU_$INIT'})
        self.assertTrue(r.blocked)

    def test_survey_ranks_and_skips(self):
        old = (modsurvey.LO, modsurvey.HI)
        modsurvey.LO, modsurvey.HI = 0xE70000, 0xE80000
        try:
            rows = modsurvey.survey(self.m, self.defs, self.units,
                                    starts=lambda seg: [seg.addr])
        finally:
            modsurvey.LO, modsurvey.HI = old
        names = [r.seg.name for r in rows]
        self.assertNotIn('M_ASM', names)            # hand assembly
        self.assertNotIn('CT_END_ADDR', names)      # section markers only
        self.assertEqual(names[:2], ['CT', 'GPU'])  # unblocked, smallest first
        self.assertTrue(rows[-1].blocked)


if __name__ == '__main__':
    unittest.main()
