#!/usr/bin/env python3
"""linktest.py - link tests against ../source (CLAUDE.md section 6, item 4).

Swap dpc objects for the C objects of the same routines in a scratch copy
of ../source's kernel build and run that build's own checks.

    linktest.py run [tests/link/manifest.txt] [--source DIR] [--rev REV]
                    [--dpc DPC] [--jobs N] [--only FILE ...] [--keep-going]

Steps (tests/link/README.md has the why of each):

  1. Refresh the scratch copy tmp/linktest/source/domain_os from
     ../source/domain_os with rsync (build/, dist/ and __pycache__ left
     out; the scratch build/ is kept between runs, so make rebuilds only
     what changed): its working tree, or with --rev the commit REV
     exported with `git archive' (reproducible: other agents' uncommitted
     edits in ../source are left out).  ../source is only read.
  2. Patch the scratch Makefile: one line `-include linktest.mk' ahead of
     its `# Main target' comment, i.e. after ALL_SUBSYSTEM_OBJS is
     complete and before the first rule that names it as a prerequisite.
     linktest.mk (written by this script) keeps the full object list as
     the target `linktest-objs' and then replaces ALL_SUBSYSTEM_OBJS with
     that list minus the dropped C objects plus the dpc objects.
  3. Compile every manifest unit with dpc (the options tools/conform.py
     gives the same unit: --fragment-origin, loop-align, a5) and assemble
     it with `m68k-elf-as -m68010' into domain_os/linktest/<stem>.o.
  4. `make linktest-objs' (every C and assembly object, nothing dropped),
     then m68k-elf-nm over them: each global the dpc object defines must
     be defined by C objects only (no hand assembly), each of those
     objects must define no global that the unit does not define, and must
     hold no module data block (`.moddata.*' / `.bss.moddata.*': bead
     domain-pascal-3bp).  Those objects are dropped; their C sources stay,
     so ../source's host tests (which #include the .c) are unaffected.
  5. `make check' in the scratch copy with linktest.mk filled in: the
     layout checks (map order of every placed symbol, scratch link), the
     hand-written assembly gate, the guards, the iodefs check, the RFC
     image check (full link with --emit-relocs, fixup table, dry-run
     loader) and the libgcc check.
  6. The linked ELF must define every global of every dpc unit once, at
     an address inside .text.
  7. ../source's tools/asm_compare.py is run over the dpc objects: their
     bytes against the image bytes (taken from the image file into
     tmp/linktest/asm_ref.txt), every relocation resolved to the image
     operand.  Routines the map does not name (`0xADDR' specs) are given
     to it as LOCAL_SYMBOLS under dpc's symbol for them, and its
     KNOWN_DEVIATIONS (entries for hand-written files, which it reports
     as stale when their object is not given) are emptied; both in the
     running script only, nothing in ../source changes.

Manifest lines (tests/link/manifest.txt): `FILE SPEC... [OPTION]...' as in
tests/conformance/manifest.txt; `--section-addr SEC=0xADDR' is accepted
and ignored for code sections (the link orders sections by their map
symbols, not by address) and refused for data sections (bead
domain-pascal-3bp); module-routines= is accepted for map names (the `bsr.w'
is a relocation the link resolves to ../source's routine) and refused for
NAME=0xADDR items; enclosing= is refused (a stand-in parent).  A unit whose
assembly has a `bsr.s' written as its encoding (`.short 0x61dd | bsr.s
NAME', a displacement fixed from the image with no relocation) is refused:
it would link and pass every check while calling the wrong place.

Exit status: 0 all checks pass; 1 a check failed; 2 setup error.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import conform  # noqa: E402
import sau2map  # noqa: E402

SOURCE = os.path.abspath(os.path.join(REPO, '..', 'source'))
SCRATCH = os.path.join(REPO, 'tmp', 'linktest')
MANIFEST = os.path.join(REPO, 'tests', 'link', 'manifest.txt')
DPC = os.path.join(REPO, 'target', 'debug', 'dpc')
NM = 'm68k-elf-nm'
OBJDIR = 'linktest'             # under the scratch domain_os/
HOOK_ANCHOR = '# Main target\n'
HOOK = ('# domain-pascal tools/linktest.py: swap dpc objects in (scratch copy '
        'only)\n-include linktest.mk\n\n')
# the kernel's image file window (tools/imgdis.sh, conform.IMAGE_BASE):
# file offset = VA - 0xDFFC00
IMAGE_WINDOW = '%s:0xDFFC00:0xC00-0x%X'


class SetupError(Exception):
    pass


# ---------------------------------------------------------------------------
# Pure logic (tools/tests/test_linktest.py)
# ---------------------------------------------------------------------------

class Unit:
    def __init__(self, path, specs, dpc_opts, lineno=0, mods=()):
        self.path = path            # relative to the repository
        self.specs = specs
        self.dpc_opts = dpc_opts    # loop-align / a5 tokens, as dpc options
        self.lineno = lineno
        self.mods = list(mods)      # module-routines= tokens (map names only)

    @property
    def stem(self):
        return os.path.splitext(os.path.basename(self.path))[0]


def parse_manifest_line(toks, lineno=0):
    """A Unit from manifest tokens (FILE SPEC... [OPTION]...)."""
    path, specs, align, a5, mods = toks[0], [], [], [], []
    if not path.endswith('.pas'):
        raise SetupError('%s: link tests take .pas units' % path)
    rest = iter(toks[1:])
    for t in rest:
        if t == '--section-addr':
            sec, _ = conform.parse_section_addr(next(rest, ''))
            if not sec.startswith('.text'):
                raise SetupError('%s: %s is a data section: module data blocks '
                                 'cannot be linked into ../source yet (bead '
                                 'domain-pascal-3bp)' % (path, sec))
        elif t.startswith(conform.LOOP_ALIGN_TOKENS):
            align.append(t)
        elif t.startswith(conform.A5_TOKENS):
            a5.append(t)
        elif t.startswith(conform.MODULE_ROUTINES_TOKEN):
            if any('=' in item for item in t[len(conform.MODULE_ROUTINES_TOKEN):].split(',')):
                raise SetupError('%s: %s: a module-local routine outside the unit has '
                                 'no name in ../source to link against' % (path, t))
            mods.append(t)
        elif t.startswith(conform.ENCLOSING_TOKEN):
            raise SetupError('%s: %s cannot be linked into ../source (the '
                             'enclosing routine is a stand-in)' % (path, t))
        else:
            specs.append(t)
    if not specs:
        raise SetupError('%s: no routine given' % path)
    try:
        opts = conform.loop_align_args(align) + conform.a5_args(a5)
    except conform.SetupError as e:
        raise SetupError('%s: %s' % (path, e))
    return Unit(path, specs, opts, lineno, mods)


def read_manifest(path, refused=None):
    """The manifest's units.  A refused line raises SetupError, or, with a
    `refused' list, is appended to it as (FILE, message) and left out."""
    units = []
    with open(path) as f:
        for i, line in enumerate(f, 1):
            toks = line.split('#', 1)[0].split()
            if not toks:
                continue
            try:
                units.append(parse_manifest_line(toks, i))
            except SetupError as e:
                if refused is None:
                    raise
                refused.append((toks[0], str(e)))
    return units


def parse_nm(text):
    """`m68k-elf-nm -A --defined-only' output -> {object: {global: type}}
    (every object named, even one with no global; local and weak symbols
    are left out)."""
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


def plan_drops(unit_globals, defs, asm_objs=()):
    """The objects to drop for one unit: every object defining one of
    `unit_globals' (from parse_nm's `defs').  Errors (a list of strings)
    when a definer is hand-written assembly, or defines a global the unit
    does not, or when a global of the unit is defined nowhere (a name the
    C tree does not have: the dpc object would add a symbol rather than
    replace one; reported, not fatal)."""
    drops, errors, notes = [], [], []
    for sym in sorted(unit_globals):
        definers = sorted(o for o, d in defs.items() if sym in d)
        if not definers:
            notes.append('%s: no object of ../source defines it (added)' % sym)
        for o in definers:
            if o in asm_objs:
                errors.append('%s is defined by hand-written assembly %s' % (sym, o))
                continue
            extra = sorted(set(defs[o]) - set(unit_globals))
            if extra:
                errors.append('%s defines %s besides %s, which the unit does not'
                              % (o, ', '.join(extra), sym))
                continue
            if o not in drops:
                drops.append(o)
    return drops, errors, notes


# The end of the kernel's wired VAs (map `D52 F4FC00 IODEFS_GUARD'): map
# symbols at or past it are absolute addresses of the I/O region
# (AST_$ZERO_BUFF 0xFF8C00), not objects of the link.
ABS_FROM = 0xF4FC00


def plan_absolutes(undefined, defined, linkmap):
    """Symbols the dpc objects reference that no object of the link
    defines: those the map puts at or past ABS_FROM become absolute
    symbols (absdefs_s); any other is an error.  -> ({name: addr},
    [error])."""
    absdefs, errors = {}, []
    for sym in sorted(set(undefined) - set(defined)):
        m = linkmap.lookup(sym)
        if m is not None and m.name == sym and m.addr >= ABS_FROM:
            absdefs[sym] = m.addr
        else:
            errors.append('%s is defined by no object of the link%s' % (
                sym, '' if m is None else ' (map 0x%X)' % m.addr))
    return absdefs, errors


def absdefs_s(absdefs):
    """An object of absolute symbols, as ../source's
    arch/m68k/sau2/iodefs.s gives the I/O-page names."""
    lines = ['| Written by domain-pascal tools/linktest.py: map names of the '
             'I/O region', '| that the dpc objects reference and ../source '
             'does not define.']
    for n, a in sorted(absdefs.items()):
        lines += ['        .globl  %s' % n, '        .equ    %s, 0x%08X' % (n, a)]
    return '\n'.join(lines) + '\n'


def patch_makefile(text):
    """../source's Makefile with the linktest.mk hook (idempotent)."""
    if HOOK in text:
        return text
    if text.count(HOOK_ANCHOR) != 1:
        raise SetupError('Makefile: anchor %r not found exactly once; the hook '
                         'needs a new place' % HOOK_ANCHOR.strip())
    return text.replace(HOOK_ANCHOR, HOOK + HOOK_ANCHOR)


