"""Tests for tools/linktest.py (no ../source, image or real dpc needed).

The full run (`make linktest') exports ../source and compiles its Pascal
tree with dpc; it is not part of `make test'.  The end-to-end tests here
use a fake ../source (a git repository made in a temporary directory)
and a fake dpc (a shell script).
"""

import contextlib
import io
import os
import shutil
import stat
import subprocess
import sys
import tempfile
import textwrap
import unittest
import unittest.mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..'))
import linktest  # noqa: E402

REFUSAL = 'not supported yet'


def diag(path, line, msg, sev='error'):
    return '%s:%d:5: %s: %s\n' % (path, line, sev, msg)


def R(module, status, kinds=()):
    return linktest.Result(module, status, len(kinds), kinds)


class DpcResultTest(unittest.TestCase):
    def test_ok(self):
        r = linktest.dpc_result('a/a.pas', 0, diag('a/x.pas', 3, 'unused', 'warning'))
        self.assertEqual((r.status, r.errors, r.kinds, r.first_kind), ('OK', 0, [], '-'))

    def test_refused_counts_errors_and_distinct_kinds(self):
        err = (diag('a/x.pas', 10, 'comparing `FOO` with 3 is ' + REFUSAL) +
               diag('a/x.pas', 12, 'comparing `BAR` with 70 is ' + REFUSAL) +
               diag('a/y.pas', 4, 'two function calls in one statement are ' + REFUSAL) +
               diag('a/y.pas', 9, 'just a warning', 'warning') +
               'some context line\n')
        r = linktest.dpc_result('a/a.pas', 1, err)
        self.assertEqual(r.status, 'REFUSED')
        self.assertEqual(r.errors, 3)
        self.assertEqual(r.kinds, ['comparing `X` with N is ' + REFUSAL,
                                   'two function calls in one statement are ' + REFUSAL])
        self.assertEqual(r.first, 'a/x.pas:10:5: comparing `FOO` with 3 is ' + REFUSAL)

    def test_plain_error_is_a_kind_too(self):
        r = linktest.dpc_result('r/r.pas', 1, diag('r/server.pas', 822, 'index out of range'))
        self.assertEqual((r.status, r.kinds), ('REFUSED', ['index out of range']))

    def test_type_phrases_folded(self):
        # every spelling TypeTable::display gives, named or spelled out
        for ty in ('acl_$rights_word_t', 'proc2_$sig_set_t', 'set of 0..7',
                   '[byte] set of 0..7', 'array[1..4] of char', '^foo_t', '(a, b)'):
            self.assertEqual(linktest.kind_of('functions returning %s are %s (codegen-patterns 3: '
                                              'result convention unknown)' % (ty, REFUSAL)),
                             'functions returning T are %s (codegen-patterns N: result '
                             'convention unknown)' % REFUSAL, ty)
            self.assertEqual(linktest.kind_of('a packed field of type %s that is not a whole '
                                              'byte, word or longword is %s' % (ty, REFUSAL)),
                             'a packed field of type T that is not a whole byte, word or '
                             'longword is ' + REFUSAL, ty)
        k = linktest.kind_of('argument type integer16 (2 bytes) does not match parameter type '
                             'integer32 (4 bytes): an integer ...')
        self.assertTrue(k.startswith('argument type T (N bytes) does not match parameter type T ('),
                        k)
        self.assertEqual(linktest.kind_of('argument type ^x_t does not match parameter type '
                                          'array[1..2] of char'),
                         'argument type T does not match parameter type T')
        self.assertEqual(linktest.kind_of('Assignment statement expression is not compatible '
                                          'with the assignment variable ((a, b) := 0..7)'),
                         'Assignment statement expression is not compatible with the '
                         'assignment variable (T := T)')
        self.assertEqual(linktest.kind_of('type transfer needs equal sizes: set of 0..7 is 2 '
                                          'bytes, integer32 is 4'),
                         'type transfer needs equal sizes: T is N bytes, T is N')
        self.assertEqual(linktest.kind_of('constant 300 is out of range for 0..255'),
                         'constant N is out of range for T')
        self.assertEqual(linktest.kind_of('this initial value of a ^t is ' + REFUSAL),
                         'this initial value of a T is ' + REFUSAL)
        # other messages: srcsurvey.kind_of alone (names in the text kept)
        self.assertIn('ROUTE_$FIND_PORTP', linktest.kind_of('at ROUTE_$FIND_PORTP 0x10'))
        self.assertEqual(linktest.kind_of('a set of 0..7 here'), 'a set of 0..7 here')

    def test_panic(self):
        err = ("thread 'main' panicked at crates/dpc-m68k/src/sel.rs:120:9:\n"
               "index out of bounds: the len is 3 but the index is 7\n"
               "note: run with `RUST_BACKTRACE=1'\n")
        r = linktest.dpc_result('m/m.pas', 101, err)
        self.assertEqual(r.status, 'FAILED')
        self.assertEqual(r.kinds, ['dpc panic: index out of bounds: the len is N but the index is N'])

    def test_signal_timeout_and_silence(self):
        self.assertEqual(linktest.dpc_result('m', -11, '').kinds, ['dpc: signal 11'])
        self.assertEqual(linktest.dpc_result('m', None, '').kinds, ['dpc: timeout'])
        self.assertEqual(linktest.dpc_result('m', 2, '').kinds, ['dpc: exit status N'])
        self.assertEqual(linktest.dpc_result('m', 2, 'dpc: cannot read x.pas\n').kinds,
                         ['dpc: dpc: cannot read x.pas'])

    def test_asm(self):
        err = ("build/a.s: Assembler messages:\n"
               "build/a.s:12: Error: operand 1 out of range\n"
               "build/a.s:40: Error: operand 1 out of range\n")
        r = linktest.asm_result('a/a.pas', err)
        self.assertEqual((r.status, r.errors), ('ASM', 2))
        self.assertEqual(r.kinds, ['as: operand N out of range'])


