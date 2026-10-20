#!/usr/bin/env python3
"""linktest.py - link test against ../source (CLAUDE.md section 6, item 4).

../source builds the SR10.2 SAU2 kernel from its Domain Pascal tree with
dpc (its C sources are gone; ../source 428dab68, 4c9e9b8c).  A module dpc
refuses is a module the kernel cannot link without.  This harness builds
that tree with our dpc, as ../source's Makefile would, and says how far
the kernel is from linking.

    linktest.py run [--source DIR] [--rev REV | --working-tree] [--dpc DPC]
                    [--jobs N] [--only MODULE ...] [--baseline FILE]
                    [--update] [--link-anyway] [--no-make] [--make-var V=X ...]
                    [--no-deep] [--top N] [--closest N]

Steps (tests/link/README.md has the why of each):

  1. Export ../source's domain_os/ at REV (default HEAD) with `git
     archive' (other agents' uncommitted edits there are left out; the
     commit id is printed and recorded) and rsync it into the scratch copy
     tmp/linktest/source/domain_os (build/, dist/, __pycache__ kept out of
     the rsync, so the scratch build/ survives between runs).
     --working-tree copies ../source's working tree instead.  ../source is
     only read.
  2. The module files: ../source's own tools/pascal_modules.py, run in the
     scratch copy (the Makefile's PAS_MODULES).
  3. Each module file is compiled from the scratch domain_os/ with
     `DPC $(DPC_FLAGS) -o build/sau2/pascal/<m>.s <m>.pas', the
     Makefile's rule and flags (DPC_FLAGS, e.g. `-config sau2', read from
     the scratch copy's Makefile by tools/dpcflags.py; none if it has no
     DPC_FLAGS; `--make-var DPC_FLAGS=...' overrides), and assembled with `m68k-elf-as -m68010' (PAS_ASFLAGS), in
     parallel (--jobs).  A module dpc refuses leaves no .s/.o behind (as
     `make pascal-report' does), so the next `make' does not take a stale
     one.  Per module: OK, REFUSED (dpc reported errors), FAILED (dpc
     stopped without an error diagnostic: a panic, a signal, a timeout) or
     ASM (gas rejected dpc's output); the error count; the distinct kinds
     of its errors, normalised as tools/srcsurvey.py's refusal histogram
     does (srcsurvey.kind_of), with the type phrases of dpc's messages
     folded to `T' first (fold_types).  dpc runs without --strict, as
     ../source's build does: its best-effort warnings (a form the image
     does not settle, compiled with a correct one of dpc's own) are
     counted per module and ranked in the report.  The default dpc is the main
     checkout's target/stable/bin/dpc, also from a git worktree.
  4. The deep pass (unless --no-deep; deep_pass): dpc stops at the first
     phase that fails, so a sema or IR refusal hides every codegen
     refusal of the module.  Each module dpc refused is compiled again
     the way tools/srcsurvey.py's "check" and "codegen" stages do it
     (survey_stage, reused: the routines a diagnostic falls in are
     stubbed and dpc is run again until the module passes), in an overlay
     of the scratch tree (tmp/linktest/deep/), in parallel.  So each
     routine is charged with the refusals of the first run that rejects
     it, and the module gets every kind over all phases.  The module's
     status (step 3) is not changed by it: that is the gate.
  5. tmp/linktest/report.txt: every module with its kinds (those of the
     module compile, and over every phase with the routines charged with
     each), the ranking of error kinds, the modules closest to compiling,
     the baseline comparison and the make result.  The kinds over every
     phase are ranked by the number of modules where the kind is the ONLY
     one left (a module whose deep pass stopped short does not count
     there), then by the modules it affects; the closest modules are the
     ones with the fewest distinct kinds.  The module compiles' own
     ranking follows, for comparison.
  6. tests/link/baseline.txt (one line per module: status, module, first
     error kind; header: the ../source commit and the dpc): a module whose
     status ranks lower (STATUS_RANK), or that compiled there and is gone,
     is a regression, exit 1.  --update rewrites it (refused with --only,
     --working-tree or no m68k-elf-as); a malformed baseline is exit 2.
  7. When every module compiles, `make domain_os check' in the scratch
     copy with DPC=<our dpc> (absolute; ../source's Makefile also takes
     SAU2_MAP and SAU2_IMAGE from $HOME, --make-var overrides): the full
     link, the RFC image and ../source's own checks.  Its failure is exit
     1.  --link-anyway runs `make -k domain_os check' even when some
     modules are refused, with PAS_MODULES set to the ones that compiled,
     to show the first link errors; informational only.
"""

import argparse
import collections
import concurrent.futures
import os
import re
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import srcsurvey  # noqa: E402
import dispcheck  # noqa: E402
import dpcflags  # noqa: E402




def main_checkout(repo=REPO):
    """The main checkout of this repository: REPO itself, or, from a git
    worktree of it (.claude/worktrees/...), the checkout whose .git the
    worktree shares."""
    p = subprocess.run(['git', '-C', repo, 'rev-parse', '--path-format=absolute',
                        '--git-common-dir'], capture_output=True, text=True)
    if p.returncode == 0 and p.stdout.strip():
        return os.path.dirname(os.path.normpath(p.stdout.strip()))
    return repo


def default_source():
    """../source beside the main checkout (../source of a worktree's
    checkout, not of the worktree)."""
    return os.path.abspath(os.path.join(main_checkout(), '..', 'source'))


def default_dpc():
    """The main checkout's target/stable/bin/dpc (`make stable` there):
    a worktree has no stable build of its own."""
    return os.path.join(main_checkout(), 'target', 'stable', 'bin', 'dpc')


