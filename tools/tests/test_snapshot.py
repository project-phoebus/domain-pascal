"""Tests for tools/snapshot.py (dpc output snapshots of the conformance
units and exec programs).

The checks run on a scratch snapshot directory with one real unit, so
they need dpc (`make build') and nothing else; skipped without it.  The
snapshots themselves are checked by `make snapshot' (part of `make
test').
"""

import os
import shutil
import sys
import tempfile
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import snapshot  # noqa: E402

UNIT = 'acl_clear_super'
SRC = 'tests/conformance/acl_clear_super.pas'


class NormaliseTest(unittest.TestCase):
    def test_as_golden(self):
        # crates/dpc/tests/golden.rs normalise_strips_comments_and_spacing
        self.assertEqual(snapshot.normalise('| hdr\n\n\t.text\n\tmove.w\t(8,%fp),%d0  | c\n'),
                         '.text\nmove.w (8,%fp),%d0\n')


class SuitesTest(unittest.TestCase):
    def test_conformance_units_are_the_manifest_pas_files(self):
        units = snapshot.conformance_units()
        self.assertEqual(units[UNIT], SRC)
        self.assertTrue(all(s.endswith('.pas') for s in units.values()))

    def test_exec_link_modules_get_their_program_options(self):
        progs = {n: (s, o) for n, s, o in snapshot.exec_programs()}
        self.assertEqual(progs['addr_values'][0], 'tests/exec/addr_values.pas')
        mods = [n for n in progs if n.startswith('addr_values+')]
        self.assertEqual(mods, ['addr_values+addr_values_data.mod'])
        self.assertEqual(progs[mods[0]][1], progs['addr_values'][1])
        self.assertEqual(progs['addr_values'][1][:2], ['-I', 'tests/exec'])


@unittest.skipUnless(os.path.isfile(snapshot.DPC), 'dpc not built')
class CheckTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp()
        os.makedirs(os.path.join(self.dir, 'conformance'))
        for ext in ('.args', '.s'):
            shutil.copy(os.path.join(snapshot.SNAPSHOTS, 'conformance', UNIT + ext),
                        os.path.join(self.dir, 'conformance'))
        self.want = {'conformance': {UNIT}, 'exec': set()}

    def tearDown(self):
        shutil.rmtree(self.dir)

    def check(self):
        out = []
        with mock.patch.object(snapshot, 'SNAPSHOTS', self.dir), \
                mock.patch.object(snapshot, 'expected_names', lambda: self.want):
            bad = snapshot.check(snapshot.DPC, out=out.append)
        return bad, '\n'.join(out)

    def edit(self, old, new):
        path = os.path.join(self.dir, 'conformance', UNIT + '.s')
        with open(path) as f:
            text = f.read()
        self.assertIn(old, text)
        with open(path, 'w') as f:
            f.write(text.replace(old, new, 1))

    def test_matches(self):
        self.assertEqual(self.check(), (0, 'snapshot: 1 checked, 0 failed'))

    def test_changed_output_fails_with_a_diff(self):
        self.edit('link.w %fp,#-4', 'link.w %fp,#-8')
        bad, out = self.check()
        self.assertEqual(bad, 1)
        self.assertIn('FAIL    conformance/%s' % UNIT, out)
        self.assertIn('-link.w %fp,#-8', out)
        self.assertIn('+link.w %fp,#-4', out)

    def test_unnormalised_snapshot_fails(self):
        self.edit('link.w %fp,#-4', 'link.w  %fp,#-4 | c')
        bad, out = self.check()
        self.assertEqual(bad, 1)
        self.assertIn('is not normalised', out)

    def test_missing_and_stale(self):
        self.want = {'conformance': {UNIT, 'other'}, 'exec': {'prog'}}
        os.rename(os.path.join(self.dir, 'conformance', UNIT + '.s'),
                  os.path.join(self.dir, 'conformance', 'gone.s'))
        bad, out = self.check()
        self.assertIn('MISSING conformance/other', out)
        self.assertIn('MISSING exec/prog', out)
        self.assertIn('STALE   conformance/gone', out)
        self.assertIn('ERROR', out)     # UNIT's .args without its .s
        self.assertEqual(bad, 4)


if __name__ == '__main__':
    unittest.main()
