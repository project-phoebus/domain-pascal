#!/usr/bin/env python3
"""Snapshots of dpc's assembly for the conformance units and exec programs.

Image conformance (`make conform') needs the SR10.2 image and link map,
and the execution tests (`make exectest') need Musashi; none of them is
in this repository, so a machine without them (CI) skips both.  The
snapshots keep their compiler side checkable there (bead
domain-pascal-rz9q): tests/snapshots/<suite>/NAME.s is dpc's output for
one unit, normalised as the golden tests normalise it (`|' comments
dropped, blanks collapsed, empty lines dropped), and NAME.args its
command line, `SRC OPTION...', with SRC relative to the repository root
and the map-derived options (fragment origins, module routines) already
resolved.

    tools/snapshot.py check [--dpc DPC]
        compiles every snapshot's SRC with its options and compares the
        output with NAME.s; also fails when a conformance .pas unit of
        tests/conformance/manifest.txt, an exec program or one of its
        link: modules has no snapshot, or a snapshot belongs to none.
        Needs nothing but dpc.  Exit 1 on a difference.

    tools/snapshot.py update [--suite conformance|exec] [--dpc DPC]
        rewrites the suite's snapshots (both by default) from dpc's
        output, and only when the suite passes: every .pas unit of the
        manifest compiled and matching the image (as `make conform'
        checks it), or `make exectest' passing.  Needs what those need;
        exit 2 when it is missing.  A snapshot change needs a reason in
        the commit message, as a golden change does.

Suites:
    conformance  NAME = the unit's file name without .pas; options
                 `--strict' and those `conform.py manifest' gives dpc.
    exec         NAME = the program's file name without .pas, and
                 PROGRAM+MODULE (MODULE's file name without .pas) for each
                 `link:' module, compiled with the program's options; the
                 options `-I tests/exec' and the program's `dpc:' lines.

Python 3 standard library only.  Scratch output goes to tmp/snapshot/.
"""

import argparse
import contextlib
import difflib
import glob
import io
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import conform  # noqa: E402
import exectest  # noqa: E402

SNAPSHOTS = os.path.join(REPO, 'tests', 'snapshots')
MANIFEST = os.path.join(REPO, 'tests', 'conformance', 'manifest.txt')
SCRATCH = os.path.join(REPO, 'tmp', 'snapshot')
DPC = os.path.join(REPO, 'target', 'debug', 'dpc')
SUITES = ('conformance', 'exec')


class Error(Exception):
    """A setup problem (exit 2)."""


def normalise(asm):
    """crates/dpc/tests/golden.rs `normalise': `|' comments removed, each
    line's words joined by one space, empty lines dropped."""
    out = []
    for line in asm.splitlines():
        words = line.split('|', 1)[0].split()
        if words:
            out.append(' '.join(words) + '\n')
    return ''.join(out)


def rel(path):
    return os.path.relpath(os.path.abspath(path), REPO)


# ---- what the suites hold (no map, image or emulator needed) -------------

def conformance_units():
    """{NAME: SRC} for the manifest's .pas units."""
    out = {}
    for lineno, toks in conform.manifest_lines(MANIFEST):
        if toks[0].endswith('.pas'):
            name = os.path.basename(toks[0])[:-len('.pas')]
            if name in out:
                raise Error('%s:%d: a second unit named %s' % (rel(MANIFEST), lineno, name))
            out[name] = toks[0]
    return out


def exec_programs():
    """[(NAME, SRC, options)]: each exec program and each of its link:
    modules (exectest.main's list, exectest.run_test's command)."""
    out = []
    for path in sorted(glob.glob(os.path.join(exectest.TESTDIR, '*.pas'))):
        if path.endswith(('.ins.pas', '.mod.pas')):
            continue
        try:
            spec = exectest.parse_spec(path)
        except exectest.SpecError as e:
            raise Error(str(e))
        opts = ['-I', rel(exectest.TESTDIR)] + spec.dpc_opts
        stem = os.path.basename(path)[:-len('.pas')]
        out.append((stem, rel(path), opts))
        for m in spec.links:
            out.append(('%s+%s' % (stem, os.path.basename(m)[:-len('.pas')]), rel(m), opts))
    return out


def expected_names():
    return {'conformance': set(conformance_units()),
            'exec': {n for n, _, _ in exec_programs()}}


# ---- compiling ----------------------------------------------------------

def compile_unit(dpc, suite, name, src, opts):
    """dpc's normalised output for SRC with OPTS, run from the repository
    root as the .args file records it; Error when dpc refuses it."""
    out = os.path.join(SCRATCH, suite, name + '.s')
    os.makedirs(os.path.dirname(out), exist_ok=True)
    p = subprocess.run([dpc] + opts + ['-o', out, src], cwd=REPO,
                       capture_output=True, text=True)
    if p.returncode:
        raise Error('%s/%s: dpc failed:\n%s' % (suite, name, p.stderr))
    with open(out) as f:
        return normalise(f.read()), out


def read_args(path):
    with open(path) as f:
        toks = f.read().split()
    if not toks:
        raise Error('%s: empty' % rel(path))
    return toks[0], toks[1:]


def write_snapshot(suite, name, src, opts, text):
    d = os.path.join(SNAPSHOTS, suite)
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, name + '.args'), 'w') as f:
        f.write(' '.join([src] + opts) + '\n')
    with open(os.path.join(d, name + '.s'), 'w') as f:
        f.write(text)