SCRATCH = os.path.join(REPO, 'tmp', 'linktest')
BASELINE = os.path.join(REPO, 'tests', 'link', 'baseline.txt')
AS = 'm68k-elf-as'
AS_FLAGS = ['-m68010']                  # ../source Makefile PAS_ASFLAGS
NM = 'm68k-elf-nm'                      # (tools/modsurvey.py imports it)
PAS_BUILD_DIR = os.path.join('build', 'sau2', 'pascal')   # Makefile PAS_BUILD_DIR
DPC_TIMEOUT = 300

# per module, best first
STATUS_RANK = {'OK': 3, 'ASM': 2, 'REFUSED': 1, 'FAILED': 0}

GAS_ERROR_RE = re.compile(r'^[^:]+:\d+: Error: (?P<msg>.*)$')


class SetupError(Exception):
    pass


# ------------------------------------------------------------ results

class Result:
    """One module file's outcome."""

    def __init__(self, module, status, errors=0, kinds=(), first=None):
        self.module = module
        self.status = status
        self.errors = errors        # error diagnostics (dpc) or lines (gas)
        self.kinds = list(kinds)    # distinct kinds, in order of appearance
        self.first = first          # the first error message, verbatim
        self.deep = None            # Deep: the kinds over every phase (deep_pass)
        self.best_effort = []       # kinds of dpc's best-effort warnings

    @property
    def first_kind(self):
        return self.kinds[0] if self.kinds else '-'

    @property
    def all_kinds(self):
        """The kinds over every phase (the deep pass), or, without one,
        the module compile's."""
        return list(self.deep.kinds) if self.deep is not None else self.kinds

    @property
    def complete(self):
        """Whether all_kinds covers every routine: the deep pass reached
        the end of both stages and charged no routine with a call to a
        stubbed routine only (Deep.complete), or dpc compiled it (ASM)."""
        return self.status == 'OK' or (self.deep is not None and self.deep.complete)

    def __repr__(self):
        return 'Result(%r, %r, %d, %r)' % (self.module, self.status, self.errors, self.kinds)


# dpc's messages that spell a type out (TypeTable::display in
# crates/dpc-sema/src/types.rs: a type's name, or `set of 0..7',
# `array[1..4] of char', `^foo_t', `(a, b)', `[byte] set of ...'), each with
# the type phrases as groups; every group is folded to `T', so a message is
# one kind whatever the type.  (The templates: dpc-sema conv.rs, check.rs;
# dpc-ir lower.rs.)
TYPE_PHRASES = [re.compile(p) for p in (
    r'^functions returning (?P<t1>.+?) are not supported yet',
    r'^a packed field of type (?P<t1>.+?) that is not a whole byte',
    r'^an address as the initial value of a (?P<t1>.+?) is ',
    r'^this initial value of a (?P<t1>.+?) is ',
    r'^argument type (?P<t1>.+?) (?:\(.*?\) )?does not match parameter type (?P<t2>.+?)'
    r'(?: \(|$)',
    r'^Expression passed to UNIV formal \S+ was converted to (?P<t1>.+)\.$',
    r'^type transfer needs equal sizes: (?P<t1>.+?) is \d+ bytes, (?P<t2>.+?) is \d+$',
    r'^`[^`]*` does not match the routine pointer type (?P<t1>.+?) \(ref ',
    r'^constant \S+ is out of range for (?P<t1>.+)$',
    r'not compatible with the assignment variable \((?P<t1>.+?) := (?P<t2>.+)\)$',
)]


def fold_types(msg):
    """MSG with the type phrases of a TYPE_PHRASES template -> `T'."""
    for pat in TYPE_PHRASES:
        m = pat.search(msg)
        if not m:
            continue
        spans = sorted((m.span(g) for g in m.groupdict() if m.group(g) is not None),
                       reverse=True)
        for a, b in spans:
            msg = msg[:a] + 'T' + msg[b:]
        break
    return msg


def kind_of(msg):
    """srcsurvey.kind_of (the refusal histogram's normalisation) after
    fold_types, so one refusal is one kind whatever type it names."""
    return srcsurvey.kind_of(fold_types(msg))


def _distinct(seq):
    out = []
    for x in seq:
        if x not in out:
            out.append(x)
    return out


# The prefix of dpc's best-effort warnings: a refusal of a form the image
# does not settle, compiled anyway with a correct form of dpc's own
# (`dpc --strict' refuses it; dpc_syntax::diag::best_effort, bead
# domain-pascal-5kgg).  The link test runs dpc without --strict, as
# ../source's build does.
BEST_EFFORT = 'best effort (not byte-identical): '


def best_effort_ranking(results):
    """[(kind, modules, warnings)] of the best-effort warnings of the
    modules that compile: most modules first."""
    mods, total = collections.Counter(), collections.Counter()
    for r in results:
        for k in r.best_effort:
            total[k] += 1
        for k in set(r.best_effort):
            mods[k] += 1
    return sorted(((k, mods[k], total[k]) for k in total), key=lambda t: (-t[1], -t[2], t[0]))


def dpc_result(module, returncode, stderr):
    """dpc's exit status and stderr for MODULE -> Result (status OK only
    when dpc succeeded; the caller assembles)."""
    diags = srcsurvey.parse_diags(stderr)
    errs = [d for d in diags if d['sev'] == 'error']
    if returncode == 0:
        r = Result(module, 'OK')
        r.best_effort = [kind_of(d['msg'][len(BEST_EFFORT):]) for d in diags
                         if d['sev'] == 'warning' and d['msg'].startswith(BEST_EFFORT)]
        return r
    if errs:
        return Result(module, 'REFUSED', len(errs),
                      _distinct(kind_of(d["msg"]) for d in errs),
                      '%s:%d:%d: %s' % (errs[0]['file'], errs[0]['line'], errs[0]['col'],
                                        errs[0]['msg']))
    return Result(module, 'FAILED', 1, [failure_kind(returncode, stderr)],
                  first_line(stderr) or 'exit status %s' % returncode)


