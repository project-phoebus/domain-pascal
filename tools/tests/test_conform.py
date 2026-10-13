"""End-to-end tests for tools/conform.py against the SR10.2 SAU2 image.

The .s files used here are harness fixtures (tests/conformance/handasm/),
hand-transcribed from the image, not compiler output.  Skipped when the
image, the map or the m68k-elf toolchain is absent.
"""

import contextlib
import io
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import conform  # noqa: E402
import sau2map  # noqa: E402

HANDASM = os.path.join(conform.REPO, 'tests', 'conformance', 'handasm')
SCRATCH = os.path.join(conform.REPO, 'tmp', 'conform-tests')
MISSING = conform.missing_prereqs()
DPC = os.path.join(conform.REPO, 'target', 'debug', 'dpc')


@unittest.skipIf(MISSING, 'missing: %s' % ', '.join(MISSING))
class ConformTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.map = sau2map.load()
        os.makedirs(SCRATCH, exist_ok=True)

    def run_check(self, path, specs, **kw):
        out = []
        ok = conform.check(path, specs, self.map, out=out.append, **kw)
        return ok, '\n'.join(out)

    def variant(self, src, name, old, new):
        """A copy of fixture `src' with `old' replaced by `new'."""
        with open(os.path.join(HANDASM, src)) as f:
            text = f.read()
        self.assertIn(old, text)
        path = os.path.join(SCRATCH, name)
        with open(path, 'w') as f:
            f.write(text.replace(old, new))
        return path

    # -- PROC1_$TST_LOCK 0xE148CA..0xE148E6 ------------------------------
    def test_tst_lock_passes(self):
        ok, out = self.run_check(os.path.join(HANDASM, 'proc1_tst_lock.s'),
                                 ['PROC1_$TST_LOCK'])
        self.assertTrue(ok, out)
        self.assertEqual(out, 'OK PROC1_$TST_LOCK 0xE148CA 0x1C  '
                              'tests/conformance/handasm/proc1_tst_lock.s')

    def test_tst_lock_abs_long(self):
        # gas must emit 2079 00e1 eac8 for the absolute-long movea.l
        ok, _ = self.run_check(os.path.join(HANDASM, 'proc1_tst_lock.s'),
                               ['PROC1_$TST_LOCK'], quiet=True)
        self.assertTrue(ok)
        work = conform.scratch_dir(os.path.join(HANDASM, 'proc1_tst_lock.s'))
        with open(os.path.join(work, 'code.bin'), 'rb') as f:
            self.assertEqual(f.read()[4:10], bytes.fromhex('207900e1eac8'))

    def test_tst_lock_frame_size_fails(self):
        path = self.variant('proc1_tst_lock.s', 'tst_lock_link8.s',
                            'link.w\t%fp,#-4', 'link.w\t%fp,#-8')
        ok, out = self.run_check(path, ['PROC1_$TST_LOCK'])
        self.assertFalse(ok)
        self.assertIn('bytes differ first at 0xE148CD', out)
        self.assertIn('* e148ca', out)

    def test_tst_lock_wrong_symbol_fails(self):
        path = self.variant('proc1_tst_lock.s', 'tst_lock_sym.s',
                            '(PROC1_$CURRENT_PCB).l', '(PROC1_$READY_COUNT).l')
        ok, out = self.run_check(path, ['PROC1_$TST_LOCK'])
        self.assertFalse(ok)
        self.assertIn('R_68K_32 names PROC1_$READY_COUNT', out)
        self.assertIn('is PROC1_$CURRENT_PCB', out)

    def test_unknown_external_is_an_error(self):
        path = self.variant('proc1_tst_lock.s', 'tst_lock_unknown.s',
                            '(PROC1_$CURRENT_PCB).l', '(NOT_IN_THE_MAP).l')
        with self.assertRaisesRegex(conform.SetupError, 'NOT_IN_THE_MAP'):
            self.run_check(path, ['PROC1_$TST_LOCK'])

    def test_wrong_length_fails(self):
        ok, out = self.run_check(os.path.join(HANDASM, 'proc1_tst_lock.s'),
                                 ['PROC1_$TST_LOCK:1E'])
        self.assertFalse(ok)
        self.assertIn('size:', out)

    def test_missing_last_instruction_fails(self):
        path = self.variant('proc1_tst_lock.s', 'tst_lock_short.s', '\trts\t', '|')
        ok, out = self.run_check(path, ['PROC1_$TST_LOCK'])
        self.assertFalse(ok)
        self.assertIn('size: ours 0x1A bytes', out)

    def test_trailing_junk_fails(self):
        path = self.variant('proc1_tst_lock.s', 'tst_lock_long.s', '\trts\t', '\trts\n\tnop\t')
        ok, out = self.run_check(path, ['PROC1_$TST_LOCK'])
        self.assertFalse(ok)
        self.assertIn('size: ours 0x1E bytes', out)

    def test_literal_address_fails(self):
        # right bytes, but the reference is hard-coded, not named
        path = self.variant('proc1_tst_lock.s', 'tst_lock_literal.s',
                            '(PROC1_$CURRENT_PCB).l', '(0xE1EAC8).l')
        ok, out = self.run_check(path, ['PROC1_$TST_LOCK'])
        self.assertFalse(ok)
        self.assertIn('0xE148D0: literal 0xE1EAC8 (PROC1_$CURRENT_PCB) with no relocation', out)

    def test_lowercase_external_is_an_error(self):
        # external names are upper-cased; the map lookup must be exact
        path = self.variant('proc1_tst_lock.s', 'tst_lock_lower.s',
                            '(PROC1_$CURRENT_PCB).l', '(proc1_$current_pcb).l')
        with self.assertRaisesRegex(conform.SetupError, r'proc1_\$current_pcb'):
            self.run_check(path, ['PROC1_$TST_LOCK'])

    def test_local_misplaced_routine_label_fails(self):
        # same bytes, but PROC1_$TST_LOCK is a local label 4 bytes in
        path = self.variant('proc1_tst_lock.s', 'tst_lock_local.s',
                            '\t.globl\tPROC1_$TST_LOCK\nPROC1_$TST_LOCK:\n'
                            '\tlink.w\t%fp,#-4\t\t\t\t| 4e56 fffc\n',
                            '\tlink.w\t%fp,#-4\nPROC1_$TST_LOCK:\n')
        ok, out = self.run_check(path, ['PROC1_$TST_LOCK'])
        self.assertFalse(ok)
        self.assertIn('routine PROC1_$TST_LOCK is not defined as a global code symbol', out)

    def test_global_not_in_map_fails(self):
        path = self.variant('proc1_tst_lock.s', 'tst_lock_extra_global.s',
                            'PROC1_$TST_LOCK:\n', 'PROC1_$TST_LOCK:\n\t.globl\tWORKER\nWORKER:\n')
        ok, out = self.run_check(path, ['PROC1_$TST_LOCK'])
        self.assertFalse(ok)
        self.assertIn('global symbol WORKER is not in the map', out)

    def test_data_section_compared(self):
        # a data section must be placed and is then compared with the image
        # (0xE148E6 starts PROC1_$GET_LOCKS: 4e56 fffc)
        good = self.variant('proc1_tst_lock.s', 'tst_lock_data_ok.s',
                            '\trts\t', '\t.data\n\t.short\t0x4e56\n\t.text\n\trts\t')
        ok, out = self.run_check(good, ['PROC1_$TST_LOCK'])
        self.assertFalse(ok)
        self.assertIn('section .data has contents but no --section-addr', out)
        ok, out = self.run_check(good, ['PROC1_$TST_LOCK'], section_addrs={'.data': 0xE148E6})
        self.assertTrue(ok, out)
        bad = self.variant('proc1_tst_lock.s', 'tst_lock_data_bad.s',
                           '\trts\t', '\t.data\n\t.short\t0x4e57\n\t.text\n\trts\t')
        ok, out = self.run_check(bad, ['PROC1_$TST_LOCK'], section_addrs={'.data': 0xE148E6})
        self.assertFalse(ok)
        self.assertIn('section .data differs first at 0xE148E7: ours 57, image 56', out)

    def test_manifest_setup_error_exits_2(self):
        path = os.path.join(SCRATCH, 'manifest_unknown.txt')
        with open(path, 'w') as f:
            f.write('tests/conformance/handasm/proc1_tst_lock.s  NO_SUCH_ROUTINE\n')
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            rc = conform.main(['manifest', path])
        self.assertEqual(rc, 2)
        self.assertIn('NO_SUCH_ROUTINE is not in the map', buf.getvalue())

    def test_module_local_spec(self):
        ok, out = self.run_check(os.path.join(HANDASM, 'proc1_tst_lock.s'), ['0xE148CA:1C'])
        self.assertTrue(ok, out)

    # -- 0xE00EB0 + AST_$PAGE_ZERO 0xE00EB0..0xE00F16 --------------------
    def test_two_routines_pass(self):
        ok, out = self.run_check(os.path.join(HANDASM, 'ast_page_zero.s'),
                                 ['AST_$PAGE_ZERO', '0xE00EB0'])
        self.assertTrue(ok, out)
        self.assertEqual([l.split('  ')[0] for l in out.splitlines()],
                         ['OK module-local@0xE00EB0 0xE00EB0 0x3C',
                          'OK AST_$PAGE_ZERO 0xE00EEC 0x2A'])

    def test_pc_relative_relocation(self):
        # worker and caller in separate sections: the bsr.s becomes an
        # R_68K_PC8 relocation, which must land on 0xE00EB0
        path = self.variant('ast_page_zero.s', 'apz_sections.s',
                            '\t.text\n', '\t.section .text.worker,"ax"\n')
        with open(path) as f:
            text = f.read().replace('\t.globl\tAST_$PAGE_ZERO',
                                    '\t.section .text.caller,"ax"\n\t.globl\tAST_$PAGE_ZERO')
        with open(path, 'w') as f:
            f.write(text)
        ok, out = self.run_check(path, ['0xE00EB0', 'AST_$PAGE_ZERO'])
        self.assertTrue(ok, out)
        # one more instruction ahead of the caller moves it: placement, size,
        # bytes and the PC8 target all fail
        with open(path, 'w') as f:
            f.write(text.replace('\t.section .text.caller,"ax"\n',
                                 '\t.section .text.caller,"ax"\n\tnop\n'))
        ok, out = self.run_check(path, ['0xE00EB0', 'AST_$PAGE_ZERO'])
        self.assertFalse(ok)
        self.assertIn('symbol AST_$PAGE_ZERO linked at 0xE00EEE, map has 0xE00EEC', out)

    def test_not_contiguous_is_an_error(self):
        with self.assertRaisesRegex(conform.SetupError, 'not contiguous'):
            self.run_check(os.path.join(HANDASM, 'ast_page_zero.s'),
                           ['0xE00EB0:3A', 'AST_$PAGE_ZERO'])

    def test_scratch_dir_per_unit_path(self):
        a = conform.scratch_dir(os.path.join(HANDASM, 'proc1_tst_lock.s'))
        b = conform.scratch_dir(os.path.join(conform.REPO, 'tests', 'conformance',
                                             'proc1_tst_lock.pas'))
        self.assertNotEqual(a, b)
        self.assertTrue(a.startswith(conform.SCRATCH + os.sep))

    # -- fragments: PKT_$RECENTLY_MISSING 0xE128BA (2 mod 4) ---------------
    # A compiled routine starting at 2 mod 4 whose loop head 0xE128DC is
    # 4-aligned by the filler at 0xE128DA: alignment is absolute.
    def test_fragment_filler_lands_on_absolute_alignment(self):
        path = os.path.join(HANDASM, 'pkt_recently_missing.s')
        ok, out = self.run_check(path, ['PKT_$RECENTLY_MISSING'])
        self.assertTrue(ok, out)
        self.assertEqual(out, 'OK PKT_$RECENTLY_MISSING 0xE128BA 0x3C  '
                              'tests/conformance/handasm/pkt_recently_missing.s')
        # code.bin keeps the 2 pad bytes, linked at 0xE128B8
        with open(os.path.join(conform.scratch_dir(path), 'code.bin'), 'rb') as f:
            code = f.read()
        self.assertEqual(len(code), 2 + 0x3C)
        self.assertEqual(code[2 + 0xE128DA - 0xE128BA:][:2], bytes.fromhex('2048'))

    def test_fragment_without_origin_misplaces_filler(self):
        # the same .balignw, relative to a section that starts at 2 mod 4:
        # offset 0x20 is already aligned, so the filler is left out
        path = self.variant('pkt_recently_missing.s', 'pkt_no_origin.s',
                            '\t.set\t__dpc_fragment_origin,0xe128ba\n', '')
        with open(path) as f:
            text = f.read()
        with open(path, 'w') as f:
            f.write(text.replace('\t.space\t2', '|'))
        ok, out = self.run_check(path, ['PKT_$RECENTLY_MISSING'])
        self.assertFalse(ok)
        self.assertIn('size: ours 0x3A bytes', out)
        # (the bmi.s over the loop is 2 shorter, so the bytes differ first at
        # its displacement 0xE128D5); the filler's place now holds the loop
        self.assertIn('bytes differ first at 0xE128D5', out)
        self.assertRegex(out, r'\* e128da  b2a8 fff8 .*\| 2048 ')

    def test_fragment_origin_must_match_unit(self):
        path = self.variant('pkt_recently_missing.s', 'pkt_wrong_origin.s',
                            '__dpc_fragment_origin,0xe128ba', '__dpc_fragment_origin,0xe128bc')
        with self.assertRaisesRegex(conform.SetupError, 'fragment at 0xE128BC'):
            self.run_check(path, ['PKT_$RECENTLY_MISSING'])

    def test_fragment_pad_must_be_zero(self):
        # code in the pad is linked below the unit and would escape the
        # byte comparison
        path = self.variant('pkt_recently_missing.s', 'pkt_code_in_pad.s',
                            '\t.space\t2', '\tnop')
        ok, out = self.run_check(path, ['PKT_$RECENTLY_MISSING'])
        self.assertFalse(ok)
        self.assertIn('fragment pad 0xE128B8..0xE128BA is not zero: 4e71', out)

    def test_fragment_args(self):
        r = conform.unit_routines(['PROC1_$TST_LOCK', 'PROC1_$GET_LOCKS'], self.map)
        self.assertEqual(conform.fragment_args(r, self.map),
                         ['--fragment-origin', '0xE148CA'])
        seg = self.map.segment_containing(0xE128BA)            # PKT, 0xE1202C..
        self.assertEqual((seg.name, seg.addr, seg.end), ('PKT', 0xE1202C, 0xE12D38))
        whole = [conform.Routine(None, seg.addr, seg.size)]
        self.assertEqual(conform.fragment_args(whole, self.map), [])
        # the tail of the module keeps the module-end padding
        tail = conform.unit_routines(['PKT_$PING_SERVER'], self.map)
        self.assertEqual(tail[-1].end, seg.end)
        self.assertEqual(conform.fragment_args(tail, self.map),
                         ['--fragment-origin', '0xE12BB8', '--fragment-ends-module'])
        # one .pas is one module
        past = [conform.Routine(None, seg.addr + 4, seg.size)]
        with self.assertRaisesRegex(conform.SetupError, 'runs past the end of module PKT'):
            conform.fragment_args(past, self.map)

    # -- a compiled multi-routine fragment of length 0x2E ------------------
    @unittest.skipUnless(os.path.exists(DPC), 'dpc not built')
    def test_compiled_fragment_has_no_module_end_padding(self):
        src = os.path.join(conform.REPO, 'tests', 'conformance', 'proc1_tst_lock.pas')
        specs = ['PROC1_$TST_LOCK', 'PROC1_$GET_LOCKS']
        work = os.path.join(SCRATCH, 'compiled_fragment')
        os.makedirs(work, exist_ok=True)
        asm = conform.compile_pas(src, DPC, work, ['--fragment-origin', '0xE148CA'])
        ok, out = self.run_check(asm, specs, work=work)
        self.assertTrue(ok, out)
        # compiled as a whole module, it gets the module-end filler: 0x30
        asm = conform.compile_pas(src, DPC, work)
        ok, out = self.run_check(asm, specs, work=work)
        self.assertFalse(ok)
        self.assertIn('size: ours 0x30 bytes', out)

    # -- module data: a NOBITS data area placed at the map's D segment -----
    @unittest.skipUnless(os.path.exists(DPC), 'dpc not built')
    def test_data_area_reference_is_checked_against_its_placement(self):
        src = os.path.join(conform.REPO, 'tests', 'conformance', 'proc1_get_loadav.pas')
        specs = ['PROC1_$GET_LOADAV']
        work = os.path.join(SCRATCH, 'data_area')
        os.makedirs(work, exist_ok=True)
        asm = conform.compile_pas(src, DPC, work, ['--fragment-origin', '0xE14BBA'])
        sec = '.bss.moddata.PROC1_'
        ok, out = self.run_check(asm, specs, work=work, section_addrs={sec: 0xE254E8})
        self.assertTrue(ok, out)
        # `lea <data>.l,A5' at 0xE14BC0: its operand must be the D segment
        ok, out = self.run_check(asm, specs, work=work, section_addrs={sec: 0xE254EC})
        self.assertFalse(ok)
        self.assertIn('0xE14BC2: R_68K_32 names %s (0xE254EC)' % sec, out)
        ok, out = self.run_check(asm, specs, work=work)
        self.assertFalse(ok)
        self.assertIn('reference into section %s, which has no --section-addr' % sec, out)

    # -- loop alignment on/off per unit (codegen-patterns 7) --------------
    def manifest(self, name, line):
        path = os.path.join(SCRATCH, name)
        with open(path, 'w') as f:
            f.write(line + '\n')
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            rc = conform.main(['manifest', path, '--dpc', DPC])
        return rc, buf.getvalue()

    @unittest.skipUnless(os.path.exists(DPC), 'dpc not built')
    def test_manifest_forwards_loop_align(self):
        unit = 'tests/conformance/ast_page_zero.pas  0xE00EB0 AST_$PAGE_ZERO'
        # the worker 0xE00EB0 comes first and aligns (filler 0xE00ED6);
        # AST_$PAGE_ZERO has no loop
        rc, out = self.manifest('ma_from.txt', unit + '  loop-align-off-from=ast_$page_zero')
        self.assertEqual(rc, 0, out)
        rc, out = self.manifest('ma_on.txt', unit + '  loop-align=on')
        self.assertEqual(rc, 0, out)
        # off: the filler is missing, so everything after it moves
        rc, out = self.manifest('ma_off.txt', unit + '  loop-align=off')
        self.assertEqual(rc, 1, out)
        self.assertIn('size: ours 0x64 bytes', out)
        self.assertIn('bytes differ first at 0xE00ED6', out)
        # an unknown routine is dpc's error, so a setup error
        rc, out = self.manifest('ma_bad.txt', unit + '  loop-align-off-from=no_such')
        self.assertEqual(rc, 2, out)
        self.assertIn('no routine `no_such`', out)

    def test_manifest_loop_align_errors(self):
        rc, out = self.manifest('ma_s.txt', 'tests/conformance/handasm/proc1_tst_lock.s  '
                                'PROC1_$TST_LOCK  loop-align=off')
        self.assertEqual(rc, 2, out)
        self.assertIn('loop-align options apply to .pas units only', out)
        rc, out = self.manifest('ma_two.txt', 'tests/conformance/proc1_tst_lock.pas  '
                                'PROC1_$TST_LOCK  loop-align=off  loop-align=on')
        self.assertEqual(rc, 2, out)
        self.assertIn('more than one', out)

    # -- which routines load A5 (codegen-patterns 4) ----------------------
    @unittest.skipUnless(os.path.exists(DPC), 'dpc not built')
    def test_manifest_forwards_a5(self):
        smd = ('tests/conformance/smd_busy_wait.pas  SMD_$PUTC SMD_$BUSY_WAIT SMD_$LITES  '
               '--section-addr .bss.moddata.SMD=0xE2E060')
        rc, out = self.manifest('a5_from.txt', smd + '  a5-always-from=smd_$busy_wait')
        self.assertEqual(rc, 0, out)
        # the per-routine list does the same
        rc, out = self.manifest('a5_list.txt', smd + '  a5-always=SMD_$BUSY_WAIT,SMD_$LITES')
        self.assertEqual(rc, 0, out)
        # used mode (the default): neither loads A5, so `pea (A5)' is missing
        rc, out = self.manifest('a5_used.txt', smd)
        self.assertEqual(rc, 1, out)
        self.assertIn('bytes differ first at 0xE1D8A2', out)
        # only SMD_$LITES: SMD_$BUSY_WAIT differs
        rc, out = self.manifest('a5_one.txt', smd + '  a5-always=SMD_$LITES')
        self.assertEqual(rc, 1, out)
        self.assertIn('bytes differ first at 0xE1D8A2', out)
        rem = ('tests/conformance/rem_name_register_server.pas  REM_NAME_$REGISTER_SERVER:1A  '
               '--section-addr .bss.moddata.REM_NAME=0xE7DBB8')
        rc, out = self.manifest('a5_abs.txt', rem + '  a5-absolute=REM_NAME_$REGISTER_SERVER')
        self.assertEqual(rc, 0, out)
        rc, out = self.manifest('a5_noabs.txt', rem)
        self.assertEqual(rc, 1, out)
        # unknown routines and conflicts are dpc's errors, so setup errors
        rc, out = self.manifest('a5_bad.txt', smd + '  a5-always=no_such')
        self.assertEqual(rc, 2, out)
        self.assertIn('--a5-always: no routine `no_such`', out)
        rc, out = self.manifest('a5_conflict.txt', rem + '  a5-always=REM_NAME_$REGISTER_SERVER'
                                '  a5-absolute=REM_NAME_$REGISTER_SERVER')
        self.assertEqual(rc, 2, out)
        self.assertIn('in both --a5-always and --a5-absolute', out)

    def test_manifest_a5_errors(self):
        rc, out = self.manifest('a5_s.txt', 'tests/conformance/handasm/proc1_tst_lock.s  '
                                'PROC1_$TST_LOCK  a5-always=PROC1_$TST_LOCK')
        self.assertEqual(rc, 2, out)
        self.assertIn('a5 options apply to .pas units only', out)
        rc, out = self.manifest('a5_two.txt', 'tests/conformance/proc1_tst_lock.pas  '
                                'PROC1_$TST_LOCK  a5-always=A  a5-always=B')
        self.assertEqual(rc, 2, out)
        self.assertIn('given more than once', out)