class RankingTest(unittest.TestCase):
    RESULTS = [R('a.pas', 'OK'),
               R('b.pas', 'REFUSED', ['k1']),
               R('c.pas', 'REFUSED', ['k1', 'k2']),
               R('d.pas', 'REFUSED', ['k2', 'k3']),
               R('e.pas', 'REFUSED', ['k3']),
               R('f.pas', 'REFUSED', ['k3', 'k2']),
               R('g.pas', 'ASM', ['as: x'])]

    def test_rank(self):
        self.assertEqual(linktest.rank_kinds(self.RESULTS),
                         [('k3', 1, 3), ('k1', 1, 2), ('as: x', 1, 1), ('k2', 0, 3)])

    def test_summary(self):
        self.assertEqual(linktest.summary_line(self.RESULTS),
                         '1 of 7 module files compile; 5 refused by dpc, 1 not assembled')
        self.assertEqual(linktest.summary_line([R('a', 'OK')]), '1 of 1 module files compile')

    def test_format_limit(self):
        lines = linktest.format_ranking(linktest.rank_kinds(self.RESULTS), 2)
        self.assertEqual(len(lines), 4)
        self.assertIn('2 more kind(s)', lines[-1])


class BaselineTest(unittest.TestCase):
    def read(self, text):
        with tempfile.NamedTemporaryFile('w', suffix='.txt', delete=False) as fh:
            fh.write(text)
        try:
            return linktest.read_baseline(fh.name)
        finally:
            os.remove(fh.name)

    def test_round_trip(self):
        res = [R('a/a.pas', 'OK'), R('b/b.pas', 'REFUSED', ['x is ' + REFUSAL, 'y'])]
        text = linktest.format_baseline('abc123', res, 'b23a505d')
        self.assertIn('# source: abc123\n# dpc: b23a505d\n', text)
        self.assertIn('REFUSED\tb/b.pas\tx is %s\n' % REFUSAL, text)
        meta, base = self.read(text)
        self.assertEqual(meta, {'source': 'abc123', 'dpc': 'b23a505d'})
        self.assertEqual(base, {'a/a.pas': ('OK', '-'), 'b/b.pas': ('REFUSED', 'x is ' + REFUSAL)})

    def test_malformed(self):
        for text, what in (('MAYBE\ta.pas\t-\n', "unknown status 'MAYBE'"),
                           ('# x\nOK a.pas -\n', ':2: malformed line'),
                           ('OK\ta.pas\n', ':1: malformed line'),
                           ('OK\ta.pas\t-\nOK\ta.pas\t-\n', ':2: module a.pas listed twice')):
            with self.assertRaises(linktest.BaselineError) as cm:
                self.read(text)
            self.assertIn(what, str(cm.exception))

    def test_compare(self):
        base = {'a.pas': ('OK', '-'), 'b.pas': ('REFUSED', 'k1'), 'c.pas': ('OK', '-'),
                'd.pas': ('REFUSED', 'k1'), 'gone.pas': ('OK', '-'), 'same.pas': ('OK', '-'),
                'e.pas': ('REFUSED', 'k1'), 'f.pas': ('FAILED', 'p'), 'g.pas': ('ASM', 'as: x'),
                'gone2.pas': ('REFUSED', 'k1')}
        now = [R('a.pas', 'REFUSED', ['k9']), R('b.pas', 'OK'), R('c.pas', 'ASM', ['as: x']),
               R('d.pas', 'REFUSED', ['k2']), R('new.pas', 'REFUSED', ['k1']),
               R('same.pas', 'OK'), R('e.pas', 'FAILED', ['dpc panic: p']),
               R('f.pas', 'REFUSED', ['k1']), R('g.pas', 'REFUSED', ['k3'])]
        lines, worse = linktest.compare_baseline(base, now)
        text = '\n'.join(lines)
        self.assertIn('WORSE a.pas: OK -> REFUSED (k9)', text)
        self.assertIn('FIXED b.pas: REFUSED (k1) -> OK', text)
        self.assertIn('WORSE c.pas: OK -> ASM (as: x)', text)
        self.assertIn('diff  d.pas: REFUSED (k1) -> REFUSED (k2)', text)
        # any drop in rank, not only from OK
        self.assertIn('WORSE e.pas: REFUSED (k1) -> FAILED (dpc panic: p)', text)
        self.assertIn('WORSE g.pas: ASM (as: x) -> REFUSED (k3)', text)
        self.assertIn('better f.pas: FAILED (p) -> REFUSED (k1)', text)
        # a module that compiled and is gone counts; one that did not, not
        self.assertIn('GONE  gone.pas: OK -> none', text)
        self.assertIn('gone  gone2.pas: REFUSED (k1) -> none', text)
        self.assertIn('new   new.pas: none -> REFUSED (k1)', text)
        self.assertNotIn('same.pas', text)
        self.assertEqual(worse, 5)          # a, c, e, g, gone
        # restricted to some modules (--only)
        lines, worse = linktest.compare_baseline(base, now[:2], {'a.pas', 'b.pas'})
        self.assertEqual((len(lines), worse), (2, 1))


