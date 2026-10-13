"""Tests for tools/nestsurvey.py's instruction helpers (no image needed)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..'))
import nestsurvey  # noqa: E402


def ins(*texts):
    return [(0x100 + 2 * i, 2, t) for i, t in enumerate(texts)]


class EntryTest(unittest.TestCase):
    def test_a1_read_at_entry(self):
        # 0xE1359C
        r = ins('linkw %fp,#-4', 'moveml %d2/%a2-%a3,%sp@-', 'moveal %a1,%a2',
                'movel %fp@(12),%d2')
        self.assertEqual(nestsurvey.a1_entry(r), 0x104)

    def test_a1_written_first_or_after_a_call(self):
        self.assertIsNone(nestsurvey.a1_entry(ins('linkw %fp,#0', 'moveal %fp@(8),%a1',
                                                  'movew %a1@,%d0')))
        self.assertIsNone(nestsurvey.a1_entry(ins('linkw %fp,#0', 'jsr 0xe20b12',
                                                  'movew %a1@,%d0')))

    def test_a1_source_of_a_call(self):
        # 0xE13920..0xE1392A: pushes, then `movea.l A6,A1; bsr.w'
        r = ins('subql #2,%sp', 'movew %d6,%sp@-', 'movel %d5,%sp@-', 'pea %a2@',
                'moveal %fp,%a1', 'bsrw 0xe1359c')
        self.assertEqual(nestsurvey.a1_source(r, 5)[0], 'fp')
        r = ins('moveal %a4,%a1', 'bsrw 0xe1359c')
        self.assertEqual(nestsurvey.a1_source(r, 1)[0], 'reg')
        r = ins('moveal %fp@(-76),%a1', 'pea %a1@', 'bsrw 0xe04b00')
        self.assertEqual(nestsurvey.a1_source(r, 2)[0], 'chain')
        r = ins('jsr 0xe20b12', 'bsrw 0xe1360c')
        self.assertEqual(nestsurvey.a1_source(r, 1)[0], '-')

    def test_reads_and_writes(self):
        self.assertTrue(nestsurvey.writes('moveal %a4,%a1', '%a1'))
        self.assertFalse(nestsurvey.writes('pea %a1@', '%a1'))
        self.assertTrue(nestsurvey.reads('movew %a1@(4),%d0', '%a1'))
        self.assertTrue(nestsurvey.reads('clrl %a1@', '%a1'))
        self.assertFalse(nestsurvey.reads('moveal %a0,%a1', '%a1'))

    def test_entry_sequence(self):
        # 0xE082A8: parameters, then the link
        r = nestsurvey.R('AREA_', 0x100, 0x200, None,
                         ins('linkw %fp,#-36', 'moveml %d2-%d7/%a2-%a4,%sp@-',
                             'movel %fp@(8),%d7', 'movew %fp@(12),%d3',
                             'moveal %fp@,%a4', 'tstw %d3'), -36)
        before, link, after = nestsurvey.entry(r)
        self.assertEqual(before, ['movel %fp@(8),%d7', 'movew %fp@(12),%d3'])
        self.assertEqual(link, 'moveal %fp@,%a4')
        self.assertEqual(after, [])


if __name__ == '__main__':
    unittest.main()
