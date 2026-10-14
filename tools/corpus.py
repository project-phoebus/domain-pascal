#!/usr/bin/env python3
"""Front-end corpus test: dpc --check-only over Apollo's own SR10.2 Pascal.

    corpus.py [-v] [--update] [--corpus DIR] [--baseline FILE] [NAME ...]

The corpus (bead domain-pascal-qia) is every customer insert
sys/ins/*.ins.pas of the SR10.2 install and every Pascal program
(*.pas but not *.ins.pas) elsewhere in it: the domain_examples, the
sys/source/emt and sys/source/bp1 samples, systest/ssr_util and the
usr/X11 examples.  The install is not part of this repository (it is
Apollo's and not redistributable); it is found at CORPUS_DIR, by default
~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2.
Without it the test is skipped with a notice (exit 0).

Preparing the files.  Many files of the install still carry their Aegis
type headers (the install was copied off a Domain file system): a 32-byte
header that starts 0x0020 0x0001, followed for a `uasc' file by plain text
and for a `rec' file by records, each a 4-byte big-endian count that
includes itself, then the data, padded to an even length.  rbak
(~/src/domainos-archeology/rbak-2.0c/src/rbak.c, `uid_table' header
sizes and `convert_rec') strips both the same way.  This script copies
every .pas file of the install into tmp/corpus/root/ (a mirror of the
install), decoding them; a file is taken as `rec' when the text after
the header parses exactly as such records, else as `uasc'.

Running dpc.  The Aegis root `/' is the mirror (`dpc --root', so
`%include '/sys/ins/base.ins.pas'' works).  A program is checked in its
own directory (relative includes such as `sasc.ins.pas'), with `-I
<mirror>' for relative `sys/ins/...' names (usr/X11/examples/dmwin) and
`-I tmp/corpus/inc', in which `X11' stands for usr/X11/include (xlib.ins.pas
includes `X11/x.ins.pas'; the install has no other directory holding an
`X11'; this search path is a harness choice, not taken from Apollo).
An insert is checked as a module that includes base.ins.pas, the inserts
of PREREQS (what the insert uses that another declares), then the insert.

Outcomes (dpc's exit status): `ok' (0), `diag' (1: diagnostics),
`internal' (3, or a signal), `timeout'.  The report gives the counts and
the distinct diagnostic kinds (messages with `names' and numbers
abstracted).  The per-file outcome and first diagnostic kind are compared
with the baseline tests/corpus/baseline.txt; any difference fails, as does
any internal error or timeout.  --update rewrites the baseline.
"""

import argparse
import collections
import glob
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DPC = os.path.join(ROOT, 'target', 'debug', 'dpc')
WORK = os.path.join(ROOT, 'tmp', 'corpus')
BASELINE = os.path.join(ROOT, 'tests', 'corpus', 'baseline.txt')
CORPUS = os.environ.get('CORPUS_DIR', os.path.expanduser(
    '~/src/domainos-archeology/sr10.2-install/install/ri.apollo.os.v.10.2'))
TIMEOUT = 30

# Inserts that use declarations of other inserts besides base.ins.pas:
# for each, a name it uses and the insert that declares it (checked:
# without that insert, dpc reports the name undefined).
PREREQS = {
    'ctm.ins.pas': ['gpr.ins.pas'],             # gpr_$pixel_value_t
    'ios_dir.ins.pas': ['ios.ins.pas'],         # ios_$open_options_t
    'ios_dir_trait.ins.pas': ['ios.ins.pas',    # ios_$open_options_t
                              'ios_dir.ins.pas'],   # ios_dir_$mode_t
    'io_traits.ins.pas': ['ios.ins.pas'],       # ios_$ec_key_t
    'passthru.ins.pas': ['ios.ins.pas',         # ios_$ec_key_t
                         'io_traits.ins.pas',   # io_$epv_ptr
                         'trait.ins.pas'],      # trait_$kind_val_t
    'sfcb.ins.pas': ['mutex.ins.pas'],          # mutex_$lock_rec_t
    'sio_trait.ins.pas': ['sio.ins.pas'],       # sio_$opt_t
    'tpad.ins.pas': ['smdu.ins.pas'],           # smd_$pos_t
}

HEADER_MAGIC = b'\x00\x20\x00\x01'
HEADER_SIZE = 32