def linktest_mk(drops, adds):
    def lst(xs):
        return ''.join(' \\\n    %s' % x for x in xs)
    return ('# Written by domain-pascal tools/linktest.py; scratch copy only.\n'
            'LINKTEST_ALL_OBJS := $(ALL_SUBSYSTEM_OBJS)\n'
            '.PHONY: linktest-objs\n'
            'linktest-objs: $(LINKTEST_ALL_OBJS)\n'
            'LINKTEST_DROP :=%s\n'
            'LINKTEST_ADD :=%s\n'
            'ALL_SUBSYSTEM_OBJS := $(filter-out $(LINKTEST_DROP),'
            '$(ALL_SUBSYSTEM_OBJS)) $(LINKTEST_ADD)\n' % (lst(drops), lst(adds)))


# ---------------------------------------------------------------------------
# Steps
# ---------------------------------------------------------------------------

def run(cmd, cwd=None, log=None):
    p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if log is not None:
        with open(log, 'w') as f:
            f.write('$ %s\n%s%s' % (' '.join(cmd), p.stdout, p.stderr))
    return p


def must(cmd, cwd=None, log=None):
    p = run(cmd, cwd=cwd, log=log)
    if p.returncode:
        raise SetupError('%s failed%s:\n%s' % (
            ' '.join(cmd), ' (log %s)' % conform.display_path(log) if log else '',
            tail(p.stdout + p.stderr)))
    return p.stdout