def stored_names(suite):
    d = os.path.join(SNAPSHOTS, suite)
    files = os.listdir(d) if os.path.isdir(d) else []
    return {f[:-len('.args')] for f in files if f.endswith('.args')} | \
        {f[:-len('.s')] for f in files if f.endswith('.s')}


# ---- check ---------------------------------------------------------------

def check(dpc, out=print):
    """Returns the number of failures."""
    if not os.path.isfile(dpc):
        raise Error('%s not built (run `make build`)' % dpc)
    want = expected_names()
    bad = n = 0
    for suite in SUITES:
        have = stored_names(suite)
        for name in sorted(want[suite] - have):
            out('MISSING %s/%s (run tools/snapshot.py update)' % (suite, name))
            bad += 1
        for name in sorted(have - want[suite]):
            out('STALE   %s/%s: no such unit (run tools/snapshot.py update)' % (suite, name))
            bad += 1
        for name in sorted(have & want[suite]):
            base = os.path.join(SNAPSHOTS, suite, name)
            n += 1
            try:
                src, opts = read_args(base + '.args')
                got, path = compile_unit(dpc, suite, name, src, opts)
                with open(base + '.s') as f:
                    stored = f.read()
            except (Error, OSError) as e:
                out('ERROR   %s' % e)
                bad += 1
                continue
            if stored != normalise(stored):
                out('FAIL    %s/%s: %s.s is not normalised (run tools/snapshot.py update)'
                    % (suite, name, rel(base)))
                bad += 1
            elif got != stored:
                out('FAIL    %s/%s (dpc output: %s)' % (suite, name, rel(path)))
                diff = list(difflib.unified_diff(stored.splitlines(), got.splitlines(),
                                                 'snapshot', 'dpc', lineterm='', n=2))
                for l in diff[:40]:
                    out('        ' + l)
                if len(diff) > 40:
                    out('        ... %d more lines' % (len(diff) - 40))
                bad += 1
    out('snapshot: %d checked, %d failed' % (n, bad))
    return bad


# ---- update --------------------------------------------------------------

def update_conformance(dpc, out=print):
    """Every .pas unit compiled, checked against the image, and only when
    all match, written."""
    miss = conform.missing_prereqs()
    if miss:
        raise Error('conformance: missing: %s' % ', '.join(miss))
    linkmap = conform.sau2map.load()
    done, failed = [], []
    for lineno, toks in conform.manifest_lines(MANIFEST):
        if not toks[0].endswith('.pas'):
            continue
        try:
            u = conform.manifest_unit(toks, linkmap)
        except conform.SetupError as e:
            raise Error('%s:%d: %s' % (rel(MANIFEST), lineno, e))
        name = os.path.basename(u.file)[:-len('.pas')]
        opts = ['--strict'] + u.dpc_opts
        text, asm = compile_unit(dpc, 'conformance', name, u.file, opts)
        log = []
        try:
            ok = conform.check(asm, u.specs, linkmap, u.sect, out=log.append,
                               work=conform.scratch_dir(u.src), name=u.file,
                               local_syms=u.local, enclosing=u.enclosing,
                               required_globals=u.globs)
        except conform.SetupError as e:
            raise Error('%s:%d: %s' % (rel(MANIFEST), lineno, e))
        if not ok:
            failed.append(u.file)
            for l in log:
                out(l)
        done.append((name, u.file, opts, text))
    if failed:
        raise Error('conformance: %d unit(s) do not match the image; snapshots not '
                    'updated: %s' % (len(failed), ', '.join(failed)))
    return done


def update_exec(dpc, out=print):
    """`make exectest' passing, then every program and link module
    compiled and written."""
    why = exectest.missing_prereqs()
    if why:
        raise Error('exec: %s' % why)
    log = io.StringIO()
    with contextlib.redirect_stdout(log):
        rc = exectest.main([])
    if rc:
        out(log.getvalue().rstrip())
        raise Error('exec: exectest failed; snapshots not updated')
    return [(name, src, opts, compile_unit(dpc, 'exec', name, src, opts)[0])
            for name, src, opts in exec_programs()]


def update(dpc, suites, out=print):
    if not os.path.isfile(dpc):
        raise Error('%s not built (run `make build`)' % dpc)
    for suite in suites:
        units = (update_conformance if suite == 'conformance' else update_exec)(dpc, out)
        d = os.path.join(SNAPSHOTS, suite)
        keep = {name for name, _, _, _ in units}
        for name in stored_names(suite) - keep:
            for ext in ('.args', '.s'):
                with contextlib.suppress(FileNotFoundError):
                    os.remove(os.path.join(d, name + ext))
        for name, src, opts, text in units:
            write_snapshot(suite, name, src, opts, text)
        out('snapshot: %s: %d written' % (suite, len(units)))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    c = sub.add_parser('check', help='compare dpc output with the snapshots')
    c.add_argument('--dpc', default=DPC)
    u = sub.add_parser('update', help='rewrite the snapshots of passing suites')
    u.add_argument('--suite', choices=SUITES, action='append')
    u.add_argument('--dpc', default=DPC)
    a = ap.parse_args(argv)
    try:
        if a.cmd == 'check':
            return 1 if check(a.dpc) else 0
        update(a.dpc, a.suite or SUITES)
        return 0
    except Error as e:
        print('snapshot: %s' % e, file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