def first_line(text):
    for ln in text.splitlines():
        if ln.strip():
            return ln.strip()
    return None


def failure_kind(returncode, stderr):
    """The kind of a dpc run that failed with no error diagnostic."""
    if returncode is None:
        return 'dpc: timeout'
    if returncode < 0:
        return 'dpc: signal %d' % -returncode
    lines = [ln.strip() for ln in stderr.splitlines() if ln.strip()]
    for i, ln in enumerate(lines):
        if 'panicked at' in ln:
            # Rust: "thread 'main' panicked at FILE:L:C:" then the message
            msg = lines[i + 1] if i + 1 < len(lines) and ln.endswith(':') else ln
            return 'dpc panic: ' + kind_of(msg)
    if lines:
        return 'dpc: ' + kind_of(lines[0])
    return 'dpc: exit status N'


def asm_result(module, stderr):
    """gas's stderr for MODULE's assembly -> Result with status ASM."""
    msgs = [m.group('msg') for m in map(GAS_ERROR_RE.match, stderr.splitlines()) if m]
    if not msgs:
        msgs = [first_line(stderr) or 'failed']
    return Result(module, 'ASM', len(msgs),
                  _distinct('as: ' + kind_of(m) for m in msgs), msgs[0])


def rank_kinds(results, deep=True):
    """[(kind, only, total)]: per error kind, the modules where it is the
    only kind left, and the modules it appears in; most `only' first, then
    most `total', then by name.  DEEP: the kinds over every phase
    (Result.all_kinds), and a module counts for `only' unless its deep
    pass is incomplete (Deep.complete); else the module compile's kinds
    (dpc's first failing phase only)."""
    only, total = collections.Counter(), collections.Counter()
    for r in results:
        if r.status == 'OK':
            continue
        ks = r.all_kinds if deep else r.kinds
        for k in ks:
            total[k] += 1
        if len(ks) == 1 and (not deep or r.deep is None or r.deep.complete):
            only[ks[0]] += 1
    return sorted(((k, only[k], total[k]) for k in total),
                  key=lambda t: (-t[1], -t[2], t[0]))


def closest(results):
    """The modules that do not compile, fewest distinct kinds over every
    phase first; then those whose deep pass reached the end, fewest
    routines refused, by name."""
    return sorted((r for r in results if r.status != 'OK'),
                  key=lambda r: (len(r.all_kinds), not r.complete,
                                 len(r.deep.stubbed) if r.deep else 0, r.module))


def format_closest(results, limit=None):
    rs = closest(results)
    lines = ['  kinds  routines  module: kinds']
    for r in rs[:limit]:
        lines.append('  %5d  %8s  %s%s: %s' % (
            len(r.all_kinds), len(r.deep.stubbed) if r.deep else '-', r.module,
            '' if r.complete else ' (incomplete)', '; '.join(_short(k) for k in r.all_kinds)))
    if limit is not None and len(rs) > limit:
        lines.append('  ... %d more module(s) in the report' % (len(rs) - limit))
    return lines


def _short(kind, n=90):
    return kind if len(kind) <= n else kind[:n - 3] + '...'


def counts(results):
    return collections.Counter(r.status for r in results)


def summary_line(results):
    c = counts(results)
    s = '%d of %d module files compile' % (c['OK'], len(results))
    rest = [('%d refused by dpc' % c['REFUSED']) if c['REFUSED'] else None,
            ('%d dpc failure(s)' % c['FAILED']) if c['FAILED'] else None,
            ('%d not assembled' % c['ASM']) if c['ASM'] else None]
    rest = [x for x in rest if x]
    return s + ('; ' + ', '.join(rest) if rest else '')


def format_ranking(ranking, limit=None):
    lines = ['  only  total  kind']
    for k, o, t in ranking[:limit]:
        lines.append('  %4d  %5d  %s' % (o, t, k))
    if limit is not None and len(ranking) > limit:
        lines.append('  ... %d more kind(s) in the report' % (len(ranking) - limit))
    return lines


# ------------------------------------------------------------ baseline

def format_baseline(commit, results, dpc=None):
    """The baseline file's text: the ../source commit and the dpc (its
    stable commit, or its path), then one line per module: status, module
    file, first error kind (`-' when it compiles)."""
    lines = ['# tools/linktest.py baseline: status, module file, first error kind.',
             '# Regenerate with `python3 tools/linktest.py run --update` and say why',
             '# in the commit.  `make linktest` compares with it (exit 1 when a',
             '# module\'s status got worse or a module that compiled is gone).',
             '# source: %s' % commit,
             '# dpc: %s' % (dpc or 'unknown'),
             '# %s' % summary_line(results)]
    lines += ['%s\t%s\t%s' % (r.status, r.module, r.first_kind) for r in results]
    return '\n'.join(lines) + '\n'


class BaselineError(SetupError):
    pass


def read_baseline(path):
    """-> ({'source': commit, 'dpc': dpc} as far as given,
    {module: (status, first kind)}).  A malformed line is a
    BaselineError naming the file and line."""
    meta, out = {}, {}
    with open(path) as fh:
        for n, ln in enumerate(fh, 1):
            ln = ln.rstrip('\n')
            for key in ('source', 'dpc'):
                if ln.startswith('# %s: ' % key):
                    meta[key] = ln[len('# %s: ' % key):].strip()
            if not ln or ln.startswith('#'):
                continue
            f = ln.split('\t', 2)
            if len(f) != 3 or not f[1] or not f[2]:
                raise BaselineError('%s:%d: malformed line (want STATUS<TAB>MODULE<TAB>KIND): %r'
                                    % (path, n, ln))
            if f[0] not in STATUS_RANK:
                raise BaselineError('%s:%d: unknown status %r (one of %s)'
                                    % (path, n, f[0], ', '.join(STATUS_RANK)))
            if f[1] in out:
                raise BaselineError('%s:%d: module %s listed twice' % (path, n, f[1]))
            out[f[1]] = (f[0], f[2])
    return meta, out