def tail(text, n=25):
    lines = text.rstrip().splitlines()
    return '\n'.join(lines[-n:])


def rsync_cmd(src, dst):
    """The one rsync of the harness: from `src' (../source, only read) into
    the scratch `dst'.  The scratch build/ and the harness's own files are
    left alone by --delete.  Contents are compared (-c) and a changed file
    gets the current time (no -t), so make rebuilds exactly what changed
    whatever the source's mtimes: a git export's are all the commit's, and
    a working-tree file may be older than an object built from another
    version of it by an earlier --rev run."""
    return (['rsync', '-rlpc', '--delete', '--exclude=/build', '--exclude=/dist',
             '--exclude=__pycache__', '--exclude=/' + OBJDIR, '--exclude=/linktest.mk',
             src.rstrip('/') + '/', dst.rstrip('/') + '/'])


def export_rev(source, rev, work):
    """`git archive' of ../source's domain_os/ at `rev' into
    work/export/domain_os (reads the repository only) -> (that directory,
    the full commit id)."""
    commit = must(['git', '-C', source, 'rev-parse', '--verify', rev + '^{commit}']).strip()
    exp = os.path.join(work, 'export')
    shutil.rmtree(exp, ignore_errors=True)
    os.makedirs(exp)
    tar = os.path.join(work, 'export.tar')
    must(['git', '-C', source, 'archive', '--format=tar', '-o', tar, commit, 'domain_os'])
    must(['tar', '-xf', tar, '-C', exp])
    os.remove(tar)
    return os.path.join(exp, 'domain_os'), commit