class A5ArgsTest(unittest.TestCase):
    def test_tokens(self):
        self.assertEqual(conform.a5_args([]), [])
        self.assertEqual(
            conform.a5_args(['a5-always-from=SMD_$BUSY_WAIT', 'a5-always=A,B',
                             'a5-absolute=C']),
            ['--a5-always-from=SMD_$BUSY_WAIT', '--a5-always=A,B', '--a5-absolute=C'])
        for bad in (['a5-always-from='], ['a5-always='], ['a5-always=A,'],
                    ['a5-absolute=,A'], ['a5-always-from=A,B'],
                    ['a5-always=A', 'a5-always=B']):
            with self.assertRaises(conform.SetupError):
                conform.a5_args(bad)


class LoopAlignArgsTest(unittest.TestCase):
    def test_tokens(self):
        self.assertEqual(conform.loop_align_args([]), [])
        self.assertEqual(conform.loop_align_args(['loop-align=off']), ['--loop-align=off'])
        self.assertEqual(conform.loop_align_args(['loop-align-off-from=VTOC_$DISMOUNT']),
                         ['--loop-align-off-from=VTOC_$DISMOUNT'])
        for bad in (['loop-align=maybe'], ['loop-align-off-from='],
                    ['loop-align=on', 'loop-align=off']):
            with self.assertRaises(conform.SetupError):
                conform.loop_align_args(bad)


if __name__ == '__main__':
    unittest.main()