def compare_baseline(base, results, modules=None):
    """Differences between BASE ({module: (status, kind)}) and RESULTS,
    restricted to MODULES when given -> (lines, regressions).  A
    regression is a module whose status ranks lower than in the baseline
    (STATUS_RANK: OK, ASM, REFUSED, FAILED), or a module that compiled
    there and is gone (no longer a module file of ../source: renamed or
    removed, which says so).  A gone module that did not compile, a new
    one and a change of first kind are reported, not counted."""
    now = {r.module: (r.status, r.first_kind) for r in results}
    lines, regressions = [], 0
    for m in sorted(set(base) | set(now)):
        if modules is not None and m not in modules:
            continue
        b, c = base.get(m), now.get(m)
        if b == c:
            continue
        if c is None:
            worse = b[0] == 'OK'
            tag = 'GONE ' if worse else 'gone '
        elif b is None:
            worse, tag = False, 'new  '
        else:
            rb, rc = STATUS_RANK[b[0]], STATUS_RANK[c[0]]
            worse = rc < rb
            tag = 'WORSE' if worse else 'FIXED' if c[0] == 'OK' else \
                'better' if rc > rb else 'diff '
        regressions += worse
        fmt = lambda x: 'none' if x is None else (x[0] if x[0] == 'OK' else '%s (%s)' % x)
        lines.append('  %s %s: %s -> %s' % (tag, m, fmt(b), fmt(c)))
    return lines, regressions


# ------------------------------------------------------------ scratch copy

def rsync_cmd(src, dst):
    """The one rsync of the harness: from SRC (only read) into the
    scratch DST.  The scratch build/ and dist/ are left alone by --delete.
    Contents are compared (-c) and a changed file gets the current time
    (no -t), so make rebuilds exactly what changed whatever the source's
    mtimes (a git export's are all the commit's)."""
    return ['rsync', '-rlpc', '--delete', '--exclude=/build', '--exclude=/dist',
            '--exclude=__pycache__', src.rstrip('/') + '/', dst.rstrip('/') + '/']


def must(cmd, cwd=None):
    p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if p.returncode:
        raise SetupError('%s failed:\n%s' % (' '.join(cmd), (p.stdout + p.stderr)[-2000:]))
    return p.stdout


def refresh(source, work, rev=None):
    """Bring WORK/source/domain_os up to date with SOURCE's domain_os/ at
    REV (git archive), or its working tree when REV is None -> (the scratch
    domain_os, the commit id or a working-tree description)."""
    if not os.path.isdir(os.path.join(source, 'domain_os')):
        raise SetupError('%s/domain_os not found' % source)
    dst = os.path.join(work, 'source', 'domain_os')
    os.makedirs(dst, exist_ok=True)
    if rev:
        try:
            src, commit = srcsurvey.export_rev(source, rev, work)
        except subprocess.CalledProcessError as e:
            raise SetupError('git export of %s at %s failed: %s' % (
                source, rev, (e.stderr or b'').decode(errors='replace').strip()))
    else:
        src = os.path.join(source, 'domain_os')
        head = subprocess.run(['git', '-C', source, 'rev-parse', 'HEAD'],
                              capture_output=True, text=True).stdout.strip()
        commit = 'working tree of %s' % (head or 'unknown')
    must(rsync_cmd(src, dst))
    return dst, commit


def module_files(tree):
    """../source's own list (tools/pascal_modules.py, the Makefile's
    PAS_MODULES), run in TREE."""
    out = must([sys.executable, os.path.join('tools', 'pascal_modules.py')], cwd=tree)
    mods = [ln.strip() for ln in out.splitlines() if ln.strip()]
    if not mods:
        raise SetupError('tools/pascal_modules.py listed no module file in %s' % tree)
    return mods


def outputs(module):
    stem = os.path.join(PAS_BUILD_DIR, module[:-len('.pas')] if module.endswith('.pas') else module)
    return stem + '.s', stem + '.o'


def compile_module(dpc, tree, module, assemble=True, flags=()):
    """Compile (and assemble) MODULE as ../source's Makefile does, from
    TREE (the scratch domain_os), with its DPC_FLAGS FLAGS (put before
    `-o', as the Makefile's rule does) -> (Result, dpc stderr)."""
    s, o = outputs(module)
    os.makedirs(os.path.join(tree, os.path.dirname(s)), exist_ok=True)
    for f in (s, o):
        if os.path.exists(os.path.join(tree, f)):
            os.remove(os.path.join(tree, f))
    try:
        p = subprocess.run([dpc] + list(flags) + ['-o', s, module], cwd=tree, capture_output=True, text=True,
                           timeout=DPC_TIMEOUT)
        rc, err = p.returncode, p.stderr
    except subprocess.TimeoutExpired:
        rc, err = None, ''
    with open(os.path.join(tree, s + '.log'), 'w') as fh:
        fh.write(err)
    r = dpc_result(module, rc, err)
    if r.status != 'OK':
        if os.path.exists(os.path.join(tree, s)):
            os.remove(os.path.join(tree, s))
        return r, err
    if assemble:
        # Every displacement checked against its target (gas wraps an
        # out-of-reach one silently; tools/dispcheck.py): a bad one is an
        # assembly failure.
        a, bad = dispcheck.assemble_checked(s, o, AS_FLAGS, cwd=tree, as_cmd=AS)
        if a.returncode or bad:
            if os.path.exists(os.path.join(tree, o)):
                os.remove(os.path.join(tree, o))
            if bad:
                return Result(module, 'ASM', len(bad), ['dispcheck: bad displacement'],
                              'dispcheck: bad displacement: ' + bad[0]), err
            return asm_result(module, a.stderr), err
    return r, err