def refresh(source, work, rev=None):
    """Bring the scratch copy up to date with ../source/domain_os: its
    working tree (default), or the commit `rev' (reproducible: other
    agents' uncommitted edits in ../source are not picked up)."""
    if not os.path.isdir(os.path.join(source, 'domain_os')):
        raise SetupError('%s/domain_os not found' % source)
    dst = os.path.join(work, 'source', 'domain_os')
    os.makedirs(dst, exist_ok=True)
    if rev:
        src, commit = export_rev(source, rev, work)
        print('linktest: ../source at %s (%s), not its working tree' % (rev, commit[:12]))
        must(rsync_cmd(src, dst))
    else:
        must(rsync_cmd(os.path.join(source, 'domain_os'), dst))
    mk = os.path.join(dst, 'Makefile')
    with open(mk) as f:
        text = f.read()
    with open(mk, 'w') as f:
        f.write(patch_makefile(text))
    return dst


def compile_unit(unit, linkmap, dpc, root):
    """dpc + m68k-elf-as for one unit -> (object path relative to root,
    {global: section}, {local symbol: image address} for 0xADDR specs)."""
    src = os.path.join(REPO, unit.path)
    routines = conform.unit_routines(unit.specs, linkmap)
    frag = conform.fragment_args(routines, linkmap)
    mopts, _ = conform.module_routine_args(unit.mods, routines, linkmap)
    odir = os.path.join(root, OBJDIR)
    os.makedirs(odir, exist_ok=True)
    asm = conform.compile_pas(src, dpc, odir, frag + unit.dpc_opts + mopts)
    enc = conform.encoded_calls(asm)
    if enc:
        # `.short 0x61dd | bsr.s NAME': a displacement fixed from the image
        # addresses, with no relocation; linked anywhere else it lands
        # beside NAME, and nothing in ../source's checks notices
        # (ASKNODE_$INFO's bsr.s reached ASKNODE_$GET_INFO+0x26)
        raise SetupError('%s: bsr.s written as its encoding to %s: the displacement '
                         'is fixed from the image and has no relocation'
                         % (unit.path, ', '.join(sorted(enc))))
    obj = os.path.join(odir, unit.stem + '.o')
    must([conform.AS, '-m68010', '-o', obj, asm])
    globs = object_globals(obj)
    local = {}
    for r in routines:
        if r.name is None:
            local[r.addr] = r
    # dpc's symbol for an unnamed routine: the local code symbol at its
    # offset (the unit's routines are contiguous from the first)
    locals_ = {}
    if local:
        base = routines[0].addr
        lead = base & 3 if frag else 0
        for name, value in object_locals(obj):
            a = base + value - lead
            if a in local:
                locals_[name] = a
        missing = set(local) - set(locals_.values())
        if missing:
            raise SetupError('%s: no local symbol for %s' % (
                unit.path, ', '.join('0x%X' % a for a in sorted(missing))))
    return os.path.relpath(obj, root), globs, locals_


