#!/usr/bin/env python3
"""Execution tests: run dpc output on the Musashi 68010 emulator.

    exectest.py [-v] [--trace] [--build-only] [tests/exec/NAME.pas ...]

Each test in tests/exec/ is a Pascal module whose first comment holds
directives (tests/exec/README.md):

    call: NAME [ARG]... [-> KIND]    a call of an entry routine (any number)
    link: FILE.mod.pas...            more modules (in tests/exec/) linked in
    dpc: OPTION...                   extra dpc options (all modules)
    xfail: REASON                    a known miscompile: must fail
    expect:                          the rest of the comment, one line each,
                                     is the expected output

The module is compiled with target/debug/dpc (-I tests/exec for
exec.ins.pas), assembled with m68k-elf-as -m68010, linked with
tools/exectest/stubs.s and dpc's M$ARITH runtime (the archive `dpc
--print-runtime' names, target/runtime/m68k/libm_arith.a, built by `make
build'; EXECTEST_RUNTIME overrides the path) by
tools/exectest/link.ld at 0x10000, flattened with objcopy, and run with
tools/exectest/runner, which this script builds against Musashi into
target/exectest/.  Musashi is not in this repository (its licence allows
non-commercial use only); it is found at MUSASHI_DIR, by default
~/src/domainos-archeology/otterlater/musashi.  Without it, or without the
m68k-elf tools or the runtime, the tests are skipped with a notice (exit 0).

Work files go to tmp/exectest/<name>/ (the .s is kept for reports).
"""

import argparse
import difflib
import functools
import glob
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TESTDIR = os.path.join(ROOT, 'tests', 'exec')
TOOLDIR = os.path.join(ROOT, 'tools', 'exectest')
BUILD = os.path.join(ROOT, 'target', 'exectest')
WORK = os.path.join(ROOT, 'tmp', 'exectest')
DPC = os.path.join(ROOT, 'target', 'debug', 'dpc')
# dpc's run-time library (M$ARITH; runtime/README.md): by default the
# path `dpc --print-runtime=m68k' prints
RUNTIME = os.environ.get('EXECTEST_RUNTIME')
MUSASHI = os.environ.get('MUSASHI_DIR', os.path.expanduser(
    '~/src/domainos-archeology/otterlater/musashi'))
MUSASHI_SRCS = ['m68kcpu', 'm68kops', 'm68kopac', 'm68kopdm', 'm68kopnz',
                'm68kdasm']
CC = os.environ.get('CC', 'cc')
AS = 'm68k-elf-as'
LD = 'm68k-elf-ld'
OBJCOPY = 'm68k-elf-objcopy'
NM = 'm68k-elf-nm'
BASE = 0x10000


class TestSpec:
    def __init__(self, path):
        self.path = path
        self.name = os.path.splitext(os.path.basename(path))[0]
        self.calls = []        # [(symbol, [args], kind or None)]
        self.dpc_opts = []
        self.links = []
        self.xfail = None
        self.expect = []


class SpecError(Exception):
    pass


def first_comment(text):
    """The text of the file's first `{ }' or `(* *)' comment."""
    m = re.search(r'\{(.*?)\}|\(\*(.*?)\*\)', text, re.S)
    if not m:
        return None
    return m.group(1) if m.group(1) is not None else m.group(2)