def compile_all(dpc, tree, modules, jobs, assemble=True, flags=()):
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as ex:
        futs = [ex.submit(compile_module, dpc, tree, m, assemble, flags) for m in modules]
        return [f.result()[0] for f in futs]


# ------------------------------------------------------------ deep pass

class Deep:
    """A module's refusals over every phase: what tools/srcsurvey.py's
    check and codegen stages charge its routines with (deep_pass)."""

    def __init__(self):
        self.kinds = {}         # kind -> routines charged with it ('-': outside any)
        self.stubbed = []       # the routines stubbed, in order
        self.problem = None     # why the pass stopped short of the end, or None
        self.notes = []         # srcsurvey's remarks
        self.unseen = []        # (routine, callee): stubbed for calling a stubbed routine

    @property
    def complete(self):
        return self.problem is None and not self.unseen

    def add(self, kind, routine):
        at = self.kinds.setdefault(kind, [])
        r = routine or '-'
        if r not in at:
            at.append(r)

    def __repr__(self):
        return 'Deep(%r, %r)' % (self.kinds, self.problem)


class _Log:
    """A log for srcsurvey.survey_stage that keeps every dpc stderr."""

    def __init__(self):
        self.runs = []

    def write(self, text):
        self.runs.append(text)

    @property
    def last(self):
        return self.runs[-1] if self.runs else ''


def overlay_tree(src, dst, writable):
    """DST as a copy of the tree SRC in which only WRITABLE (paths
    relative to SRC) are files of their own, copied; every other entry is
    a symbolic link to SRC's (a directory with nothing writable in it, as
    one link).  build/ and dist/ are left out.  The deep pass stubs
    routines in the writable files only, so SRC is never written."""
    src = os.path.abspath(src)
    need = set()
    for w in writable:
        d = os.path.dirname(w)
        while d:
            need.add(d)
            d = os.path.dirname(d)
    os.makedirs(dst)

    def walk(rel):
        for name in sorted(os.listdir(os.path.join(src, rel))):
            r = os.path.join(rel, name) if rel else name
            if not rel and name in ('build', 'dist'):
                continue
            if r in writable and os.path.isfile(os.path.join(src, r)):
                shutil.copyfile(os.path.join(src, r), os.path.join(dst, r))
            elif r in need and os.path.isdir(os.path.join(src, r)):
                os.mkdir(os.path.join(dst, r))
                walk(r)
            else:
                os.symlink(os.path.join(src, r), os.path.join(dst, r))
    walk('')


def deep_dir(work, module):
    return os.path.join(work, 'deep', module[:-len('.pas')] if module.endswith('.pas') else module)


# dpc's error for a call of an internal routine whose body is not in the
# unit (dpc-sema): after stubbing, a call of a stubbed internal routine
# srcsurvey had no address for (module_routine_opts needs the header's
# "Original address: 0xADDR, N bytes"; bead domain-pascal-626x)
STUB_CALL_RE = re.compile(r'^call of internal routine `([^`]+)`, which has no body in this unit')


def stop_reason(stage, mod, err, loc):
    """Why srcsurvey's STAGE stopped short for MOD (survey_stage returned
    not ok), from its last dpc run (exit status MOD.last_rc, stderr ERR;
    LOC: the srcsurvey.Locator of the tree) -> (Deep.problem, the kind of
    a dpc failure or None)."""
    errs = [d for d in srcsurvey.parse_diags(err) if d['sev'] == 'error']
    rc = getattr(mod, 'last_rc', 1)
    if mod.problem:
        return '%s stage, %s' % (stage, mod.problem), None
    if not errs:
        k = failure_kind(rc, err)
        what = 'dpc timed out (%d s)' % DPC_TIMEOUT if rc is None else \
            'dpc died of signal %d' % -rc if rc < 0 else 'dpc failed without an error'
        return '%s stage, %s' % (stage, what), k
    if any(loc(d['file'], d['line']) is None for d in errs):
        return '%s stage, stops at an error outside any routine' % stage, None
    return ('%s stage, nothing new to stub (the errors fall in routines stubbed already)'
            % stage), None


def deep_pass(dpc, tree, module, work, linkmap=None):
    """deep_pass_unit, with any exception it raises (a bug, an unreadable
    file) recorded in Deep.problem, so one module does not stop the run."""
    deep = Deep()
    try:
        deep_pass_unit(deep, dpc, tree, module, work, linkmap)
    except Exception as e:      # noqa: BLE001 - per module, reported
        deep.problem = 'the deep pass failed: %s: %s' % (type(e).__name__, e)
    return deep