class MakeLogTest(unittest.TestCase):
    def test_errors(self):
        log = textwrap.dedent("""\
            [AS] x.s
            asm_compare: note: run atomic_op_error ends at image 0xE209FA
            m68k-elf-ld: the fixup table must start past the split
            m68k-elf-ld:sau2.ld:264: undefined symbol `MMAP_$MMAPE' referenced in expression
            build/a.o: in function `F': undefined reference to `G_$H'
            build/b.o: in function `F2': undefined reference to `G_$H'
            make: *** [build/sau2/domain_os.elf] Error 1
            check_rfc: FAIL header
            """)
        lines, undef = linktest.make_errors(log)
        self.assertEqual(lines, ['m68k-elf-ld: the fixup table must start past the split',
                                 'make: *** [build/sau2/domain_os.elf] Error 1',
                                 'check_rfc: FAIL header'])
        self.assertEqual(dict(undef), {'MMAP_$MMAPE': 1, 'G_$H': 2})


class PathsTest(unittest.TestCase):
    def test_outputs(self):
        self.assertEqual(linktest.outputs('pbu/ec2_pbu.pas'),
                         ('build/sau2/pascal/pbu/ec2_pbu.s', 'build/sau2/pascal/pbu/ec2_pbu.o'))

    def test_rsync_keeps_scratch_build(self):
        cmd = linktest.rsync_cmd('/s/domain_os', '/t/domain_os/')
        self.assertEqual(cmd[-2:], ['/s/domain_os/', '/t/domain_os/'])
        for x in ('--delete', '--exclude=/build', '--exclude=/dist', '-rlpc'):
            self.assertIn(x, cmd)


# ---------------------------------------------------------------- fakes

FAKE_PASCAL_MODULES = """\
import os, sys
root = sys.argv[1] if len(sys.argv) > 1 else '.'
for d in sorted(os.listdir(root)):
    if os.path.isfile(os.path.join(root, d, d + '.pas')):
        print('%s/%s.pas' % (d, d))
"""

# A fake dpc: refuses a module whose text has REFUSE, panics on PANIC,
# else writes an empty assembly file to the -o path.
FAKE_DPC = """\
#!/bin/sh
out=""; src=""
while [ $# -gt 0 ]; do
  case "$1" in
    -o) out=$2; shift 2;;
    *) src=$1; shift;;
  esac
done
if grep -q REFUSE "$src"; then
  echo "$src:2:3: error: a thing is not supported yet" >&2
  echo "$src:4:3: error: another thing of 12 bytes is not supported yet" >&2
  echo "partial" > "$out"
  exit 1
fi
if grep -q PANIC "$src"; then
  echo "thread 'main' panicked at x.rs:1:1:" >&2; echo "boom" >&2; exit 101
fi
printf '\\t.text\\n' > "$out"
"""


def write(path, text, mode=None):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as fh:
        fh.write(text)
    if mode:
        os.chmod(path, mode)


def git(repo, *args):
    subprocess.run(['git', '-C', repo, '-c', 'user.name=t', '-c', 'user.email=t@t',
                    '-c', 'commit.gpgsign=false'] + list(args),
                   check=True, capture_output=True)


