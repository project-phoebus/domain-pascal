"""Tests for tools/linktest.py's logic that need no kernel build.

The full run (`make linktest') rebuilds a scratch copy of ../source and
takes about 20 s; it is not part of `make test'.
"""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import linktest  # noqa: E402
import sau2map  # noqa: E402


class ManifestTest(unittest.TestCase):
    def test_plain_unit(self):
        u = linktest.parse_manifest_line(
            ['tests/conformance/iic.pas', 'IIC_$INIT', 'IIC_$ACQUIRE'])
        self.assertEqual(u.specs, ['IIC_$INIT', 'IIC_$ACQUIRE'])
        self.assertEqual(u.dpc_opts, [])
        self.assertEqual(u.stem, 'iic')

    def test_options_forwarded(self):
        u = linktest.parse_manifest_line(
            ['x.pas', '0xE3CCCE:20', 'loop-align=off', 'a5-always=FOO'])
        self.assertEqual(u.specs, ['0xE3CCCE:20'])
        self.assertEqual(u.dpc_opts, ['--loop-align=off', '--a5-always=FOO'])

    def test_module_routines_kept(self):
        u = linktest.parse_manifest_line(
            ['x.pas', 'DISK_$UNASSIGN_ALL', 'module-routines=DISK_$UNASSIGN'])
        self.assertEqual(u.mods, ['module-routines=DISK_$UNASSIGN'])
        self.assertEqual(u.dpc_opts, [])

    def test_code_section_addr_ignored(self):
        u = linktest.parse_manifest_line(
            ['scsi.pas', 'SCSI_$CINIT', '--section-addr', '.text.SCSI_.SCSI_WIRED_PROC=0xE88800'])
        self.assertEqual(u.specs, ['SCSI_$CINIT'])

    def test_refused(self):
        for toks in (['x.s', 'FOO'],
                     ['x.pas'],
                     ['x.pas', 'FOO', '--section-addr', '.bss.moddata.PROC2=0xE7BE84'],
                     ['x.pas', 'FOO', 'module-routines=BAR,LOCAL=0xE43182'],
                     ['x.pas', '0xE76470:2E', 'enclosing=RECEIVE_PACKET']):
            with self.assertRaises(linktest.SetupError, msg=toks):
                linktest.parse_manifest_line(toks)

    def test_refused_lines_collected(self):
        d = tempfile.mkdtemp(prefix='linktest-manifest-', dir=_tmp())
        try:
            m = os.path.join(d, 'm.txt')
            with open(m, 'w') as f:
                f.write('# comment\n'
                        'a.pas FOO\n'
                        'b.pas 0xE76470:2E enclosing=RECEIVE_PACKET\n'
                        'c.pas BAR module-routines=X=0xE43182\n')
            with self.assertRaises(linktest.SetupError):
                linktest.read_manifest(m)
            refused = []
            units = linktest.read_manifest(m, refused)
            self.assertEqual([u.path for u in units], ['a.pas'])
            self.assertEqual([r[0] for r in refused], ['b.pas', 'c.pas'])
            self.assertIn('enclosing', refused[0][1])
        finally:
            shutil.rmtree(d)

    def test_checked_in_manifest_parses(self):
        units = linktest.read_manifest(linktest.MANIFEST)
        self.assertTrue(units)
        for u in units:
            self.assertTrue(os.path.exists(os.path.join(linktest.REPO, u.path)), u.path)


NM = """\
build/sau2/proc1/tst_lock.o:00000000 T PROC1_$TST_LOCK
build/sau2/proc1/get_locks.o:00000000 T PROC1_$GET_LOCKS
build/sau2/tone/time.o:00000000 T TONE_$TIME
build/sau2/tone/time.o:00000004 b tone_wait_type
build/sau2/xpd/ptrace_opts.o:00000000 T XPD_$RESET_PTRACE_OPTS
build/sau2/xpd/ptrace_opts.o:00000010 T XPD_$SET_PTRACE_OPTS
build/sau2/proc1/sau2/set_lock.o:00000000 T PROC1_$SET_LOCK
build/sau2/misc/zero_page.o:00000000 T ZERO_PAGE
build/sau2/misc/zero_page.o:00000000 W weakling
"""