def deep_pass_unit(deep, dpc, tree, module, work, linkmap=None):
    """MODULE's refusals over every phase, into DEEP.  dpc stops at the first
    phase that fails (a sema or IR refusal hides every codegen one), so
    the module is compiled as tools/srcsurvey.py's stages "check" (`dpc
    --check-only') and "codegen" do it (survey_stage): every routine a
    diagnostic falls in is stubbed and dpc is run again until the module
    passes (the stubbed internal routines named with --module-routine,
    their code sections from LINKMAP, the SAU2 map).  Each routine is
    charged with the errors of the first stage and run that rejects it.
    The module and its fragments are copied into an overlay of TREE
    (overlay_tree, under WORK/deep/), so TREE is left as it is.  The pass
    stops short (Deep.problem) at an error outside any routine (a
    declaration) or in a routine srcsurvey cannot stub, or when dpc fails
    without an error.  A routine charged only with a call of a stubbed
    internal routine that srcsurvey could not name with --module-routine
    (STUB_CALL_RE) is listed in Deep.unseen, not as a kind: the module's
    kinds are then incomplete too (Deep.complete).  Each dpc run has
    DPC_TIMEOUT, as the module compile.  Fills DEEP."""
    dst = deep_dir(work, module)
    if os.path.lexists(dst):
        shutil.rmtree(dst)
    try:
        with open(os.path.join(tree, module)) as fh:
            text = fh.read()
    except OSError as e:
        deep.problem = 'cannot read the module file: %s' % e
        return deep
    frags = [i for i in srcsurvey.module_includes(text) if not i.endswith('.ins.pas')]
    overlay_tree(tree, dst, {module} | set(frags))
    log = _Log()
    try:
        try:
            mod = srcsurvey.Module(dst, module)
        except (OSError, ValueError) as e:
            deep.problem = 'srcsurvey cannot read the module: %s' % e
            return deep
        done = []
        for stage, check_only in (('check', True), ('codegen', False)):
            entries, stubbed, ok = srcsurvey.survey_stage(dpc, mod, check_only, done, log,
                                                          linkmap, note=deep.notes.append,
                                                          timeout=DPC_TIMEOUT)
            done += stubbed
            for d, r in entries:
                if d['sev'] != 'error':
                    continue
                m = STUB_CALL_RE.match(d['msg'])
                if m and m.group(1).lower() in mod.stubbed:
                    # srcsurvey could not name the stubbed callee with
                    # --module-routine (no address in its header): an
                    # artifact of the stubbing, not a kind of the module
                    if (r or '-', m.group(1)) not in deep.unseen:
                        deep.unseen.append((r or '-', m.group(1)))
                    continue
                deep.add(kind_of(d['msg']), r)
            if ok:
                continue
            deep.problem, k = stop_reason(stage, mod, log.last, srcsurvey.Locator(dst))
            if k:
                deep.add(k, None)
            break
        deep.stubbed = done
    finally:
        with open(dst + '.log', 'w') as fh:
            fh.write(''.join(log.runs))
    return deep


def deep_all(dpc, tree, results, work, jobs, linkmap=None, strict=False):
    """Result.deep for every module of RESULTS that dpc refused or failed
    on (deep_pass, in parallel); an ASM module's kinds are gas's, all of
    them (dpc compiled it); a dpc timeout is not run again.  STRICT: dpc
    --strict for srcsurvey's stages (srcsurvey.DPC_STRICT); the link test
    runs without it, as its own compiles and ../source's build do, so the
    ranking is what still blocks after best effort (bead
    domain-pascal-5kgg)."""
    srcsurvey.DPC_STRICT = strict
    if os.path.isdir(os.path.join(work, 'deep')):
        shutil.rmtree(os.path.join(work, 'deep'))      # no stale module left
    todo = []
    for r in results:
        if r.status == 'ASM':
            r.deep = Deep()
            for k in r.kinds:
                r.deep.add(k, None)
        elif r.status == 'FAILED' and r.kinds == ['dpc: timeout']:
            r.deep = Deep()
            r.deep.add(r.kinds[0], None)
            r.deep.problem = 'not run again (dpc timed out)'
        elif r.status in ('REFUSED', 'FAILED'):
            todo.append(r)
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as ex:
        futs = [(r, ex.submit(deep_pass, dpc, tree, r.module, work, linkmap)) for r in todo]
        for r, f in futs:
            r.deep = f.result()
            # the module compile's kinds are the deep pass's first run's;
            # one that is not (the stages' `--check-only', `-I .') is kept
            for k in r.kinds:
                if k not in r.deep.kinds:
                    r.deep.add(k, '?')
                    r.deep.notes.append('linktest: %r: from the module compile only' % k)


# ------------------------------------------------------------ make

# a line of ../source's make output that reports a failure: the linker's
# own lines, make's `***', a tool's `error:' / `Error:', a FAIL; not
# asm_compare's notes (`run atomic_op_error ends ...')
MAKE_ERROR_RE = re.compile(r'^(?:m68k-elf-)?ld\b|^make(?:\[\d+\])?: \*\*\*|\b[Ee]rror:|\bFAIL')
UNDEF_RE = re.compile(r"undefined (?:reference to|symbol) [`']([^'`]+)'")


def make_errors(log, limit=12):
    """-> (the first distinct lines of a make log that report a failure,
    other than undefined symbols; {undefined symbol: lines naming it})."""
    undef = collections.Counter(UNDEF_RE.findall(log))
    lines = _distinct(ln.rstrip() for ln in log.splitlines()
                      if MAKE_ERROR_RE.search(ln) and not UNDEF_RE.search(ln))
    return lines[:limit], undef


def run_make(tree, dpc, jobs, targets, make_vars=(), keep_going=False, modules=None, log=None):
    cmd = ['make', '-j%d' % jobs] + (['-k'] if keep_going else []) + \
        ['DPC=%s' % dpc] + list(make_vars)
    if modules is not None:
        cmd.append('PAS_MODULES=%s' % ' '.join(modules))
    cmd += list(targets)
    p = subprocess.run(cmd, cwd=tree, capture_output=True, text=True)
    shown = [('PAS_MODULES=<%d modules>' % len(modules)) if c.startswith('PAS_MODULES=') else c
             for c in cmd]
    text = '$ %s\n%s%s' % (' '.join(shown), p.stdout, p.stderr)
    if log:
        with open(log, 'w') as fh:
            fh.write(text)
    return p.returncode, text


def clean_link_outputs(tree):
    """The kernel and its layout are regenerated by every make, but make
    keeps them when no prerequisite changed; remove them so a run always
    relinks (a failed earlier link may have left them half-written)."""
    for f in ('domain_os.elf', 'layout.ld', 'layout-check.elf'):
        p = os.path.join(tree, 'build', 'sau2', f)
        if os.path.exists(p):
            os.remove(p)