@unittest.skipUnless(shutil.which('git') and shutil.which('rsync'), 'needs git and rsync')
class EndToEndTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.src = os.path.join(self.tmp, 'source')
        dom = os.path.join(self.src, 'domain_os')
        write(os.path.join(dom, 'tools', 'pascal_modules.py'), FAKE_PASCAL_MODULES)
        write(os.path.join(dom, 'aa', 'aa.pas'), 'module aa;\n')
        write(os.path.join(dom, 'bb', 'bb.pas'), 'module bb; REFUSE\n')
        write(os.path.join(dom, 'cc', 'cc.pas'), 'module cc;\n')
        git(self.src, 'init', '-q')
        git(self.src, 'add', '-A')
        git(self.src, 'commit', '-q', '-m', 'one')
        self.dpc = os.path.join(self.tmp, 'bin', 'dpc')
        write(self.dpc, FAKE_DPC, stat.S_IRWXU)
        self.work = os.path.join(self.tmp, 'work')
        self.base = os.path.join(self.tmp, 'baseline.txt')
        self.old_as = linktest.AS
        linktest.AS = 'true'            # an `assembler' that is always there

    def tearDown(self):
        linktest.AS = self.old_as
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_tool(self, *extra):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            rc = linktest.main(['run', '--source', self.src, '--dpc', self.dpc,
                                '--work', self.work, '--baseline', self.base, '--no-make',
                                '-j', '2'] + list(extra))
        return rc, out.getvalue()

    def test_update_then_regression(self):
        rc, out = self.run_tool('--update')
        self.assertEqual(rc, 0, out)
        self.assertIn('2 of 3 module files compile; 1 refused by dpc', out)
        with open(self.base) as fh:
            text = fh.read()
        self.assertIn('OK\taa/aa.pas\t-', text)
        self.assertIn('REFUSED\tbb/bb.pas\ta thing is not supported yet', text)
        tree = os.path.join(self.work, 'source', 'domain_os')
        self.assertTrue(os.path.exists(os.path.join(tree, 'build/sau2/pascal/aa/aa.s')))
        # a refused module leaves no assembly behind (make would take it)
        self.assertFalse(os.path.exists(os.path.join(tree, 'build/sau2/pascal/bb/bb.s')))
        with open(os.path.join(self.work, 'report.txt')) as fh:
            rep = fh.read()
        self.assertIn('REFUSED  bb/bb.pas: 2 error(s); first: bb/bb.pas:2:3: a thing', rep)
        self.assertIn('another thing of N bytes is not supported yet', rep)

        # an uncommitted edit in ../source is not picked up (HEAD export)
        write(os.path.join(self.src, 'domain_os', 'aa', 'aa.pas'), 'module aa; PANIC\n')
        rc, out = self.run_tool()
        self.assertEqual(rc, 0, out)
        self.assertIn('0 regression(s)', out)

        # committed: a regression, exit 1
        git(self.src, 'commit', '-q', '-am', 'two')
        rc, out = self.run_tool()
        self.assertEqual(rc, 1, out)
        self.assertIn('WORSE aa/aa.pas: OK -> FAILED (dpc panic: boom)', out)
        self.assertIn('the baseline is at ../source', out)

        # a refused module that now crashes dpc is a regression too
        write(os.path.join(self.src, 'domain_os', 'aa', 'aa.pas'), 'module aa;\n')
        git(self.src, 'commit', '-q', '-am', 'three')
        self.assertEqual(self.run_tool('--update')[0], 0)
        write(os.path.join(self.src, 'domain_os', 'bb', 'bb.pas'), 'module bb; PANIC\n')
        git(self.src, 'commit', '-q', '-am', 'four')
        rc, out = self.run_tool()
        self.assertEqual(rc, 1, out)
        self.assertIn('WORSE bb/bb.pas: REFUSED (a thing is not supported yet) -> FAILED', out)

        # a module that compiled and is gone (renamed, removed) is one too
        git(self.src, 'rm', '-q', '-r', 'domain_os/cc')
        git(self.src, 'commit', '-q', '-m', 'five')
        rc, out = self.run_tool()
        self.assertEqual(rc, 1, out)
        self.assertIn('GONE  cc/cc.pas: OK -> none', out)
        git(self.src, 'revert', '--no-edit', 'HEAD')

        # the working tree, on request
        rc, out = self.run_tool('--working-tree', '--only', 'aa/aa.pas')
        self.assertEqual(rc, 0, out)
        self.assertIn('1 of 1 module files compile', out)

    def test_make(self):
        dom = os.path.join(self.src, 'domain_os')
        write(os.path.join(dom, 'Makefile'),
              'PAS_MODULES := all\n'
              'domain_os:\n\t@echo "dpc=$(DPC) mods=$(PAS_MODULES)"\n'
              'check:\n\t@test -z "$(FAILCHECK)" || { echo "check_rfc: FAIL x"; exit 1; }\n')
        git(self.src, 'add', '-A')
        git(self.src, 'commit', '-q', '-m', 'mk')
        if True:
            def run(*extra):
                out = io.StringIO()
                with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
                    rc = linktest.main(['run', '--source', self.src, '--dpc', self.dpc,
                                        '--work', self.work, '--baseline', self.base,
                                        '-j', '2'] + list(extra))
                with open(os.path.join(self.work, 'make.log')) as fh:
                    return rc, out.getvalue(), fh.read()
            # bb is refused: no make unless --link-anyway, which links the others
            os.makedirs(self.work, exist_ok=True)
            write(os.path.join(self.work, 'make.log'), '')
            rc, out, _log = run()
            self.assertEqual(rc, 0, out)
            self.assertIn('make domain_os check: not run, 1 module file(s)', out)
            rc, out, log = run('--link-anyway')
            self.assertEqual(rc, 0, out)
            self.assertIn('mods=aa/aa.pas cc/cc.pas', log)
            self.assertIn('-k', log.splitlines()[0])
            # every module compiles: the real make, and its failure is exit 1
            write(os.path.join(dom, 'bb', 'bb.pas'), 'module bb;\n')
            git(self.src, 'commit', '-q', '-am', 'fixed')
            rc, out, log = run()
            self.assertEqual(rc, 0, out)
            self.assertIn('make domain_os check: OK', out)
            self.assertIn('dpc=%s mods=all' % self.dpc, log)
            rc, out, log = run('--make-var', 'FAILCHECK=1')
            self.assertEqual(rc, 1, out)
            self.assertIn('check_rfc: FAIL x', out)

    def test_update_refused(self):
        self.assertEqual(self.run_tool('--update', '--working-tree')[0], 2)
        self.assertEqual(self.run_tool('--update', '--only', 'aa/aa.pas')[0], 2)
        linktest.AS = 'no-such-assembler-here'
        self.assertEqual(self.run_tool('--update')[0], 2)
        self.assertFalse(os.path.exists(self.base))

    def test_bad_baseline(self):
        write(self.base, 'OK aa/aa.pas\n')
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            rc = linktest.main(['run', '--source', self.src, '--dpc', self.dpc,
                                '--work', self.work, '--baseline', self.base, '--no-make'])
        self.assertEqual(rc, 2)
        self.assertIn('baseline.txt:1: malformed line', err.getvalue())

    def test_unknown_only(self):
        rc, _out = self.run_tool('--only', 'zz/zz.pas')
        self.assertEqual(rc, 2)

    def test_source_left_alone(self):
        self.run_tool()
        p = subprocess.run(['git', '-C', self.src, 'status', '--porcelain'],
                           capture_output=True, text=True)
        self.assertEqual(p.stdout, '')