class DropTest(unittest.TestCase):
    def setUp(self):
        self.defs = linktest.parse_nm(NM)

    def test_parse_nm(self):
        self.assertEqual(self.defs['build/sau2/tone/time.o'], {'TONE_$TIME': 'T'})
        self.assertEqual(self.defs['build/sau2/misc/zero_page.o'], {'ZERO_PAGE': 'T'})

    def test_whole_objects(self):
        drops, errs, notes = linktest.plan_drops(
            {'PROC1_$TST_LOCK': 'T', 'PROC1_$GET_LOCKS': 'T'}, self.defs)
        self.assertEqual(sorted(drops), ['build/sau2/proc1/get_locks.o',
                                         'build/sau2/proc1/tst_lock.o'])
        self.assertEqual((errs, notes), ([], []))

    def test_local_symbols_do_not_block(self):
        drops, errs, _ = linktest.plan_drops({'TONE_$TIME': 'T'}, self.defs)
        self.assertEqual((drops, errs), (['build/sau2/tone/time.o'], []))

    def test_object_with_other_globals(self):
        drops, errs, _ = linktest.plan_drops({'XPD_$RESET_PTRACE_OPTS': 'T'}, self.defs)
        self.assertEqual(drops, [])
        self.assertIn('XPD_$SET_PTRACE_OPTS', errs[0])

    def test_hand_assembly(self):
        drops, errs, _ = linktest.plan_drops(
            {'PROC1_$SET_LOCK': 'T'}, self.defs, {'build/sau2/proc1/sau2/set_lock.o'})
        self.assertEqual(drops, [])
        self.assertIn('hand-written assembly', errs[0])

    def test_added(self):
        drops, errs, notes = linktest.plan_drops({'IO_$ALLOC': 'T'}, self.defs)
        self.assertEqual((drops, errs), ([], []))
        self.assertIn('IO_$ALLOC', notes[0])


class FakeMap:
    def __init__(self, syms):
        self.syms = {n: sau2map.Symbol(n, a) for n, a in syms.items()}

    def lookup(self, name):
        return self.syms.get(name)


class AbsoluteTest(unittest.TestCase):
    def test_plan(self):
        lm = FakeMap({'AST_$ZERO_BUFF': 0xFF8C00, 'ML_$LOCK': 0xE20B12})
        absdefs, errs = linktest.plan_absolutes(
            {'AST_$ZERO_BUFF', 'ML_$LOCK', 'MMU_$INSTALL', 'NOWHERE'},
            {'MMU_$INSTALL'}, lm)
        self.assertEqual(absdefs, {'AST_$ZERO_BUFF': 0xFF8C00})
        self.assertEqual(len(errs), 2)
        self.assertTrue(any('ML_$LOCK' in e and '0xE20B12' in e for e in errs))

    def test_source(self):
        s = linktest.absdefs_s({'AST_$ZERO_BUFF': 0xFF8C00})
        self.assertIn('.globl  AST_$ZERO_BUFF', s)
        self.assertIn('.equ    AST_$ZERO_BUFF, 0x00FF8C00', s)


MAKEFILE = """\
OBJS :=
OBJS += a.o b.o
ALL_SUBSYSTEM_OBJS := $(OBJS) c.o

# Main target
link: $(ALL_SUBSYSTEM_OBJS)
\t@echo $^
show:
\t@echo $(ALL_SUBSYSTEM_OBJS)
"""


def _tmp():
    t = os.path.join(linktest.REPO, 'tmp')
    return t if os.path.isdir(t) else None


def _snapshot(root):
    """{relative path: (mtime, bytes)} of every file under root."""
    snap = {}
    for dp, dns, fns in os.walk(root):
        for n in fns:
            p = os.path.join(dp, n)
            with open(p, 'rb') as f:
                snap[os.path.relpath(p, root)] = (os.stat(p).st_mtime_ns, f.read())
    return snap


