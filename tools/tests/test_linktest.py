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

    def test_data_section_addr_kept(self):
        u = linktest.parse_manifest_line(
            ['x.pas', 'PROC1_$GET_LOADAV', '--section-addr',
             '.bss.moddata.PROC1_.WIRED_DATA=0xE254E8'])
        self.assertEqual(u.specs, ['PROC1_$GET_LOADAV'])
        self.assertEqual(u.data_addrs, {'.bss.moddata.PROC1_.WIRED_DATA': 0xE254E8})

    def test_refused(self):
        for toks in (['x.s', 'FOO'],
                     ['x.pas'],
                     ['x.pas', 'FOO', '--section-addr', '.data.X=0xE7BE84'],
                     ['x.pas', 'FOO', '--section-addr', '.bss.moddata.P=0xE7BE84',
                      '--section-addr', '.bss.moddata.P=0xE7BE84'],
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

    def test_data_addresses_agree_with_conformance(self):
        # a block's image address is the one the conformance gate proves
        conf = {}
        with open(os.path.join(linktest.REPO, 'tests', 'conformance', 'manifest.txt')) as f:
            for line in f:
                toks = line.split('#', 1)[0].split()
                for i, tok in enumerate(toks[:-1]):
                    if tok == '--section-addr':
                        conf.setdefault(toks[0], {})[toks[i + 1]] = True
        units = linktest.read_manifest(linktest.MANIFEST)
        self.assertTrue(any(u.data_addrs for u in units))
        for u in units:
            for sec, addr in u.data_addrs.items():
                self.assertIn('%s=0x%X' % (sec, addr), conf.get(u.path, {}), u.path)


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


# A made-up module M with map routines M_$A, M_$B (the unit) and C helpers
# without map names: m_$tail (in a definer), m_$shared (its own object,
# used only by the module), m_$util (its own object, also used outside).
NM_WHOLE = """\
build/sau2/m/a.o:00000000 T M_$A
build/sau2/m/a.o:00000020 T m_$tail
build/sau2/m/b.o:00000000 T M_$B
build/sau2/m/shared.o:00000000 T m_$shared
build/sau2/m/util.o:00000000 T m_$util
build/sau2/m/other.o:00000000 T M_$ELSEWHERE
build/sau2/m/other.o:00000010 T m_$other_helper
build/sau2/n/user.o:00000000 T N_$USER
build/sau2/n/nhelp.o:00000000 T n_$help
"""
NM_U = """\
build/sau2/m/a.o:         U m_$shared
build/sau2/m/a.o:         U m_$util
build/sau2/m/b.o:         U m_$tail
build/sau2/m/b.o:         U CRASH_SYSTEM
build/sau2/n/user.o:         U m_$util
build/sau2/n/user.o:         U n_$help
"""
MAP_NAMES = {'M_$A', 'M_$B', 'M_$ELSEWHERE', 'N_$USER', 'CRASH_SYSTEM'}


class WholeModuleDropTest(unittest.TestCase):
    def setUp(self):
        self.defs = linktest.parse_nm(NM_WHOLE)
        self.refs = linktest.parse_nm_undefined(NM_U)
        self.whole = (self.refs, MAP_NAMES.__contains__)

    def test_parse_nm_undefined(self):
        self.assertEqual(self.refs['build/sau2/m/b.o'], {'m_$tail', 'CRASH_SYSTEM'})
        self.assertNotIn('build/sau2/m/shared.o', self.refs)

    def test_fragment_keeps_refusal(self):
        _, errs, _ = linktest.plan_drops({'M_$A': 'T', 'M_$B': 'T'}, self.defs)
        self.assertEqual(len(errs), 1)
        self.assertIn('m_$tail', errs[0])

    def test_helpers_dropped(self):
        drops, errs, notes = linktest.plan_drops(
            {'M_$A': 'T', 'M_$B': 'T'}, self.defs, (), self.whole)
        self.assertEqual(errs, [])
        # a.o with its helper, b.o, and shared.o (used only by a.o); not
        # util.o (n/user.o uses it), nor other.o (a map name), nor
        # n/nhelp.o (another directory)
        self.assertEqual(sorted(drops), ['build/sau2/m/a.o', 'build/sau2/m/b.o',
                                         'build/sau2/m/shared.o'])
        self.assertTrue(any('m_$tail' in n for n in notes))
        self.assertTrue(any('m_$shared' in n for n in notes))

    def test_helper_still_referenced(self):
        defs = dict(self.defs)
        refs = dict(self.refs)
        refs['build/sau2/n/user.o'] = refs['build/sau2/n/user.o'] | {'m_$tail'}
        _, errs, _ = linktest.plan_drops({'M_$A': 'T', 'M_$B': 'T'}, defs, (),
                                         (refs, MAP_NAMES.__contains__))
        self.assertEqual(len(errs), 1)
        self.assertIn('build/sau2/n/user.o still references', errs[0])

    def test_map_name_outside_unit_refused(self):
        drops, errs, _ = linktest.plan_drops({'M_$A': 'T', 'M_$B': 'T',
                                              'm_$other_helper': 'T'},
                                             self.defs, (), self.whole)
        self.assertNotIn('build/sau2/m/other.o', drops)
        self.assertTrue(any('M_$ELSEWHERE' in e and 'map names' in e for e in errs))

    def test_hand_assembly_never_dropped(self):
        drops, _, _ = linktest.plan_drops(
            {'M_$A': 'T', 'M_$B': 'T'}, self.defs, {'build/sau2/m/shared.o'}, self.whole)
        self.assertNotIn('build/sau2/m/shared.o', drops)

    def test_chain_of_helpers(self):
        # shared.o's own helper, in a fourth object, goes too
        defs = linktest.parse_nm(NM_WHOLE + 'build/sau2/m/deep.o:00000000 T m_$deep\n')
        refs = linktest.parse_nm_undefined(NM_U + 'build/sau2/m/shared.o:   U m_$deep\n')
        drops, errs, _ = linktest.plan_drops({'M_$A': 'T', 'M_$B': 'T'}, defs, (),
                                             (refs, MAP_NAMES.__contains__))
        self.assertEqual(errs, [])
        self.assertIn('build/sau2/m/deep.o', drops)


NM_S = """\
build/sau2/proc1/proc1_data.o:00000000 00000cc4 B PROC1_$DATA
build/sau2/cal/cal_data.o:00000000 0000000c B CAL_$TIMEZONE
build/sau2/term/term_data.o:00000000 00001398 D TERM_$DATA
build/sau2/proc1/tst_lock.o:00000000 0000001c T PROC1_$TST_LOCK
build/sau2/mst/mst_data.o:00000000 00000030 D MST_$PAGE_AVAIL_BITMAP
"""

SYMTAB = """\

linktest/scsi.o:     file format elf32-m68k

SYMBOL TABLE:
00000000 l    df *ABS*\t00000000 scsi.pas
00000000 l    d  .bss.moddata.SCSI_.SCSI_WIRED_DATA\t00000000 .bss.moddata.SCSI_.SCSI_WIRED_DATA
00000012 l       *ABS*\t00000000 __dpc_data_size.SCSI_WIRED_DATA
00000000 g       .text.SCSI_.OS_INIT_PROC\t00000000 SCSI_$CINIT
00000004 g       .bss.moddata.SCSI_.SCSI_WIRED_DATA\t00000000 SCSI_$PRESENT
00000000         *UND*\t00000000 PROC1_$DATA
"""


class DataTest(unittest.TestCase):
    def setUp(self):
        self.c_data = linktest.parse_nm_sizes(NM_S)

    def test_pascal_data_section(self):
        self.assertEqual(linktest.pascal_data_section('.bss.moddata.PROC2'), '.data')
        self.assertEqual(linktest.pascal_data_section('.bss.moddata.PROC1_.WIRED_DATA'),
                         'WIRED_DATA')
        with self.assertRaises(linktest.SetupError):
            linktest.pascal_data_section('.data.X')
        self.assertEqual(linktest.data_size_symbol('.data'), '__dpc_data_size')
        self.assertEqual(linktest.data_size_symbol('wired_data'),
                         '__dpc_data_size.WIRED_DATA')

    def test_parse_nm_sizes(self):
        self.assertEqual(self.c_data['PROC1_$DATA'],
                         ('build/sau2/proc1/proc1_data.o', 0xCC4))
        self.assertEqual(self.c_data['TERM_$DATA'][1], 0x1398)
        self.assertNotIn('PROC1_$TST_LOCK', self.c_data)     # code

    def test_anchor_symbols(self):
        self.assertEqual(linktest.anchor_symbols({'.data.TERM_$DATA': 0xE2C9F0,
                                                  '.bss.NAME_$DATA': 0xE80264,
                                                  '.text.FOO': 1}),
                         {'TERM_$DATA': 0xE2C9F0, 'NAME_$DATA': 0xE80264})

    def test_owner_block(self):
        self.assertEqual(linktest.data_owner(0xE254E8, {'PROC1_$DATA': 0xE254E8}, {},
                                             [], self.c_data),
                         ('PROC1_$DATA', 'MODULE_DATA_DEFINE'))

    def test_owner_map_name(self):
        self.assertEqual(linktest.data_owner(0xE7B030, {}, {}, ['CAL_$TIMEZONE'],
                                             self.c_data),
                         ('CAL_$TIMEZONE', 'map name'))

    def test_owner_anchor(self):
        self.assertEqual(linktest.data_owner(0xE2C9F0, {}, {'TERM_$DATA': 0xE2C9F0},
                                             [], self.c_data),
                         ('TERM_$DATA', 'DATA_ANCHORS'))

    def test_owner_none(self):
        # a map name no C object defines (SCSI_$DATA_START is a link mark),
        # a block whose C object is not linked, a C object with no image
        # address (MST_$PAGE_AVAIL_BITMAP: placed by its users)
        owner, why = linktest.data_owner(0xE88804, {'GONE': 0xE88804}, {},
                                         ['SCSI_$DATA_START'], self.c_data)
        self.assertIsNone(owner)
        self.assertIn('0xE88804', why)

    def test_owner_ambiguous(self):
        owner, why = linktest.data_owner(0xE254E8, {'PROC1_$DATA': 0xE254E8}, {},
                                         ['CAL_$TIMEZONE'], self.c_data)
        self.assertIsNone(owner)
        self.assertIn('CAL_$TIMEZONE', why)

    def test_same_owner_found_twice(self):
        self.assertEqual(linktest.data_owner(0xE254E8, {'PROC1_$DATA': 0xE254E8}, {},
                                             ['PROC1_$DATA'], self.c_data)[0],
                         'PROC1_$DATA')

    def test_extent(self):
        self.assertIsNone(linktest.extent_error('s', 0xE254E8, 0xC, 'PROC1_$DATA', 'o', 0xCC4))
        self.assertIsNone(linktest.extent_error('s', 0xE254E8, 0xCC4, 'PROC1_$DATA', 'o', 0xCC4))
        e = linktest.extent_error('s', 0xE7B030, 0x12, 'CAL_$TIMEZONE', 'o', 0xC)
        self.assertIn('0x12', e)

    def test_symtab(self):
        st = linktest.parse_symtab(SYMTAB)
        self.assertIn(('SCSI_$PRESENT', 4, 'g', '.bss.moddata.SCSI_.SCSI_WIRED_DATA'), st)
        self.assertIn(('PROC1_$DATA', 0, '', '*UND*'), st)
        defines, declared = linktest.unit_data(st, {'.bss.moddata.SCSI_.SCSI_WIRED_DATA': 4})
        self.assertEqual(defines, {'.bss.moddata.SCSI_.SCSI_WIRED_DATA': {'SCSI_$PRESENT': 4}})
        self.assertEqual(declared, {'__dpc_data_size.SCSI_WIRED_DATA': 0x12})

    def test_define_mismatches(self):
        where = {'A': [(0x1004, 'B')], 'B': [(0x1010, 'B')]}
        errs, unnamed = linktest.define_mismatches({'A': 4, 'B': 8, 'C': 2}, 0x1000, where)
        self.assertEqual(unnamed, ['C'])
        self.assertEqual(len(errs), 1)
        self.assertIn('B', errs[0])

    def test_a5_offsets(self):
        asm = ("PKT_$NEXT_ID:\n\tlink.w\t%fp,#-4\n\tpea\t(%a5)\n"
               "\tlea\t(.Ldata).l,%a5\n\tpea\t(0x50,%a5)\n"
               "\tmove.w\t(0x5c,%a5),%d2\n\tmove.l\t(0x3c,%a5,%d0.w),%d0\n"
               "\tlea\t(%a5),%a0\t| var at 0\n\tmovem.l\t(-0xc,%fp),%d2/%a5\n"
               "\tmovea.l\t(-4,%fp),%a5\n")
        self.assertEqual(linktest.a5_offsets(asm), {0x50, 0x5C, 0x3C, 0})
        # the save alone is no data operand
        self.assertEqual(linktest.a5_offsets(
            "\tpea\t(%a5)\n\tlea\t(.Ldata.WIRED_DATA).l,%a5\n"), set())
        # but a `pea (%a5)' that passes the variable at offset 0 is
        self.assertEqual(linktest.a5_offsets("\tpea\t(%a5)\n\tjsr\tX\n"), {0})

    def test_reloc_addends(self):
        text = ("\t\t\t4: R_68K_32\tPKT_$DATA+0x50\n\t\t\ta: R_68K_32\tML_$SPIN_LOCK\n"
                "\t\t\t10: R_68K_32\tPKT_$DATA+0x5c\n\t\t\t12: R_68K_32\tPKT_$DATA\n"
                "\t\t\t14: R_68K_32\tPKT_$DATA_X+0x4\n")
        self.assertEqual(linktest.reloc_addends(text, 'PKT_$DATA'), {0x50, 0x5C, 0})
        self.assertEqual(linktest.offset_evidence({0x3C, 0x50}, {0, 0x50}),
                         ([0x50], [0x3C]))

    def test_unverified_count(self):
        self.assertEqual(linktest.unverified_count(
            'asm_compare: 3 object(s) ... 7 resolved to the image operand, 0 unverified; 1'), 0)
        self.assertEqual(linktest.unverified_count('x, 2 unverified; y'), 2)
        self.assertIsNone(linktest.unverified_count('crashed'))


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
