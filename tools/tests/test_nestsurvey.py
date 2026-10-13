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


class FunctionTest(unittest.TestCase):
    def test_d0_read_after_call(self):
        # 0xE0082C..0xE00830: popped, then the result tested
        self.assertTrue(nestsurvey.reads_d0_after(
            ins('bsrw 0xe00800', 'addqw #8,%sp', 'tstb %d0'), 0))
        # 0xE04700..0xE04704: not popped, the result stored
        self.assertTrue(nestsurvey.reads_d0_after(
            ins('bsrw 0xe04600', 'movel %d0,%a3@'), 0))
        # D0 written, not read: a procedure as far as this call shows
        self.assertFalse(nestsurvey.reads_d0_after(
            ins('jsr 0xe20b12', 'addqw #4,%sp', 'moveq #1,%d0'), 0))
        self.assertFalse(nestsurvey.reads_d0_after(
            ins('jsr 0xe20b12', 'movew %d2,%d0'), 0))


class FamilyTest(unittest.TestCase):
    def routine(self, r0, calls=(), a1=None, dyn=()):
        r = nestsurvey.R('DIR', r0, r0 + 0x10, None, [], -4)
        r.calls, r.a1, r.dyn = list(calls), a1, list(dyn)
        return r

    def test_top_parent_through_nested_parents(self):
        # DIR 0xE4F034 is nested in 0xE4F100, itself nested in 0xE4F3BA,
        # which an A1 call from the top-level 0xE4FE0A makes nested too
        top = self.routine(0xE4FE0A)
        lvl1 = self.routine(0xE4F3BA, a1=0xE4F3C2)
        lvl2 = self.routine(0xE4F100, dyn=[(0xE4F108, 'moveal %fp@,%a2')])
        lvl3 = self.routine(0xE4F034, dyn=[(0xE4F03C, 'moveal %fp@,%a0')])
        lvl1.calls = [nestsurvey.Call(top, 0xE4FE40, lvl1.r0, 'fp', '', None)]
        lvl2.calls = [nestsurvey.Call(lvl1, 0xE4F400, lvl2.r0, '-', '', None)]
        lvl3.calls = [nestsurvey.Call(lvl2, 0xE4F200, lvl3.r0, '-', '', None)]
        by_start = {r.r0: r for r in (top, lvl1, lvl2, lvl3)}
        nss = {lvl1.r0, lvl2.r0, lvl3.r0}
        for r in (lvl1, lvl2, lvl3):
            self.assertEqual(nestsurvey.top_parent(r, by_start, nss), top.r0)
        # two callers of a `(A6)' routine: no single parent
        lvl3.calls.append(nestsurvey.Call(top, 0xE4FE50, lvl3.r0, '-', '', None))
        self.assertIsNone(nestsurvey.top_parent(lvl3, by_start, nss))


if __name__ == '__main__':
    unittest.main()