def object_globals(obj):
    out = must([NM, '--defined-only', '-g', obj])
    return {f[2]: f[1] for f in (l.split() for l in out.splitlines())
            if len(f) == 3 and f[1] in 'TDB'}


def object_undefined(obj):
    out = must([NM, '-u', obj])
    return {l.split()[-1] for l in out.splitlines() if l.strip()}


def object_locals(obj):
    out = must([NM, '--defined-only', obj])
    return [(f[2], int(f[0], 16)) for f in (l.split() for l in out.splitlines())
            if len(f) == 3 and f[1] == 't']


def nm_all(root, objs):
    defs = {}
    for i in range(0, len(objs), 400):
        defs.update(parse_nm(must([NM, '-A', '--defined-only'] + objs[i:i + 400],
                                  cwd=root)))
    return defs


def make_var(root, name):
    """The value of make variable `name' in the scratch Makefile (as
    split words)."""
    out = subprocess.run(['make', '-pq', 'linktest-objs'], cwd=root,
                         capture_output=True, text=True).stdout
    for line in out.splitlines():
        if line.startswith(name + ' :='):
            return line.split(':=', 1)[1].split()
    raise SetupError('could not read %s from the scratch Makefile' % name)


def check_swap(objs, drops, adds):
    """The link list after the hook: every dpc object in, every dropped
    object out (else the hook did not take and make check would test
    ../source's own objects)."""
    have = set(objs)
    return (['%s is not linked' % o for o in adds if o not in have] +
            ['%s is still linked' % o for o in drops if o in have])


def has_block(root, obj, nonempty=False):
    """The module data block sections of `obj' (non-empty ones only with
    `nonempty')."""
    out = must(['m68k-elf-objdump', '-h', obj], cwd=root)
    return [f[1] for f in (l.split() for l in out.splitlines())
            if len(f) >= 7 and f[0].isdigit() and
            f[1].startswith(('.moddata.', '.bss.moddata.')) and
            (int(f[2], 16) or not nonempty)]


def elf_check(root, units_globals):
    """Every dpc global defined once in the kernel ELF, inside .text."""
    elf = os.path.join(root, 'build', 'sau2', 'domain_os.elf')
    out = must([NM, elf])
    where = {}
    for f in (l.split() for l in out.splitlines()):
        if len(f) == 3:
            where.setdefault(f[2], []).append((int(f[0], 16), f[1]))
    bad = []
    for unit, globs in units_globals:
        for g in sorted(globs):
            hits = where.get(g, [])
            if len(hits) != 1 or hits[0][1] not in 'Tt':
                bad.append('%s: %s in the ELF: %s' % (unit.path, g, hits or 'missing'))
    return bad, where


def asm_compare(root, objs, local_syms, work):
    """../source's asm_compare.py over the dpc objects (step 7)."""
    ref = os.path.join(work, 'asm_ref.txt')
    shutil.copyfile(os.path.join(root, 'tools', 'asm_image_ref.txt'), ref)
    image = conform.IMAGE
    size = os.path.getsize(image)
    driver = ('import sys, json; sys.path.insert(0, sys.argv[1]); '
              'import asm_compare as a; '
              'a.LOCAL_SYMBOLS.update({k: int(v) for k, v in '
              'json.loads(sys.argv[2]).items()}); '
              'a.KNOWN_DEVIATIONS.clear(); '
              'sys.argv = [\"asm_compare.py\"] + sys.argv[3:]; a.main()')
    base = [sys.executable, '-c', driver, os.path.join(root, 'tools'),
            json.dumps(local_syms), '--map', sau2map.DEFAULT_MAP, '--root', root]
    must(base + ['--ref', ref, '--extract-file', IMAGE_WINDOW % (image, size)] + objs,
         cwd=root)
    return run(base + ['--ref', ref, '-v'] + objs, cwd=root,
               log=os.path.join(work, 'asm_compare.log'))


