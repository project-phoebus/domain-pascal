#!/usr/bin/env python3
"""Execution tests on aarch64: dpc output run natively on this Mac.

    exectest_a64.py [-v] [tests/exec/NAME.pas ...]

The tests of tests/exec/ (tests/exec/README.md), compiled with
`dpc --target=aarch64-apple-darwin', assembled and linked with the host's
clang, and run as a native program (docs/aarch64.md, "Tests").  Each test's
`call:' directives become a generated C driver; the stubs of exec.ins.pas
are tools/exectest/a64_stubs.c; dpc's aarch64 run time is the archive
`dpc --print-runtime=aarch64-apple-darwin' names (built by `make build').

tests/exec/aarch64.txt lists the tests that do not apply on aarch64
(`skip REASON': m68k registers, supervisor instructions, a device
register), that are known to fail there (`xfail REASON'), or whose
output differs there by design (`expect REASON': little-endian byte
order, 8-byte pointers), with the aarch64 output in
tests/exec/aarch64/NAME.expect.

Needs an arm64 macOS host (`uname -m' arm64, `uname -s' Darwin); elsewhere
the tests are skipped with a notice.  Work files go to
tmp/exectest_a64/<name>/.
"""

import argparse
import os
import platform
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import exectest  # noqa: E402

ROOT = exectest.ROOT
TESTDIR = exectest.TESTDIR
WORK = os.path.join(ROOT, 'tmp', 'exectest_a64')
DPC = exectest.DPC
TRIPLE = 'aarch64-apple-darwin'
STUBS = os.path.join(exectest.TOOLDIR, 'a64_stubs.c')
LIST = os.path.join(TESTDIR, 'aarch64.txt')
CC = os.environ.get('CC', 'clang')
CFLAGS = ['-arch', 'arm64', '-Wno-dollar-in-identifier-extension']


def read_list():
    """{test name: (skip|xfail, reason)} from tests/exec/aarch64.txt."""
    out = {}
    if not os.path.exists(LIST):
        return out
    with open(LIST) as f:
        for n, line in enumerate(f, 1):
            line = line.split('#', 1)[0].strip()
            if not line:
                continue
            m = re.match(r'(\S+)\s+(skip|xfail|expect)\s+(.+)$', line)
            if not m:
                raise SystemExit('%s:%d: want NAME skip|xfail|expect REASON' % (LIST, n))
            out[m.group(1)] = (m.group(2), m.group(3))
    return out


def c_arg(tok, i, pre, post):
    """C for one `call:' argument: the expression passed, with set-up
    (`pre') and print-out (`post') lines for a by-address cell."""
    m = re.match(r'(@?)([lwb]):(-?(?:0x[0-9a-fA-F]+|\d+))(!?)$', tok)
    addr, size, val, out = m.groups()
    v = int(val, 0)
    if not addr:
        # By value: the low bits of an x register (the caller extends a
        # value of less than 32 bits; the callee reads its own width).
        return '(int64_t)%d' % v
    ctype = {'l': 'int32_t', 'w': 'int16_t', 'b': 'uint8_t'}[size]
    pre.append('%s cell%s = (%s)%d;' % (ctype, i, ctype, v))
    if out:
        fmt = '%u' if size == 'b' else '%d'
        post.append('printf("out %s\\n", (int)cell%s);' % (fmt, i))
    return '(int64_t)&cell%s' % i


def c_driver(spec):
    """The C driver that makes the test's calls in order."""
    lines = ['#include <stdint.h>', '#include <stdio.h>',
             'void dpc_print_bool(unsigned b);']
    decls = set()
    body = []
    for k, (name, args, kind) in enumerate(spec.calls):
        cname = 'f_%d' % k
        decls.add('extern int64_t %s(%s) __asm__("_%s");' % (
            cname, ', '.join(['int64_t'] * len(args)) or 'void', name))
        pre, post = [], []
        cargs = [c_arg(a, '%d_%d' % (k, i), pre, post) for i, a in enumerate(args)]
        body.append('{')
        body += ['  ' + p for p in pre]
        body.append('  int64_t r = %s(%s);' % (cname, ', '.join(cargs)))
        body.append('  (void)r;')
        body += ['  ' + p for p in post]
        if kind == 'l':
            body.append('  printf("=> %d\\n", (int32_t)r);')
        elif kind == 'w':
            body.append('  printf("=> %d\\n", (int16_t)r);')
        elif kind == 'uw':
            body.append('  printf("=> %u\\n", (unsigned)(r & 0xffff));')
        elif kind == 'c':
            body.append('  printf("=> %u\\n", (unsigned)(r & 0xff));')
        elif kind == 'b':
            body.append('  printf("=> "); dpc_print_bool((unsigned)(r & 0xff));')
        elif kind == 'a0':
            body.append('  printf("=> 0x%08llx\\n", (unsigned long long)r);')
        body.append('  fflush(stdout);')
        body.append('}')
    lines += sorted(decls)
    lines.append('int dpc_driver_main(void) {')
    lines += ['  ' + b for b in body]
    lines.append('  return 0;')
    lines.append('}')
    return '\n'.join(lines) + '\n'