class RefreshTest(unittest.TestCase):
    """The scratch copy is written; ../source (here a fake one) is only
    read, in both the working-tree and the --rev mode."""

    def test_rsync_direction(self):
        cmd = linktest.rsync_cmd('/src/domain_os', '/work/source/domain_os')
        self.assertEqual(cmd[-2:], ['/src/domain_os/', '/work/source/domain_os/'])
        self.assertIn('--delete', cmd)
        self.assertIn('--exclude=/build', cmd)
        self.assertNotIn('--delete-excluded', cmd)
        self.assertEqual(cmd[1], '-rlpc')    # no -t: changed files get a new mtime

    @unittest.skipIf(shutil.which('rsync') is None or shutil.which('git') is None,
                     'no rsync or git')
    def test_source_untouched(self):
        d = tempfile.mkdtemp(prefix='linktest-refresh-', dir=_tmp())
        try:
            src = os.path.join(d, 'source')
            os.makedirs(os.path.join(src, 'domain_os', 'build'))
            with open(os.path.join(src, 'domain_os', 'Makefile'), 'w') as f:
                f.write(MAKEFILE)
            with open(os.path.join(src, 'domain_os', 'a.c'), 'w') as f:
                f.write('int a;\n')
            with open(os.path.join(src, 'domain_os', 'build', 'a.o'), 'w') as f:
                f.write('object\n')
            git = ['git', '-C', src, '-c', 'user.name=t', '-c', 'user.email=t@t']
            subprocess.run(git + ['init', '-q'], check=True)
            subprocess.run(git + ['add', 'domain_os/Makefile', 'domain_os/a.c'], check=True)
            subprocess.run(git + ['commit', '-qm', 'x'], check=True)
            with open(os.path.join(src, 'domain_os', 'a.c'), 'w') as f:
                f.write('int a = 1; /* uncommitted */\n')
            before = _snapshot(src)
            work = os.path.join(d, 'work')
            os.makedirs(work)

            root = linktest.refresh(src, work)
            self.assertTrue(root.startswith(work))
            with open(os.path.join(root, 'a.c')) as f:
                self.assertIn('uncommitted', f.read())
            self.assertIn('-include linktest.mk', open(os.path.join(root, 'Makefile')).read())
            self.assertFalse(os.path.exists(os.path.join(root, 'build', 'a.o')))
            # scratch build/ survives a refresh
            os.makedirs(os.path.join(root, 'build'), exist_ok=True)
            open(os.path.join(root, 'build', 'keep.o'), 'w').close()

            root = linktest.refresh(src, work, rev='HEAD')
            with open(os.path.join(root, 'a.c')) as f:
                self.assertEqual(f.read(), 'int a;\n')
            self.assertEqual(_snapshot(src), before)
            # back to the working tree: the older working-tree a.c must come
            # out newer than an object built from the HEAD version
            obj = os.path.join(root, 'build', 'a.o')
            open(obj, 'w').close()
            os.utime(os.path.join(src, 'domain_os', 'a.c'), (1, 1))
            before = _snapshot(src)
            root = linktest.refresh(src, work)
            self.assertGreaterEqual(os.stat(os.path.join(root, 'a.c')).st_mtime_ns,
                                    os.stat(obj).st_mtime_ns)
            self.assertTrue(os.path.exists(os.path.join(root, 'build', 'keep.o')))
            self.assertEqual(_snapshot(src), before)
        finally:
            shutil.rmtree(d)


class HookTest(unittest.TestCase):
    def test_patch_idempotent(self):
        once = linktest.patch_makefile(MAKEFILE)
        self.assertIn('-include linktest.mk', once)
        self.assertLess(once.index('-include linktest.mk'), once.index('# Main target'))
        self.assertEqual(linktest.patch_makefile(once), once)

    def test_patch_needs_anchor(self):
        with self.assertRaises(linktest.SetupError):
            linktest.patch_makefile(MAKEFILE.replace('# Main target', '# Main'))

    def test_check_swap(self):
        self.assertEqual(linktest.check_swap(['a.o', 'x.o'], ['b.o'], ['x.o']), [])
        self.assertEqual(len(linktest.check_swap(['a.o', 'b.o'], ['b.o'], ['x.o'])), 2)

    @unittest.skipIf(shutil.which('make') is None, 'no make')
    def test_hook_under_make(self):
        """The hook swaps objects both in prerequisites and in recipes."""
        d = tempfile.mkdtemp(prefix='linktest-hook-',
                             dir=os.path.join(linktest.REPO, 'tmp')
                             if os.path.isdir(os.path.join(linktest.REPO, 'tmp')) else None)
        try:
            for n in ('a.o', 'b.o', 'c.o', 'x.o'):
                open(os.path.join(d, n), 'w').close()
            with open(os.path.join(d, 'Makefile'), 'w') as f:
                f.write(linktest.patch_makefile(MAKEFILE))

            def make(target):
                return subprocess.run(['make', '-s', target], cwd=d, capture_output=True,
                                      text=True, check=True).stdout.split()
            # without linktest.mk (-include): unchanged
            self.assertEqual(make('link'), ['a.o', 'b.o', 'c.o'])
            with open(os.path.join(d, 'linktest.mk'), 'w') as f:
                f.write(linktest.linktest_mk(['b.o'], ['x.o']))
            self.assertEqual(make('link'), ['a.o', 'c.o', 'x.o'])
            self.assertEqual(make('show'), ['a.o', 'c.o', 'x.o'])
            self.assertEqual(make('linktest-objs'), [])
            self.assertEqual(linktest.make_var(d, 'LINKTEST_ALL_OBJS'), ['a.o', 'b.o', 'c.o'])
            self.assertEqual(linktest.make_var(d, 'ALL_SUBSYSTEM_OBJS'), ['a.o', 'c.o', 'x.o'])
        finally:
            shutil.rmtree(d)


if __name__ == '__main__':
    unittest.main()