def parse_spec(path, text=None):
    if text is None:
        with open(path) as f:
            text = f.read()
    spec = TestSpec(path)
    body = first_comment(text)
    if body is None:
        raise SpecError('%s: no header comment' % path)
    lines = body.split('\n')
    i = 0
    seen_expect = False
    while i < len(lines):
        s = lines[i].strip()
        i += 1
        m = re.match(r'(call|link|dpc|xfail|expect):\s*(.*)$', s)
        if not m:
            continue
        key, rest = m.group(1), m.group(2)
        if key == 'call':
            toks = rest.split()
            if not toks:
                raise SpecError('%s: empty call:' % path)
            kind = None
            if '->' in toks:
                k = toks.index('->')
                if k != len(toks) - 2:
                    raise SpecError('%s: `-> KIND\' must end a call:' % path)
                kind = toks[-1]
                if kind not in ('l', 'w', 'uw', 'b', 'c', 'a0'):
                    raise SpecError('%s: bad result kind %s' % (path, kind))
                toks = toks[:k]
            for a in toks[1:]:
                if not re.match(r'@?[lwb]:-?(0x[0-9a-fA-F]+|\d+)(!)?$', a) or \
                        (a.endswith('!') and not a.startswith('@')):
                    raise SpecError('%s: bad argument %s' % (path, a))
            spec.calls.append((toks[0].upper(), toks[1:], kind))
        elif key == 'link':
            for f in rest.split():
                if not f.endswith('.mod.pas'):
                    raise SpecError('%s: link: takes .mod.pas files' % path)
                spec.links.append(os.path.join(os.path.dirname(path), f))
        elif key == 'dpc':
            spec.dpc_opts += rest.split()
        elif key == 'xfail':
            spec.xfail = rest or 'xfail'
        elif key == 'expect':
            if rest:
                raise SpecError('%s: expected lines go after expect:' % path)
            spec.expect = [l.strip() for l in lines[i:] if l.strip()]
            seen_expect = True
            break
    if not spec.calls:
        raise SpecError('%s: no call:' % path)
    if not seen_expect:
        raise SpecError('%s: no expect:' % path)
    return spec


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def missing_prereqs():
    """Why the tests cannot run, or None."""
    for t in (AS, LD, OBJCOPY, NM, CC):
        if shutil.which(t) is None:
            return '%s not found' % t
    if not os.path.isfile(os.path.join(MUSASHI, 'm68kcpu.c')):
        return 'Musashi not found at %s (set MUSASHI_DIR)' % MUSASHI
    if runtime_path() is None:
        return ('the M$ARITH runtime is not built (`make runtime\'; or set '
                'EXECTEST_RUNTIME)')
    return None


@functools.lru_cache(maxsize=None)
def runtime_path():
    """The runtime library to link: EXECTEST_RUNTIME, else what `dpc
    --print-runtime=m68k' prints; None when there is none."""
    if RUNTIME:
        return RUNTIME if os.path.isfile(RUNTIME) else None
    if not os.path.isfile(DPC):
        return None
    p = run([DPC, '--print-runtime=m68k'])
    path = p.stdout.strip()
    return path if p.returncode == 0 and os.path.isfile(path) else None


def newer(target, sources):
    if not os.path.exists(target):
        return True
    t = os.path.getmtime(target)
    return any(os.path.getmtime(s) > t for s in sources)


def build_runner():
    """Build target/exectest/runner and the stub object."""
    os.makedirs(BUILD, exist_ok=True)
    objs = []
    for n in MUSASHI_SRCS:
        src = os.path.join(MUSASHI, n + '.c')
        obj = os.path.join(BUILD, n + '.o')
        if newer(obj, [src]):
            p = run([CC, '-O2', '-w', '-I', MUSASHI, '-c', src, '-o', obj])
            if p.returncode:
                raise SpecError('building Musashi %s:\n%s' % (n, p.stderr))
        objs.append(obj)
    src = os.path.join(TOOLDIR, 'runner.c')
    obj = os.path.join(BUILD, 'runner.o')
    if newer(obj, [src]):
        p = run([CC, '-O2', '-Wall', '-I', MUSASHI, '-c', src, '-o', obj])
        if p.returncode:
            raise SpecError('building runner.c:\n%s' % p.stderr)
    objs.append(obj)
    exe = os.path.join(BUILD, 'runner')
    if newer(exe, objs):
        p = run([CC, '-o', exe] + objs)
        if p.returncode:
            raise SpecError('linking runner:\n%s' % p.stderr)
    src = os.path.join(TOOLDIR, 'stubs.s')
    obj = os.path.join(BUILD, 'stubs.o')
    if newer(obj, [src]):
        p = run([AS, '-m68010', src, '-o', obj])
        if p.returncode:
            raise SpecError('assembling %s:\n%s' % (src, p.stderr))
    return exe


def symbols(elf):
    p = run([NM, elf])
    if p.returncode:
        raise SpecError(p.stderr)
    syms = {}
    for line in p.stdout.splitlines():
        parts = line.split()
        if len(parts) == 3:
            syms.setdefault(parts[2], int(parts[0], 16))
    return syms


class Result:
    def __init__(self, ok, output, detail='', ran=False):
        self.ok = ok
        self.output = output
        self.detail = detail
        self.ran = ran          # built and run: an xfail must get this far