# ------------------------------------------------------------ driver

def dpc_commit(dpc):
    """target/stable/COMMIT beside a stable dpc, or None."""
    p = os.path.join(os.path.dirname(os.path.dirname(dpc)), 'COMMIT')
    try:
        with open(p) as fh:
            return fh.read().strip() or None
    except OSError:
        return None


def module_lines(results):
    """Per module: the status, the module compile's error count, first
    error and kinds (`-'); then, from the deep pass, every kind over all
    phases with the routines charged with it (`*'), and why the pass
    stopped short, if it did."""
    out = []
    for r in results:
        if r.status == 'OK':
            n = len(r.best_effort)
            out.append('OK       %s%s' % (r.module, ' (%d best-effort warning(s))' % n if n else ''))
            continue
        out.append('%-8s %s: %d error(s); first: %s' % (r.status, r.module, r.errors, r.first))
        for k in r.kinds:
            out.append('           - %s' % k)
        d = r.deep
        if d is None:
            continue
        out.append('           all phases: %d kind(s), %d routine(s) stubbed%s' % (
            len(d.kinds), len(d.stubbed), '' if d.problem is None else
            '; INCOMPLETE: %s' % d.problem))
        for k, at in d.kinds.items():
            out.append('           * %s  [%s]' % (k, ', '.join(at)))
        if d.unseen:
            out.append('           INCOMPLETE: %d routine(s) stubbed for a call of a stubbed '
                       'internal routine srcsurvey has no address for, their own kinds not seen: '
                       '%s' % (len(d.unseen), ', '.join('%s (%s)' % u for u in d.unseen)))
        for n in d.notes:
            out.append('           note: %s' % n)
    return out