def decode_rec(data):
    """The data of a `rec' file body, or None if `data' is not records."""
    out = []
    i = 0
    while i < len(data):
        if i + 4 > len(data):
            return None
        n = int.from_bytes(data[i:i + 4], 'big')
        if n < 4 or i + n > len(data):
            return None
        out.append(data[i + 4:i + n])
        i += n + (n & 1)
    return b''.join(out)


def decode(data):
    """(kind, text bytes) of a file of the install: plain, uasc or rec."""
    if not data.startswith(HEADER_MAGIC) or len(data) < HEADER_SIZE:
        return 'plain', data
    body = data[HEADER_SIZE:]
    rec = decode_rec(body)
    if rec is not None and body:
        return 'rec', rec
    return 'uasc', body


def corpus_files(corpus):
    """The corpus entries, as paths relative to the install, sorted:
    the inserts of sys/ins, then the programs."""
    ins = sorted(os.path.relpath(p, corpus) for p in
                 glob.glob(os.path.join(corpus, 'sys', 'ins', '*.ins.pas')))
    progs = sorted(os.path.relpath(p, corpus) for p in
                   glob.glob(os.path.join(corpus, '**', '*.pas'),
                             recursive=True)
                   if not p.endswith('.ins.pas'))
    return ins + progs


def mirror(corpus, work):
    """Copy every .pas of the install into work/root, decoded."""
    root = os.path.join(work, 'root')
    for p in glob.glob(os.path.join(corpus, '**', '*.pas'), recursive=True):
        rel = os.path.relpath(p, corpus)
        dst = os.path.join(root, rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        with open(p, 'rb') as f:
            _, text = decode(f.read())
        with open(dst, 'wb') as f:
            f.write(text)
    inc = os.path.join(work, 'inc')
    os.makedirs(inc, exist_ok=True)
    link = os.path.join(inc, 'X11')
    if os.path.lexists(link):
        os.remove(link)
    os.symlink(os.path.join(root, 'usr', 'X11', 'include'), link)
    return root, inc


def wrapper(name):
    """The module that checks insert `name'."""
    incs = ['base.ins.pas'] + PREREQS.get(name, [])
    if name != 'base.ins.pas':
        incs.append(name)
    return 'module corpus_check;\n' + ''.join(
        "%%include '/sys/ins/%s';\n" % i for i in incs)


def run_dpc(cmd, cwd, timeout=TIMEOUT):
    """(outcome, stderr text) of one dpc run."""
    try:
        r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True,
                           errors='replace', timeout=timeout)
    except subprocess.TimeoutExpired:
        return 'timeout', ''
    if r.returncode == 0:
        return 'ok', r.stderr
    if r.returncode == 1:
        return 'diag', r.stderr
    return 'internal', r.stderr


ERROR_RE = re.compile(r'(?:^|: )error: (.*)$')


def errors(stderr):
    """The error messages of a dpc run, in order."""
    out = []
    for line in stderr.splitlines():
        m = ERROR_RE.search(line)
        if m:
            out.append(m.group(1))
    return out


def kind(msg):
    """A diagnostic message with its specifics abstracted: `...` quoted
    names become `_`, type names T, file names FILE and numbers N."""
    msg = re.sub(r'`[^`]*`', '`_`', msg)
    msg = re.sub(r'(functions returning) \S+', r'\1 T', msg)
    msg = re.sub(r'(argument type) .+ (does not match parameter type) .+',
                 r'\1 T \2 T', msg)
    msg = re.sub(r'(out of range for) \S+', r'\1 T', msg)
    msg = re.sub(r'(type transfer needs equal sizes): .*',
                 r'\1: ...', msg)
    msg = re.sub(r'cannot read \S+', 'cannot read FILE', msg)
    msg = re.sub(r'\b\d+\b', 'N', msg)
    return msg


def check_one(rel, root, inc, work, timeout=TIMEOUT):
    """(outcome, error messages) of corpus entry `rel'."""
    if rel.startswith('sys/ins/') and rel.endswith('.ins.pas'):
        name = os.path.basename(rel)
        wdir = os.path.join(work, 'ins')
        os.makedirs(wdir, exist_ok=True)
        src = os.path.join(wdir, name[:-len('.ins.pas')] + '.pas')
        with open(src, 'w') as f:
            f.write(wrapper(name))
        cwd = wdir
        arg = os.path.basename(src)
    else:
        cwd = os.path.join(root, os.path.dirname(rel))
        arg = os.path.basename(rel)
    cmd = [DPC, '--check-only', '--root', root, '-I', root, '-I', inc, arg]
    outcome, stderr = run_dpc(cmd, cwd, timeout)
    msgs = errors(stderr)
    if outcome == 'internal':
        msgs = [l for l in stderr.splitlines() if 'internal error' in l] \
            or msgs or [stderr.strip()[:200]]
    return outcome, msgs