# A fake dpc with phases, like the real one: `SEMA:word' in a file of the
# unit (the module file or a file it includes) is a sema refusal, reported
# by --check-only and by a full compile; `ISEL:word' is a codegen refusal,
# reported by a full compile only and only when there is no sema error
# (dpc stops at the first phase that fails); `PANIC' panics in codegen,
# `SEGV' dies of SIGSEGV there and `HANG' sleeps for a minute;
# `CALLS:g' is dpc's sema error for a call of an internal routine with no
# body when g has an `extern' heading (stubbed, not named with
# --module-routine).
FAKE_PHASED_DPC = r"""#!PYTHON
import os, re, sys
args = sys.argv[1:]
src, check = args[-1], '--check-only' in args
out = args[args.index('-o') + 1] if '-o' in args else None
files = [src] + [f for f in re.findall(r"(?m)^\s*%include\s+'([^']+)'", open(src).read())
                 if os.path.exists(f)]
def scan(phase):
    return ['%s:%d:3: error: %s %s is not supported yet' % (f, n, phase.lower(), m.group(1))
            for f in files for n, ln in enumerate(open(f), 1)
            for m in re.finditer(phase + r':(\w+)', ln)]
ext = {n for f in files for n in re.findall(r'procedure (\w+);[^\n]*\bextern;', open(f).read())}
calls = ['%s:%d:3: error: call of internal routine `%s`, which has no body in this unit '
         '(ref 5.2 p. 5-4; name it with --module-routine)' % (f, n, m.group(1).upper())
         for f in files for n, ln in enumerate(open(f), 1)
         for m in re.finditer(r'CALLS:(\w+)', ln) if m.group(1) in ext]
errs = (scan('SEMA') + calls) or ([] if check else scan('ISEL'))
if errs:
    sys.stderr.write('\n'.join(errs) + '\n')
    sys.exit(1)
if not check and any('PANIC' in open(f).read() for f in files):
    sys.stderr.write("thread 'main' panicked at x.rs:1:1:\nboom\n")
    sys.exit(101)
if not check and any('SEGV' in open(f).read() for f in files):
    import signal
    os.kill(os.getpid(), signal.SIGSEGV)
if not check and any('HANG' in open(f).read() for f in files):
    import time
    time.sleep(60)
if out:
    open(out, 'w').write('\t.text\n')
""".replace('PYTHON', sys.executable)