def run_test(spec, runner, trace=False):
    work = os.path.join(WORK, spec.name)
    os.makedirs(work, exist_ok=True)
    elf = os.path.join(work, spec.name + '.elf')
    binf = os.path.join(work, spec.name + '.bin')
    objs = []
    for src in [spec.path] + spec.links:
        stem = os.path.basename(src)[:-len('.pas')]
        asm = os.path.join(work, stem + '.s')
        obj = os.path.join(work, stem + '.o')
        p = run([DPC, '-I', TESTDIR] + spec.dpc_opts + ['-o', asm, src])
        if p.returncode:
            return Result(False, [], 'dpc failed:\n' + p.stderr)
        p = run([AS, '-m68010', asm, '-o', obj])
        if p.returncode:
            return Result(False, [], 'as failed:\n' + p.stderr)
        objs.append(obj)
    p = run([LD, '-T', os.path.join(TOOLDIR, 'link.ld'), '-o', elf] + objs +
            [os.path.join(BUILD, 'stubs.o'), runtime_path()])
    if p.returncode:
        return Result(False, [], 'ld failed:\n' + p.stderr)
    p = run([OBJCOPY, '-O', 'binary', elf, binf])
    if p.returncode:
        return Result(False, [], 'objcopy failed:\n' + p.stderr)
    syms = symbols(elf)
    if syms.get('_end') is None:
        return Result(False, [], 'no _end symbol')
    if syms.get('_etext') is None:
        return Result(False, [], 'no _etext symbol')
    cmd = [runner, '--bin', binf, '--base', hex(BASE), '--end', hex(syms['_end']),
           '--text-end', hex(syms['_etext'])]
    if trace:
        cmd.append('--trace')
    for name, addr in sorted(syms.items()):
        if name.startswith('EXEC_$'):
            cmd += ['--stub', '%s=%s' % (name, hex(addr))]
    for name, args, kind in spec.calls:
        if name not in syms:
            return Result(False, [], 'no symbol %s in the linked test' % name)
        c = [hex(syms[name])] + args
        if kind:
            c += ['->', kind]
        cmd += ['--call', ' '.join(c)]
    p = run(cmd, timeout=60)
    out = [l.strip() for l in p.stdout.splitlines() if l.strip()]
    detail = p.stderr if trace else ''
    if p.returncode not in (0, 1):
        return Result(False, out, 'runner failed (%d):\n%s' % (p.returncode, p.stderr))
    return Result(out == spec.expect and p.returncode == 0, out, detail, ran=True)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('tests', nargs='*')
    ap.add_argument('-v', '--verbose', action='store_true')
    ap.add_argument('--trace', action='store_true',
                    help='instruction trace on stderr (with one test)')
    ap.add_argument('--build-only', action='store_true')
    a = ap.parse_args(argv)

    why = missing_prereqs()
    if why:
        print('exectest: skipped: %s' % why)
        return 0
    if not os.path.isfile(DPC):
        print('exectest: %s not built (run cargo build)' % DPC)
        return 1
    try:
        runner = build_runner()
    except SpecError as e:
        print('exectest: %s' % e)
        return 1
    if a.build_only:
        return 0

    paths = a.tests or sorted(glob.glob(os.path.join(TESTDIR, '*.pas')))
    paths = [p for p in paths
             if not p.endswith('.ins.pas') and not p.endswith('.mod.pas')]
    npass = nfail = nxfail = 0
    for path in paths:
        try:
            spec = parse_spec(path)
        except SpecError as e:
            print('ERROR %s' % e)
            nfail += 1
            continue
        res = run_test(spec, runner, a.trace)
        if a.trace and res.detail:
            sys.stderr.write(res.detail)
        rel = os.path.relpath(path, ROOT)
        if spec.xfail:
            if res.ok:
                print('XPASS %s (remove its xfail: %s)' % (rel, spec.xfail))
                nfail += 1
            elif not res.ran:
                # an xfail documents a miscompile: dpc refusing the test,
                # or a tool failing, is not the expected failure
                print('FAIL  %s (xfail, but not run)' % rel)
                report(spec, res)
                nfail += 1
            else:
                print('xfail %s: %s' % (rel, spec.xfail))
                nxfail += 1
                if a.verbose:
                    report(spec, res)
            continue
        if res.ok:
            npass += 1
            if a.verbose:
                print('ok    %s' % rel)
        else:
            nfail += 1
            print('FAIL  %s' % rel)
            report(spec, res)
    print('exectest: %d passed, %d failed, %d expected failures'
          % (npass, nfail, nxfail))
    return 1 if nfail else 0


def report(spec, res):
    if res.detail and not res.output:
        print(res.detail.rstrip())
        return
    for l in difflib.unified_diff(spec.expect, res.output, 'expected', 'actual',
                                  lineterm='', n=2):
        print('      ' + l)
    print('      asm: %s' % os.path.relpath(
        os.path.join(WORK, spec.name, spec.name + '.s'), ROOT))


if __name__ == '__main__':
    sys.exit(main())