def read_baseline(path):
    """{entry: (outcome, first kind)} from a baseline file."""
    base = {}
    if not os.path.exists(path):
        return base
    with open(path) as f:
        for line in f:
            line = line.rstrip('\n')
            if not line or line.startswith('#'):
                continue
            parts = line.split('\t')
            base[parts[1]] = (parts[0], parts[2] if len(parts) > 2 else '')
    return base


def format_baseline(results):
    lines = ['# tools/corpus.py baseline: outcome, corpus entry, first '
             'diagnostic kind.',
             '# Regenerate with `python3 tools/corpus.py --update` and say '
             'why in the commit.']
    for rel, (outcome, msgs) in results.items():
        k = kind(msgs[0]) if msgs and outcome != 'ok' else ''
        lines.append('\t'.join([outcome, rel, k]).rstrip('\t'))
    return '\n'.join(lines) + '\n'


def compare(base, results):
    """Differences between the baseline and this run, as lines."""
    diffs = []
    for rel, (outcome, msgs) in results.items():
        k = kind(msgs[0]) if msgs and outcome != 'ok' else ''
        if rel not in base:
            diffs.append('new entry: %s %s %s' % (outcome, rel, k))
        elif base[rel] != (outcome, k):
            diffs.append('%s: was %s %s, now %s %s' %
                         (rel, base[rel][0], base[rel][1], outcome, k))
    for rel in base:
        if rel not in results:
            diffs.append('missing entry: %s' % rel)
    return diffs


def report(results, verbose=False):
    counts = collections.Counter(o for o, _ in results.values())
    kinds = collections.Counter()
    for outcome, msgs in results.values():
        if outcome == 'diag':
            for m in msgs:
                kinds[kind(m)] += 1
    print('corpus: %d files: %s' % (len(results), ', '.join(
        '%s %d' % (o, counts.get(o, 0))
        for o in ('ok', 'diag', 'internal', 'timeout'))))
    if kinds:
        print('corpus: %d distinct diagnostic kinds:' % len(kinds))
        for k, n in kinds.most_common():
            print('  %4d  %s' % (n, k))
    for rel, (outcome, msgs) in results.items():
        if outcome in ('internal', 'timeout') or (verbose and outcome != 'ok'):
            print('%s %s' % (outcome, rel))
            for m in msgs[:5 if verbose else 1]:
                print('    ' + m)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('-v', '--verbose', action='store_true')
    ap.add_argument('--update', action='store_true',
                    help='rewrite the baseline')
    ap.add_argument('--corpus', default=CORPUS)
    ap.add_argument('--baseline', default=BASELINE)
    ap.add_argument('names', nargs='*',
                    help='entries (paths relative to the install) to run; '
                         'no baseline comparison')
    a = ap.parse_args(argv)
    if not os.path.isdir(os.path.join(a.corpus, 'sys', 'ins')):
        print('corpus: skipped: no SR10.2 install at %s (set CORPUS_DIR)'
              % a.corpus)
        return 0
    if not os.path.exists(DPC):
        print('corpus: %s not built (run cargo build)' % DPC)
        return 1
    if os.path.isdir(WORK):
        shutil.rmtree(WORK)
    os.makedirs(WORK)
    root, inc = mirror(a.corpus, WORK)
    entries = a.names or corpus_files(a.corpus)
    results = collections.OrderedDict()
    for rel in entries:
        results[rel] = check_one(rel, root, inc, WORK)
    report(results, a.verbose)
    bad = [r for r, (o, _) in results.items() if o in ('internal', 'timeout')]
    if a.names:
        return 1 if bad else 0
    if a.update:
        os.makedirs(os.path.dirname(a.baseline), exist_ok=True)
        with open(a.baseline, 'w') as f:
            f.write(format_baseline(results))
        print('corpus: baseline written to %s' %
              os.path.relpath(a.baseline, ROOT))
        return 1 if bad else 0
    diffs = compare(read_baseline(a.baseline), results)
    for d in diffs:
        print('corpus: ' + d)
    if diffs:
        print('corpus: FAILED: %d differences from %s (if intended, rerun '
              'with --update)' % (len(diffs), os.path.relpath(a.baseline, ROOT)))
    return 1 if bad or diffs else 0


if __name__ == '__main__':
    sys.exit(main())