# module -> {file: text}: x hides a codegen refusal behind a sema one; y
# has the sema one alone, z the codegen one alone (in two routines); w
# stops at a declaration; v panics in codegen; u compiles
PHASED = {
    'x/x.pas': {'x/x.pas': "module x;\n%include 'x/f.pas';\n",
                'x/f.pas': 'procedure p;\nbegin\n  SEMA:alpha;\nend;\n'
                           'procedure q;\nbegin\n  ISEL:beta;\nend;\n'},
    'y/y.pas': {'y/y.pas': "module y;\n%include 'y/f.pas';\n",
                'y/f.pas': 'procedure r;\nbegin\n  SEMA:alpha;\nend;\n'},
    'z/z.pas': {'z/z.pas': "module z;\n%include 'z/f.pas';\n",
                'z/f.pas': 'procedure s;\nbegin\n  ISEL:beta;\nend;\n'
                           'procedure t;\nbegin\n  ISEL:beta;\nend;\n'},
    'w/w.pas': {'w/w.pas': "module w;\nconst c = SEMA:decl;\n%include 'w/f.pas';\n",
                'w/f.pas': 'procedure o;\nbegin\n  ISEL:gamma;\nend;\n'},
    'v/v.pas': {'v/v.pas': "module v;\n%include 'v/f.pas';\n",
                'v/f.pas': 'procedure n;\nbegin\n  PANIC;\nend;\n'},
    'u/u.pas': {'u/u.pas': "module u;\n%include 'u/f.pas';\n",
                'u/f.pas': 'procedure m;\nbegin\nend;\n'},
}

SEMA_ALPHA = 'sema alpha is ' + REFUSAL
ISEL_BETA = 'isel beta is ' + REFUSAL


def phased_tree(root):
    for files in PHASED.values():
        for f, text in files.items():
            write(os.path.join(root, f), text)


def snapshot(root):
    out = {}
    for d, _dirs, files in os.walk(root):
        for f in files:
            with open(os.path.join(d, f)) as fh:
                out[os.path.relpath(os.path.join(d, f), root)] = fh.read()
    return out


class DeepPassTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.tree = os.path.join(self.tmp, 'domain_os')
        phased_tree(self.tree)
        write(os.path.join(self.tree, 'build', 'junk.s'), 'x\n')
        self.dpc = os.path.join(self.tmp, 'dpc')
        write(self.dpc, FAKE_PHASED_DPC, stat.S_IRWXU)
        self.work = os.path.join(self.tmp, 'work')

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_overlay(self):
        dst = os.path.join(self.tmp, 'ov')
        linktest.overlay_tree(self.tree, dst, {'x/x.pas', 'x/f.pas', 'x/gone.pas'})
        self.assertFalse(os.path.islink(os.path.join(dst, 'x')))
        self.assertFalse(os.path.islink(os.path.join(dst, 'x', 'f.pas')))
        self.assertTrue(os.path.islink(os.path.join(dst, 'y')))
        self.assertFalse(os.path.lexists(os.path.join(dst, 'build')))
        self.assertFalse(os.path.lexists(os.path.join(dst, 'x', 'gone.pas')))
        with open(os.path.join(dst, 'y', 'f.pas')) as fh:
            self.assertIn('SEMA:alpha', fh.read())

    def test_module_compile_sees_the_first_phase_only(self):
        r, _err = linktest.compile_module(self.dpc, self.tree, 'x/x.pas', assemble=False)
        self.assertEqual((r.status, r.kinds), ('REFUSED', [SEMA_ALPHA]))

    def test_deep_pass_sees_every_phase(self):
        before = snapshot(self.tree)
        d = linktest.deep_pass(self.dpc, self.tree, 'x/x.pas', self.work)
        self.assertEqual(d.kinds, {SEMA_ALPHA: ['p'], ISEL_BETA: ['q']})
        self.assertEqual((d.stubbed, d.problem), (['p', 'q'], None))
        self.assertEqual(snapshot(self.tree), before)      # the tree is only read
        self.assertTrue(os.path.exists(linktest.deep_dir(self.work, 'x/x.pas') + '.log'))
        d = linktest.deep_pass(self.dpc, self.tree, 'z/z.pas', self.work)
        self.assertEqual((d.kinds, d.stubbed), ({ISEL_BETA: ['s', 't']}, ['s', 't']))

    def test_deep_pass_stops_short(self):
        d = linktest.deep_pass(self.dpc, self.tree, 'w/w.pas', self.work)
        self.assertEqual(d.kinds, {'sema decl is ' + REFUSAL: ['-']})
        self.assertEqual(d.problem, 'check stage, stops at an error outside any routine')
        d = linktest.deep_pass(self.dpc, self.tree, 'v/v.pas', self.work)
        self.assertEqual(d.kinds, {'dpc panic: boom': ['-']})
        self.assertEqual(d.problem, 'codegen stage, dpc failed without an error')

    def one_routine_module(self, name, body):
        write(os.path.join(self.tree, name, name + '.pas'),
              "module %s;\n%%include '%s/f.pas';\n" % (name, name))
        write(os.path.join(self.tree, name, 'f.pas'), 'procedure k;\nbegin\n  %s;\nend;\n' % body)
        return '%s/%s.pas' % (name, name)

    def test_deep_pass_signal_and_timeout(self):
        d = linktest.deep_pass(self.dpc, self.tree, self.one_routine_module('sg', 'SEGV'),
                               self.work)
        self.assertEqual(d.kinds, {'dpc: signal 11': ['-']})
        self.assertEqual(d.problem, 'codegen stage, dpc died of signal 11')
        old = linktest.DPC_TIMEOUT
        linktest.DPC_TIMEOUT = 3
        try:
            d = linktest.deep_pass(self.dpc, self.tree, self.one_routine_module('hg', 'HANG'),
                                   self.work)
        finally:
            linktest.DPC_TIMEOUT = old
        self.assertEqual(d.kinds, {'dpc: timeout': ['-']})
        self.assertEqual(d.problem, 'codegen stage, dpc timed out (3 s)')

    def test_deep_pass_exception_is_recorded(self):
        with unittest.mock.patch.object(linktest.srcsurvey, 'Module',
                                        side_effect=RuntimeError('a bug')):
            d = linktest.deep_pass(self.dpc, self.tree, 'x/x.pas', self.work)
        self.assertEqual(d.problem, 'the deep pass failed: RuntimeError: a bug')
        self.assertFalse(d.complete)

    def test_stop_reasons(self):
        def mod(problem=None, rc=1):
            m = type('Mod', (), {})()
            m.problem, m.last_rc = problem, rc
            return m
        loc = lambda path, line: 'p' if path == 'in.pas' else None   # noqa: E731
        f = linktest.stop_reason
        self.assertEqual(f('check', mod('after 50 dpc runs of stubbing (gave up)'), '', loc),
                         ('check stage, after 50 dpc runs of stubbing (gave up)', None))
        self.assertEqual(f('codegen', mod(rc=101), "thread 'main' panicked at a:1:1:\nboom\n",
                           loc),
                         ('codegen stage, dpc failed without an error', 'dpc panic: boom'))
        self.assertEqual(f('codegen', mod(rc=-6), '', loc),
                         ('codegen stage, dpc died of signal 6', 'dpc: signal 6'))
        self.assertEqual(f('codegen', mod(rc=None), '', loc)[1], 'dpc: timeout')
        self.assertEqual(f('check', mod(), diag('out.pas', 2, 'x'), loc),
                         ('check stage, stops at an error outside any routine', None))
        self.assertEqual(f('check', mod(), diag('in.pas', 2, 'x'), loc)[0],
                         'check stage, nothing new to stub (the errors fall in routines '
                         'stubbed already)')

    def test_call_of_a_stubbed_routine_is_no_kind(self):
        # g has no header address, so srcsurvey cannot name it with
        # --module-routine once stubbed: h is charged with calling it
        write(os.path.join(self.tree, 't', 't.pas'), "module t;\n%include 't/f.pas';\n")
        write(os.path.join(self.tree, 't', 'f.pas'),
              'procedure g; internal;\nbegin\n  SEMA:alpha;\nend;\n'
              'procedure h;\nbegin\n  CALLS:g;\nend;\n')
        d = linktest.deep_pass(self.dpc, self.tree, 't/t.pas', self.work)
        self.assertEqual(d.kinds, {SEMA_ALPHA: ['g']})
        self.assertEqual((d.stubbed, d.unseen, d.problem), (['g', 'h'], [('h', 'G')], None))
        self.assertFalse(d.complete)
        r = linktest.Result('t/t.pas', 'REFUSED', 1, [SEMA_ALPHA])
        r.deep = d
        self.assertEqual(linktest.rank_kinds([r]), [(SEMA_ALPHA, 0, 1)])   # not `only'
        self.assertIn('INCOMPLETE: 1 routine(s) stubbed for a call of a stubbed internal '
                      'routine', '\n'.join(linktest.module_lines([r])))

    def test_rankings_and_closest(self):
        mods = sorted(PHASED)
        results = linktest.compile_all(self.dpc, self.tree, mods, 2, assemble=False)
        # dpc's own runs: alpha alone blocks x and y, beta z
        self.assertEqual(linktest.rank_kinds(results, False),
                         [(SEMA_ALPHA, 2, 2), ('dpc panic: boom', 1, 1), (ISEL_BETA, 1, 1),
                          ('sema decl is ' + REFUSAL, 1, 1)])
        linktest.deep_all(self.dpc, self.tree, results, self.work, 2)
        by = {r.module: r for r in results}
        self.assertEqual(by['x/x.pas'].all_kinds, [SEMA_ALPHA, ISEL_BETA])
        self.assertEqual(by['x/x.pas'].kinds, [SEMA_ALPHA])     # the status's kinds stay
        self.assertEqual(by['x/x.pas'].status, 'REFUSED')
        self.assertIsNone(by['u/u.pas'].deep)
        self.assertFalse(by['w/w.pas'].complete)
        # over every phase: x is blocked by both; w stopped short, no `only'
        rank = linktest.rank_kinds(results)
        self.assertEqual(rank, [(ISEL_BETA, 1, 2), (SEMA_ALPHA, 1, 2), ('dpc panic: boom', 0, 1),
                                ('sema decl is ' + REFUSAL, 0, 1)])
        self.assertEqual([r.module for r in linktest.closest(results)],
                         ['y/y.pas', 'z/z.pas', 'v/v.pas', 'w/w.pas', 'x/x.pas'])
        lines = linktest.format_closest(results, 2)
        self.assertIn('  %5d  %8s  y/y.pas: %s' % (1, 1, SEMA_ALPHA), lines)
        self.assertIn('3 more module(s)', lines[-1])
        self.assertIn('w/w.pas (incomplete)', '\n'.join(linktest.format_closest(results)))
        text = '\n'.join(linktest.module_lines(results))
        self.assertIn('all phases: 2 kind(s), 2 routine(s) stubbed\n'
                      '           * %s  [p]\n           * %s  [q]' % (SEMA_ALPHA, ISEL_BETA),
                      text)
        self.assertIn('INCOMPLETE: check stage, stops at an error outside any routine', text)