def runtime_path():
    p = exectest.run([DPC, '--print-runtime=' + TRIPLE])
    return p.stdout.strip() if p.returncode == 0 else None


def run_test(spec, runtime):
    work = os.path.join(WORK, spec.name)
    os.makedirs(work, exist_ok=True)
    objs = []
    stderr = ''
    for src in [spec.path] + spec.links:
        stem = os.path.basename(src)[:-len('.pas')]
        asm = os.path.join(work, stem + '.s')
        obj = os.path.join(work, stem + '.o')
        p = exectest.run([DPC, '--target=' + TRIPLE, '-I', TESTDIR] + spec.dpc_opts +
                         ['-o', asm, src])
        if p.returncode:
            return exectest.Result(False, [], 'dpc failed:\n' + p.stderr)
        stderr += p.stderr
        p = exectest.run([CC] + CFLAGS + ['-c', '-o', obj, asm])
        if p.returncode:
            return exectest.Result(False, [], 'as failed:\n' + p.stderr)
        objs.append(obj)
    # `warns:' lines name the m68k backend's best-effort paths (bead
    # domain-pascal-5kgg), which say nothing about this target: not
    # checked here.
    drv = os.path.join(work, 'driver.c')
    with open(drv, 'w') as f:
        f.write(c_driver(spec))
    exe = os.path.join(work, spec.name)
    p = exectest.run([CC] + CFLAGS + ['-o', exe, '-Wl,-e,_dpc_driver_main', drv, STUBS] +
                     objs + [runtime])
    if p.returncode:
        return exectest.Result(False, [], 'link failed:\n' + p.stderr)
    try:
        p = exectest.run([exe], timeout=60)
    except subprocess.TimeoutExpired:
        return exectest.Result(False, [], 'timed out', ran=True)
    out = [l.strip() for l in p.stdout.splitlines() if l.strip()]
    if p.returncode != 0:
        out.append('!! exit status %d' % p.returncode)
    return exectest.Result(out == spec.expect and p.returncode == 0, out, p.stderr, ran=True)


def report(spec, res):
    if res.detail and not res.output:
        print('      ' + res.detail.rstrip().replace('\n', '\n      '))
        return
    for l in exectest.difflib.unified_diff(spec.expect, res.output, 'expected', 'actual',
                                           lineterm='', n=2):
        print('      ' + l)
    if res.detail:
        print('      ' + res.detail.rstrip().replace('\n', '\n      '))
    print('      asm: %s' % os.path.relpath(
        os.path.join(WORK, spec.name, spec.name + '.s'), ROOT))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('tests', nargs='*')
    ap.add_argument('-v', '--verbose', action='store_true')
    a = ap.parse_args(argv)
    if platform.system() != 'Darwin' or platform.machine() != 'arm64':
        print('exectest_a64: skipped: not an arm64 macOS host')
        return 0
    runtime = runtime_path()
    if not runtime:
        print('exectest_a64: skipped: the aarch64 run time is not built (make build)')
        return 0
    listed = read_list()
    paths = a.tests or sorted(
        os.path.join(TESTDIR, f) for f in os.listdir(TESTDIR)
        if f.endswith('.pas') and not f.endswith(('.mod.pas', '.ins.pas')))
    counts = {'pass': 0, 'fail': 0, 'xfail': 0, 'skip': 0}
    for path in paths:
        rel = os.path.relpath(path, ROOT)
        spec = exectest.parse_spec(path)
        how, why = listed.get(spec.name, (None, None))
        if how == 'expect':
            with open(os.path.join(TESTDIR, 'aarch64', spec.name + '.expect')) as f:
                spec.expect = [l.strip() for l in f if l.strip()]
        if how == 'skip' and not a.tests:
            counts['skip'] += 1
            if a.verbose:
                print('SKIP %s (%s)' % (rel, why))
            continue
        res = run_test(spec, runtime)
        expect_fail = how == 'xfail' or (spec.xfail is not None)
        if res.ok and not expect_fail:
            counts['pass'] += 1
            if a.verbose:
                print('ok   %s' % rel)
        elif not res.ok and expect_fail:
            counts['xfail'] += 1
            if a.verbose:
                print('xfail %s (%s)' % (rel, why or spec.xfail))
        elif res.ok and expect_fail:
            counts['fail'] += 1
            print('XPASS %s (remove its xfail: %s)' % (rel, why or spec.xfail))
        else:
            counts['fail'] += 1
            print('FAIL %s' % rel)
            report(spec, res)
    print('exectest_a64: %(pass)d passed, %(fail)d failed, %(xfail)d xfail, '
          '%(skip)d skipped' % counts)
    return 1 if counts['fail'] else 0


if __name__ == '__main__':
    sys.exit(main())