def cmd_run(a):
    dpc = os.path.abspath(a.dpc)
    if not os.access(dpc, os.X_OK):
        raise SetupError('dpc not found or not executable: %s (`make stable` builds the default)'
                         % dpc)
    assemble = shutil.which(AS) is not None
    if a.update:
        # the baseline records a reproducible, complete run
        if a.only:
            raise SetupError('--update needs every module (no --only)')
        if a.working_tree:
            raise SetupError('--update needs a commit of ../source (no --working-tree)')
        if not assemble:
            raise SetupError('--update needs %s (a module is OK only once assembled)' % AS)
    base = None
    if not a.update and os.path.exists(a.baseline):
        base = read_baseline(a.baseline)
    os.makedirs(a.work, exist_ok=True)
    tree, commit = refresh(a.source, a.work, None if a.working_tree else a.rev)
    modules = module_files(tree)
    if a.only:
        unknown = [m for m in a.only if m not in modules]
        if unknown:
            raise SetupError('not a module file of ../source: %s' % ', '.join(unknown))
        modules = [m for m in modules if m in a.only]
    dc = dpc_commit(dpc)
    dpc_id = dc or dpc
    # the Makefile's DPC_FLAGS (-config ...), for these compiles and the
    # deep pass's (srcsurvey.SOURCE_DPC_FLAGS)
    flags, how = dpcflags.dpc_flags(tree, a.make_var)
    srcsurvey.SOURCE_DPC_FLAGS[:] = flags
    head = ['linktest: ../source %s%s' % (commit, '' if a.working_tree else ' (%s)' % a.rev),
            'linktest: dpc %s%s' % (dpc, ' (stable build of %s)' % dc[:12] if dc else ''),
            'linktest: DPC_FLAGS %s (%s)' % (' '.join(flags) or '(none)', how)]
    if not assemble:
        head.append('linktest: %s not found: modules are compiled, not assembled; no make'
                    % AS)
    for ln in head:
        print(ln)
    sys.stdout.flush()

    t0 = time.monotonic()
    results = compile_all(dpc, tree, modules, a.jobs, assemble, flags)
    t1 = time.monotonic()
    timing = 'compile %.1f s' % (t1 - t0)
    deep = not a.no_deep
    if deep:
        try:
            import sau2map
            linkmap = sau2map.load()
        except OSError as e:
            linkmap = None
            head.append('linktest: no SAU2 map (%s): the deep pass gives a module of several '
                        'code sections one fragment origin' % e)
        deep_all(dpc, tree, results, a.work, a.jobs, linkmap)
        timing += ', deep pass %.1f s' % (time.monotonic() - t1)
    timing += ' (-j %d)' % a.jobs
    print('linktest: %s' % timing)
    ranking = rank_kinds(results, deep)
    title = ('== error kinds over every phase, by modules they alone block (only) and modules '
             'affected (total)' if deep else
             '== error kinds (dpc\'s first failing phase only: --no-deep), by modules they alone '
             'block (only) and modules affected (total)')
    rep = list(head) + ['linktest: %s' % timing, '', '== modules'] + module_lines(results)
    rep += ['', title] + format_ranking(ranking)
    if deep:
        rep += ['', '== closest to compiling: fewest distinct kinds over every phase']
        rep += format_closest(results)
        rep += ['', '== error kinds of the module compiles alone (dpc stops at the first '
                'failing phase), for comparison'] + format_ranking(rank_kinds(results, False))
    be = best_effort_ranking(results)
    if be:
        rep += ['', '== best-effort warnings (not byte-identical) of the modules that compile, '
                'by modules and warnings']
        rep += ['  %4d  %5d  %s' % (m, n, k) for k, m, n in be]

    rc = 0
    cmp_lines = []
    if a.update:
        with open(a.baseline, 'w') as fh:
            fh.write(format_baseline(commit, results, dpc_id))
        cmp_lines.append('baseline %s rewritten' % os.path.relpath(a.baseline, REPO))
    elif base is not None:
        meta, entries = base
        diffs, worse = compare_baseline(entries, results, set(modules) if a.only else None)
        if meta.get('source') != commit:
            cmp_lines.append('note: the baseline is at ../source %s, this run at %s; differences '
                             'may come from ../source' % (meta.get('source'), commit))
        if meta.get('dpc') != dpc_id:
            cmp_lines.append('note: the baseline was made with dpc %s, this run uses %s'
                             % (meta.get('dpc'), dpc_id))
        cmp_lines += diffs or ['  no change']
        cmp_lines.append('%d regression(s) against %s' % (worse, os.path.relpath(a.baseline, REPO)))
        if worse:
            rc = 1
    else:
        cmp_lines.append('no baseline %s (create it with --update)' % os.path.relpath(a.baseline, REPO))
    rep += ['', '== baseline'] + cmp_lines

    c = counts(results)
    all_ok = c['OK'] == len(results) and not a.only
    make_lines = []
    if a.no_make or not assemble or a.only:
        make_lines.append('make: not run')
    elif all_ok or a.link_anyway:
        clean_link_outputs(tree)
        log = os.path.join(a.work, 'make.log')
        ok_mods = None if all_ok else [r.module for r in results if r.status == 'OK']
        mrc, text = run_make(tree, dpc, a.jobs, ['domain_os', 'check'], a.make_var,
                             keep_going=not all_ok, modules=ok_mods, log=log)
        what = 'make domain_os check' if all_ok else \
            'make -k domain_os check with the %d compiled modules only (--link-anyway)' % c['OK']
        make_lines.append('%s: %s (log %s)' % (what, 'OK' if mrc == 0 else 'FAILED, exit %d' % mrc,
                                                os.path.relpath(log, REPO)))
        if mrc:
            errs, undef = make_errors(text)
            make_lines += ['  ' + ln for ln in errs]
            if undef:
                make_lines.append('  %d undefined symbol(s), e.g. %s' % (
                    len(undef), ', '.join(s for s, _n in undef.most_common(8))))
            if all_ok:
                rc = 1
    else:
        make_lines.append('make domain_os check: not run, %d module file(s) do not compile '
                          '(--link-anyway links the rest)' % (len(results) - c['OK']))
    rep += ['', '== make'] + make_lines
    rep += ['', '== summary', summary_line(results)]

    os.makedirs(os.path.dirname(a.report), exist_ok=True)
    with open(a.report, 'w') as fh:
        fh.write('\n'.join(rep) + '\n')

    print(summary_line(results))
    if ranking:
        print('error kinds%s, by modules they alone block (only) and modules affected (total):'
              % (' over every phase' if deep else ' (first failing phase only)'))
        for ln in format_ranking(ranking, a.top):
            print(ln)
    if deep and ranking:
        print('closest to compiling (fewest distinct kinds over every phase):')
        for ln in format_closest(results, a.closest):
            print(ln)
    for ln in cmp_lines + make_lines:
        print(ln)
    print('report: %s' % os.path.relpath(a.report, REPO))
    return rc


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    r = sub.add_parser('run', help='build ../source\'s Pascal tree with dpc and compare')
    r.add_argument('--source', default=None, help='the ../source repository (only read)')
    r.add_argument('--rev', default='HEAD', help='the ../source commit to export (default HEAD)')
    r.add_argument('--working-tree', action='store_true',
                   help="copy ../source's working tree instead of a commit (not reproducible)")
    r.add_argument('--dpc', default=None,
                   help="the dpc to test (default: the main checkout's target/stable/bin/dpc)")
    r.add_argument('-j', '--jobs', type=int, default=os.cpu_count() or 4)
    r.add_argument('--only', action='append', metavar='MODULE',
                   help='only this module file (e.g. ast/ast.pas); repeatable; no make')
    r.add_argument('--baseline', default=BASELINE)
    r.add_argument('--update', action='store_true', help='rewrite the baseline')
    r.add_argument('--link-anyway', action='store_true',
                   help='run make -k domain_os check with the compiled modules even when '
                        'some are refused')
    r.add_argument('--no-make', action='store_true', help='never run ../source\'s make')
    r.add_argument('--make-var', action='append', default=[], metavar='VAR=VALUE',
                   help="a variable for ../source's make (e.g. SAU2_MAP=...); repeatable")
    r.add_argument('--top', type=int, default=20, help='error kinds to print (all in the report)')
    r.add_argument('--closest', type=int, default=10,
                   help='modules closest to compiling to print (all in the report)')
    r.add_argument('--no-deep', action='store_true',
                   help="no deep pass: rank the module compiles' kinds (dpc's first failing "
                        'phase) alone')
    r.add_argument('--work', default=SCRATCH, help=argparse.SUPPRESS)
    r.add_argument('--report', default=None, help='default WORK/report.txt')
    a = ap.parse_args(argv)
    if a.source is None:
        a.source = default_source()
    if a.dpc is None:
        a.dpc = default_dpc()
    if a.report is None:
        a.report = os.path.join(a.work, 'report.txt')
    try:
        return cmd_run(a)
    except SetupError as e:
        print('linktest: %s' % e, file=sys.stderr)
        return 2


def parse_nm(text):
    """`m68k-elf-nm -A --defined-only' output -> {object: {global: type}}
    (every object named, even one with no global; local and weak symbols
    are left out).  Not used here any more: tools/modsurvey.py's."""
    defs = {}
    for line in text.splitlines():
        m = re.match(r'^(.*?):(?:[0-9a-fA-F]+)?\s+([A-Za-z?])\s+(\S+)$', line)
        if not m:
            continue
        obj, typ, name = m.groups()
        defs.setdefault(obj, {})
        if typ.isupper() and typ not in 'UW':
            defs[obj][name] = typ
    return defs


if __name__ == '__main__':
    sys.exit(main())