def run_all(args):
    linkmap = sau2map.load()
    refused = [] if args.keep_going else None
    units = read_manifest(args.manifest, refused)
    if args.only:
        want = {os.path.normpath(p) for p in args.only}
        units = [u for u in units if os.path.normpath(u.path) in want]
        refused = [r for r in refused or [] if os.path.normpath(r[0]) in want]
        if not units:
            raise SetupError('--only matches no manifest unit that can link')
    work = SCRATCH
    os.makedirs(work, exist_ok=True)
    print('linktest: refreshing %s' % conform.display_path(os.path.join(work, 'source')))
    root = refresh(args.source, work, args.rev)
    shutil.rmtree(os.path.join(root, OBJDIR), ignore_errors=True)
    mk = os.path.join(root, 'linktest.mk')
    with open(mk, 'w') as f:
        f.write(linktest_mk([], []))

    compiled, failed = [], []
    for path, msg in refused or []:
        print('linktest: FAIL %s' % msg)
        failed.append(Unit(path, [], []))
    for u in units:
        try:
            obj, globs, locals_ = compile_unit(u, linkmap, args.dpc, root)
        except (SetupError, conform.SetupError) as e:
            print('linktest: FAIL %s' % e)
            if not args.keep_going:
                return 1
            failed.append(u)
            continue
        compiled.append((u, obj, globs, locals_))
        print('linktest: compiled %s -> %s (%s)' % (u.path, obj, ', '.join(sorted(globs))))

    print('linktest: building every object of the scratch tree (make -j%d linktest-objs)'
          % args.jobs)
    must(['make', '-j%d' % args.jobs, 'linktest-objs'], cwd=root,
         log=os.path.join(work, 'objs.log'))
    allobjs = make_var(root, 'LINKTEST_ALL_OBJS')
    asm_objs = {o for o in allobjs if '/sau2/' in o[len('build/sau2/'):]}
    defs = nm_all(root, allobjs)

    drops, adds = [], []
    for u, obj, globs, _ in compiled:
        d, errs, notes = plan_drops(globs, defs, asm_objs)
        if not globs:
            errs.insert(0, 'the unit defines no global symbol: its routines are '
                        'module-local, local symbols in the dpc object, while the C '
                        'callers name the C functions; it would add unreachable code '
                        'and replace nothing')
        blk = has_block(root, obj, nonempty=True)
        if blk:
            errs.insert(0, 'the dpc object has module data %s, which this link cannot '
                        'place yet (bead domain-pascal-3bp)' % ', '.join(blk))
        for o in d:
            blk = has_block(root, o)
            if blk:
                errs.append('%s holds module data %s (bead domain-pascal-3bp)'
                            % (o, ', '.join(blk)))
        for n in notes:
            print('linktest: note: %s: %s' % (u.path, n))
        if errs:
            for e in errs:
                print('linktest: FAIL %s: %s' % (u.path, e))
            failed.append(u)
            if not args.keep_going:
                return 1
            continue
        print('linktest: %s replaces %s' % (u.path, ', '.join(d) or 'nothing'))
        drops += d
        adds.append(obj)
    undefined = set()
    for o in adds:
        undefined |= object_undefined(os.path.join(root, o))
    defined = set()
    for o, d in defs.items():
        if o not in drops:
            defined |= set(d)
    for o in adds:
        defined |= set(object_globals(os.path.join(root, o)))
    absdefs, errs = plan_absolutes(undefined, defined, linkmap)
    for e in errs:
        print('linktest: FAIL ' + e)
    if errs:
        return 1
    extra = []
    if absdefs:
        src = os.path.join(root, OBJDIR, 'absdefs.s')
        with open(src, 'w') as f:
            f.write(absdefs_s(absdefs))
        must([conform.AS, '-m68010', '-o', src[:-2] + '.o', src])
        extra.append(os.path.join(OBJDIR, 'absdefs.o'))
        print('linktest: absolute symbols from the map (%s): %s' % (
            os.path.join(OBJDIR, 'absdefs.o'),
            ', '.join('%s 0x%X' % kv for kv in sorted(absdefs.items()))))
    with open(mk, 'w') as f:
        f.write(linktest_mk(drops, adds + extra))

    bad = check_swap(make_var(root, 'ALL_SUBSYSTEM_OBJS'), drops, adds + extra)
    if bad:
        raise SetupError('the linktest.mk hook did not take: ' + '; '.join(bad))
    print('linktest: make -j%d check (scratch copy, %d C object(s) dropped, %d dpc '
          'object(s) added)' % (args.jobs, len(drops), len(adds)))
    # Relink from scratch: make would keep an ELF newer than every object
    # still listed (e.g. when no dpc object is added this run), and that
    # ELF could hold the objects of an earlier run.
    for f in ('domain_os.elf', 'layout.ld', 'layout-check.elf'):
        try:
            os.remove(os.path.join(root, 'build', 'sau2', f))
        except FileNotFoundError:
            pass
    log = os.path.join(work, 'check.log')
    p = run(['make', '-j%d' % args.jobs, 'check'], cwd=root, log=log)
    ok = p.returncode == 0
    summary = [l for l in (p.stdout + p.stderr).splitlines()
               if l.startswith(('gen_layout_ld:', 'asm_compare:', 'check_rfc:'))
               or l.strip() == 'ok' or 'error' in l.lower() or 'FAIL' in l]
    for l in summary[-30:]:
        print('    ' + l)
    if not ok:
        print('linktest: FAIL make check (log %s):\n%s'
              % (conform.display_path(log), tail(p.stdout + p.stderr)))
        return 1
    print('linktest: make check passed')

    linked = [(u, g) for u, o, g, _ in compiled if o in adds]
    bad, where = elf_check(root, linked)
    for b in bad:
        print('linktest: FAIL ' + b)
    if bad:
        return 1
    for u, g in linked:
        print('linktest: linked %s: %s' % (u.path, ', '.join(
            '%s 0x%X' % (n, where[n][0][0]) for n in sorted(g, key=lambda n: where[n][0][0]))))

    local_syms = {}
    for u, o, g, l in compiled:
        if o in adds:
            local_syms.update(l)
    p = asm_compare(root, adds, local_syms, work)
    for l in (p.stdout + p.stderr).splitlines():
        print('    ' + l)
    if p.returncode:
        print('linktest: FAIL asm_compare over the dpc objects')
        return 1
    if failed:
        print('linktest: %d unit(s) not linked: %s'
              % (len(failed), ', '.join(u.path for u in failed)))
        return 1
    print('linktest: OK, %d unit(s) linked into the kernel and checked' % len(adds))
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    r = sub.add_parser('run')
    r.add_argument('manifest', nargs='?', default=MANIFEST)
    r.add_argument('--source', default=SOURCE)
    r.add_argument('--rev', default=None,
                   help="link against this commit of ../source (git archive) "
                        "instead of its working tree")
    r.add_argument('--dpc', default=DPC)
    r.add_argument('--jobs', '-j', type=int, default=8)
    r.add_argument('--only', action='append', default=[])
    r.add_argument('--keep-going', action='store_true',
                   help='leave out units that cannot be swapped and link the rest')
    args = ap.parse_args(argv)
    try:
        miss = conform.missing_prereqs()
        if miss:
            raise SetupError('missing: %s' % ', '.join(miss))
        return run_all(args)
    except (SetupError, conform.SetupError) as e:
        print('linktest: error: %s' % e)
        return 2


if __name__ == '__main__':
    sys.exit(main())