@unittest.skipUnless(shutil.which('git') and shutil.which('rsync'), 'needs git and rsync')
class DeepEndToEndTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.src = os.path.join(self.tmp, 'source')
        dom = os.path.join(self.src, 'domain_os')
        write(os.path.join(dom, 'tools', 'pascal_modules.py'), FAKE_PASCAL_MODULES)
        phased_tree(dom)
        git(self.src, 'init', '-q')
        git(self.src, 'add', '-A')
        git(self.src, 'commit', '-q', '-m', 'one')
        self.dpc = os.path.join(self.tmp, 'bin', 'dpc')
        write(self.dpc, FAKE_PHASED_DPC, stat.S_IRWXU)
        self.work = os.path.join(self.tmp, 'work')
        self.base = os.path.join(self.tmp, 'baseline.txt')
        self.old_as = linktest.AS
        linktest.AS = 'true'

    def tearDown(self):
        linktest.AS = self.old_as
        shutil.rmtree(self.tmp, ignore_errors=True)

    def run_tool(self, *extra):
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            rc = linktest.main(['run', '--source', self.src, '--dpc', self.dpc,
                                '--work', self.work, '--baseline', self.base, '--no-make',
                                '-j', '2'] + list(extra))
        with open(os.path.join(self.work, 'report.txt')) as fh:
            return rc, out.getvalue(), fh.read()

    def test_run(self):
        rc, out, rep = self.run_tool('--update')
        self.assertEqual(rc, 0, out)
        self.assertIn('1 of 6 module files compile; 4 refused by dpc, 1 dpc failure(s)', out)
        self.assertRegex(out, r'linktest: compile \d+\.\d s, deep pass \d+\.\d s \(-j 2\)')
        self.assertIn('error kinds over every phase', out)
        self.assertIn('     1      2  ' + ISEL_BETA, out)
        self.assertIn('closest to compiling', out)
        self.assertIn('      1         1  y/y.pas: ' + SEMA_ALPHA, out)
        self.assertIn('* %s  [q]' % ISEL_BETA, rep)
        self.assertIn('== error kinds of the module compiles alone', rep)
        # the baseline is the module compiles' (status, first kind), as before
        with open(self.base) as fh:
            self.assertIn('REFUSED\tx/x.pas\t%s\n' % SEMA_ALPHA, fh.read())
        # without the deep pass: dpc's own runs, as before
        rc, out, rep = self.run_tool('--no-deep')
        self.assertEqual(rc, 0, out)
        self.assertNotIn('deep pass', out)
        self.assertIn('first failing phase only', out)
        self.assertIn('     2      2  ' + SEMA_ALPHA, out)
        self.assertNotIn('all phases', rep)


@unittest.skipUnless(shutil.which('git'), 'needs git')
class WorktreeDefaultsTest(unittest.TestCase):
    def test_main_checkout_of_a_worktree(self):
        tmp = tempfile.mkdtemp()
        try:
            main = os.path.join(tmp, 'domain-pascal')
            write(os.path.join(main, 'f'), 'x\n')
            git(main, 'init', '-q')
            git(main, 'add', '-A')
            git(main, 'commit', '-q', '-m', 'one')
            wt = os.path.join(main, '.claude', 'worktrees', 'agent-x')
            git(main, 'worktree', 'add', '-q', wt)
            real = os.path.realpath(main)
            self.assertEqual(os.path.realpath(linktest.main_checkout(wt)), real)
            self.assertEqual(os.path.realpath(linktest.main_checkout(main)), real)
            # not a repository at all: itself
            self.assertEqual(linktest.main_checkout(tmp), tmp)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def test_defaults_follow_main_checkout(self):
        main = linktest.main_checkout()
        self.assertEqual(linktest.default_dpc(),
                         os.path.join(main, 'target', 'stable', 'bin', 'dpc'))
        self.assertEqual(linktest.default_source(),
                         os.path.abspath(os.path.join(main, '..', 'source')))


class ModsurveyCompatTest(unittest.TestCase):
    def test_parse_nm(self):
        d = linktest.parse_nm('a.o:00000000 T FOO\na.o:00000004 t bar\nb.o:         U FOO\n')
        self.assertEqual(d, {'a.o': {'FOO': 'T'}, 'b.o': {}})


if __name__ == '__main__':
    unittest.main()
