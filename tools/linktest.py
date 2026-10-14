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
     hold no module data block (`.moddata.*' / `.bss.moddata.*').  Those
     objects are dropped; their C sources stay, so ../source's host tests
     (which #include the .c) are unaffected.
     A unit that is a whole map code segment (no fragment option) may
     also take ../source's C helpers with it: globals that are no map
     name, defined beside its routines or alone in an object of the same
     directory, that no object left in the link references (plan_drops).
     A unit with module data (bead domain-pascal-3bp): each of its
     `.bss.moddata.*' blocks is mapped, by the image address its manifest
     `--section-addr' gives, onto the C object of ../source that starts
     there (a MODULE_DATA_DEFINE block, a C data object with a map name, or
     a gen_layout_ld.py DATA_ANCHORS entry); the declared bytes
     (`__dpc_data_size') must fit in that one object; and the unit is
     compiled again with `dpc --data-base=SECTION=SYMBOL', so it allocates
     no block and its `lea' names the C object.  A block with initial
     values (`.moddata.*', bead domain-pascal-63t) must also agree with
     the C object's initial bytes, over the bytes the unit declares (a
     NOBITS C object is zeros; a relocated C byte cannot be compared and
     fails).
  4a. dpc's run-time library (bead domain-pascal-w586; runtime/README.md):
     the archive `dpc --print-runtime=m68k' names is extracted into
     domain_os/linktest/runtime/ and its object replaces the one ../source
     object that defines its globals (math/sau2/arith.s, M$ARITH), which
     must define exactly those.  The kernel is thus linked with the
     runtime dpc ships, not ../source's copy; steps 6 and 7 check it like
     a unit (../source's own arith.o is still built and gated by its
     hand-written assembly check).
  5. `make check' in the scratch copy with linktest.mk filled in: the
     layout checks (map order of every placed symbol, scratch link), the
     hand-written assembly gate, the guards, the iodefs check, the RFC
     image check (full link with --emit-relocs, fixup table, dry-run
     loader) and the libgcc check.
  6. The linked ELF must define every global of every dpc unit once, at
     an address inside .text; a `define'd variable of a block now owned by
     C, if ../source names it too, must be at the owner + its offset.
  7. ../source's tools/asm_compare.py is run over the dpc objects: their
     bytes against the image bytes (taken from the image file into
     tmp/linktest/asm_ref.txt), every relocation resolved to the image
     operand.  Routines the map does not name (`0xADDR' specs) are given
     to it as LOCAL_SYMBOLS under dpc's symbol for them, and its
     KNOWN_DEVIATIONS (entries for hand-written files, which it reports
     as stale when their object is not given) are emptied; both in the
     running script only, nothing in ../source changes.  The C data owners
     are added to LOCAL_SYMBOLS at their image addresses too (a
     DATA_ANCHORS name is no map name or block to it), and no relocation
     may stay unverified: the data base's is proved against the image's
     `lea' operand.

Manifest lines (tests/link/manifest.txt): `FILE SPEC... [OPTION]...' as in
tests/conformance/manifest.txt; `--section-addr SEC=0xADDR' is accepted
and ignored for code sections (the link orders sections by their map
symbols, not by address); for a dpc data block it is the block's image
address, which selects the C owner (step 4); module-routines= is accepted for map names (the `bsr.w'
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
AR = 'm68k-elf-ar'
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
    def __init__(self, path, specs, dpc_opts, lineno=0, mods=(), data_addrs=None):
        self.path = path            # relative to the repository
        self.specs = specs
        self.dpc_opts = dpc_opts    # loop-align / a5 tokens, as dpc options
        self.lineno = lineno
        self.mods = list(mods)      # module-routines= tokens (map names only)
        # dpc data block section -> its image address (`--section-addr')
        self.data_addrs = dict(data_addrs or {})
        # a whole map code segment (no dpc fragment option): set by
        # compile_unit; plan_drops may then drop C helpers too
        self.whole = False

    @property
    def stem(self):
        return os.path.splitext(os.path.basename(self.path))[0]


def parse_manifest_line(toks, lineno=0):
    """A Unit from manifest tokens (FILE SPEC... [OPTION]...)."""
    path, specs, align, a5, mods, data = toks[0], [], [], [], [], {}
    if not path.endswith('.pas'):
        raise SetupError('%s: link tests take .pas units' % path)
    rest = iter(toks[1:])
    for t in rest:
        if t == '--section-addr':
            sec, addr = conform.parse_section_addr(next(rest, ''))
            if sec.startswith(DATA_PREFIXES):
                # the block's image address: which C object owns it
                if sec in data:
                    raise SetupError('%s: %s given twice' % (path, sec))
                data[sec] = addr
            elif not sec.startswith('.text'):
                raise SetupError('%s: %s is neither a code section nor a dpc '
                                 'data block (%s...)' % (path, sec, '... / '.join(DATA_PREFIXES)))
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
    return Unit(path, specs, opts, lineno, mods, data)


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


def parse_nm_undefined(text):
    """`m68k-elf-nm -A -u' output -> {object: {undefined symbol}} (only
    objects that reference something)."""
    refs = {}
    for line in text.splitlines():
        m = re.match(r'^(.*?):\s+[Uw]\s+(\S+)$', line)
        if m:
            refs.setdefault(m.group(1), set()).add(m.group(2))
    return refs


def plan_drops(unit_globals, defs, asm_objs=(), whole=None):
    """The objects to drop for one unit: every object defining one of
    `unit_globals' (from parse_nm's `defs').  Errors (a list of strings)
    when a definer is hand-written assembly, or defines a global the unit
    does not, or when a global of the unit is defined nowhere (a name the
    C tree does not have: the dpc object would add a symbol rather than
    replace one; reported, not fatal).

    `whole' is (refs, is_map_name) for a unit that is a whole map code
    segment (every routine of it: conform.fragment_args gives no option),
    with `refs' from parse_nm_undefined over every object of the link and
    `is_map_name' a predicate.  Such a unit replaces the module's C code
    entirely, so a C global that is no map name (a helper ../source made
    up: a C name for a module-local routine, a shared tail) may go with
    it, when nothing that stays references it:
      - a definer of the unit's globals may also define such helpers;
      - an object of the same directory as a definer whose globals are all
        such helpers, referenced by dropped objects only, is dropped too
        (repeated until nothing changes).
    A definer that also defines a map name outside the unit is still
    refused: that routine lives elsewhere in the image and would be lost.
    -> (drops, errors, notes)."""
    drops, errors, notes = [], [], []
    helpers = {}            # helper global -> the definer that has it
    refs, is_map_name = whole if whole else ({}, None)
    for sym in sorted(unit_globals):
        definers = sorted(o for o, d in defs.items() if sym in d)
        if not definers:
            notes.append('%s: no object of ../source defines it (added)' % sym)
        for o in definers:
            if o in asm_objs:
                errors.append('%s is defined by hand-written assembly %s' % (sym, o))
                continue
            extra = sorted(set(defs[o]) - set(unit_globals))
            if extra and whole:
                named = [x for x in extra if is_map_name(x)]
                if named:
                    errors.append('%s defines %s besides %s: map names outside the unit, '
                                  'whose code would be lost' % (o, ', '.join(named), sym))
                    continue
                for x in extra:
                    helpers.setdefault(x, o)
            elif extra:
                errors.append('%s defines %s besides %s, which the unit does not'
                              % (o, ', '.join(extra), sym))
                continue
            if o not in drops:
                drops.append(o)
    if not whole:
        return drops, errors, notes
    dirs = {os.path.dirname(o) for o in drops}
    changed = True
    while changed:
        changed = False
        for o in sorted(defs):
            gl = set(defs[o])
            if (o in drops or o in asm_objs or not gl or os.path.dirname(o) not in dirs
                    or any(is_map_name(g) for g in gl)):
                continue
            users = {u for u, r in refs.items() if r & gl}
            if users and users <= set(drops):
                drops.append(o)
                notes.append('%s: module-local C helper(s) %s, referenced only by replaced '
                             'objects: dropped' % (o, ', '.join(sorted(gl))))
                changed = True
    for h, o in sorted(helpers.items()):
        users = sorted(u for u, r in refs.items() if h in r and u not in drops)
        if users:
            errors.append('%s defines %s (no map name) besides the unit\'s globals, and %s '
                          'still references it' % (o, h, ', '.join(users)))
        else:
            notes.append('%s: C helper %s (no map name) goes with it' % (o, h))
    return drops, errors, notes


def plan_runtime(runtime_globals, defs):
    """The ../source object that dpc's run-time library replaces (step 4a):
    the one object defining its globals (from parse_nm's `defs'), which
    must define exactly those (../source's math/sau2/arith.s against
    runtime/m68k/m_arith.s: the M$ARITH module whole).  -> (object or
    None, errors)."""
    want = set(runtime_globals)
    if not want:
        return None, ['the runtime defines no global symbol']
    owners = sorted(o for o, d in defs.items() if want & set(d))
    if len(owners) != 1:
        return None, ['the runtime\'s globals are defined by %s, not by one object'
                      % (', '.join(owners) or 'no object of ../source')]
    have = set(defs[owners[0]])
    errs = []
    if have - want:
        errs.append('%s defines %s besides the runtime\'s globals'
                    % (owners[0], ', '.join(sorted(have - want))))
    if want - have:
        errs.append('%s does not define %s, which the runtime does'
                    % (owners[0], ', '.join(sorted(want - have))))
    return (None if errs else owners[0]), errs


# dpc's data blocks: `.bss.moddata.<UNIT>' for the Pascal default data
# section, `.bss.moddata.<UNIT>.<SECTION>' for a named one, and
# `.moddata.<UNIT>[.<SECTION>]' for a block with initial values
# (crates/dpc-m68k/src/emit.rs, data_section_name).
DATA_PREFIXES = ('.bss.moddata.', '.moddata.')
DATA_PREFIX = DATA_PREFIXES[0]
INIT_DATA_PREFIX = DATA_PREFIXES[1]
# what dpc --data-base records for a section (emit.rs, data_size_symbol)
DATA_SIZE_SYMBOL = '__dpc_data_size'


def pascal_data_section(sec):
    """The Pascal data section of dpc block section `sec', as dpc
    --data-base takes it (`.data' for the default section)."""
    pre = next((p for p in DATA_PREFIXES if sec.startswith(p)), None)
    if pre is None:
        raise SetupError('%s is not a dpc data block' % sec)
    _, dot, name = sec[len(pre):].partition('.')
    return name if dot else '.data'


def initial_bytes_mismatch(sec, ours, owner, cobj, theirs, relocated):
    """Why dpc block `sec''s initial bytes `ours' (its declared bytes)
    disagree with C object `owner''s initial bytes `theirs' (from
    `cobj', as many), or None.  `relocated': offsets in the C object
    that a relocation fills in (a pointer), whose bytes are not final in
    the object and so cannot be compared.  C owns the bytes in the link
    (README, "Module data"); a block with initial values says what they
    must be, and the conformance gate has tied those to the image."""
    if len(theirs) < len(ours):
        return ('%s: initial bytes: the unit declares 0x%X, %s (%s) has 0x%X'
                % (sec, len(ours), owner, cobj, len(theirs)))
    rel = sorted(o for o in relocated if o < len(ours))
    if rel:
        return ('%s: initial bytes: %s (%s) has a relocation at +0x%X; its bytes '
                'cannot be compared' % (sec, owner, cobj, rel[0]))
    bad = [i for i, (x, y) in enumerate(zip(ours, theirs)) if x != y]
    if bad:
        i = bad[0]
        return ('%s: initial bytes differ from %s (%s) first at +0x%X: dpc %02x, C %02x '
                '(%d byte(s) differ)' % (sec, owner, cobj, i, ours[i], theirs[i], len(bad)))
    return None


def data_size_symbol(psec):
    return DATA_SIZE_SYMBOL if psec == '.data' else '%s.%s' % (DATA_SIZE_SYMBOL, psec.upper())


def parse_nm_sizes(text):
    """`m68k-elf-nm -A -S --defined-only -g' output -> {global data
    symbol: (object, size)} (D, B, R, G, S types; the first definer)."""
    out = {}
    for line in text.splitlines():
        m = re.match(r'^(.*?):[0-9a-fA-F]+\s+([0-9a-fA-F]+)\s+([A-Za-z])\s+(\S+)$', line)
        if m and m.group(3) in 'DBRGS':
            out.setdefault(m.group(4), (m.group(1), int(m.group(2), 16)))
    return out


def anchor_symbols(anchors):
    """gen_layout_ld.DATA_ANCHORS ({C section: image address}) ->
    {symbol: address}: with -fdata-sections, `.data.X' / `.bss.X' /
    `.rodata.X' holds the object X."""
    out = {}
    for sec, addr in anchors.items():
        for pre in ('.data.', '.bss.', '.rodata.'):
            if sec.startswith(pre):
                out[sec[len(pre):]] = addr
    return out


def data_owner(addr, blocks, anchors, map_names, c_data):
    """The C data object of ../source whose first byte is image address
    `addr': a MODULE_DATA_DEFINE block declared there (`blocks', {name:
    addr}), a C object with a map name there (`map_names'), or one
    gen_layout_ld's DATA_ANCHORS keys there (`anchors', {symbol: addr});
    only names a link input defines as data (`c_data', parse_nm_sizes).
    -> (symbol, how), or (None, why) when there is none or more than one."""
    found = []
    for n in sorted(blocks):
        if blocks[n] == addr and n in c_data:
            found.append((n, 'MODULE_DATA_DEFINE'))
    for n in map_names:
        if n in c_data:
            found.append((n, 'map name'))
    for n in sorted(anchors):
        if anchors[n] == addr and n in c_data:
            found.append((n, 'DATA_ANCHORS'))
    names = sorted({n for n, _ in found})
    if not names:
        return None, ('no C object of ../source starts at 0x%X (no MODULE_DATA_DEFINE '
                      'block, map name or DATA_ANCHORS entry there)' % addr)
    if len(names) > 1:
        return None, 'several C objects start at 0x%X: %s' % (addr, ', '.join(names))
    return found[0]


def extent_error(sec, addr, declared, owner, obj, size):
    """Why the unit's data cannot use C object `owner' as its base, or
    None: every A5 offset of the unit (below `declared') must fall in that
    one C object (past its end lie other C objects, laid out by the link).
    This bounds the offsets only: that they hit the same fields rests on
    ../source's struct layout (tests/link/README.md, "Offsets")."""
    if declared > size:
        return ('%s (0x%X): the unit declares 0x%X bytes, ../source\'s %s (%s) is '
                '0x%X: the rest of the image block is other C objects'
                % (sec, addr, declared, owner, obj, size))
    return None


_A5_EA = re.compile(r'\((?:(-?(?:0x[0-9a-fA-F]+|\d+)),)?%a5(?:,[^)]*)?\)')


def a5_offsets(asm):
    """The displacements of the A5-relative operands of dpc assembly
    `asm' (`(d,%a5)', `(d,%a5,Xn)', `(%a5)' as 0): the offsets in the data
    block the unit touches.  The A5 save, `pea (%a5)' right before a `lea
    (.Ldata...).l,%a5', is not a data operand."""
    lines = [l.split('|', 1)[0].strip() for l in asm.splitlines()]
    lines = [l for l in lines if l and not l.endswith(':') and not l.startswith('.')]
    out = set()
    for i, l in enumerate(lines):
        if (l.replace('\t', ' ') == 'pea (%a5)' and i + 1 < len(lines) and
                re.match(r'lea\s+\(\.Ldata[^)]*\)\.l,%a5$', lines[i + 1])):
            continue
        for m in _A5_EA.finditer(l):
            out.add(int(m.group(1), 0) if m.group(1) else 0)
    return out


def reloc_addends(objdump_r, sym):
    """The addends of the relocations against `sym' in `objdump -dr'
    output (`R_68K_32 SYM+0x1e4' -> 0x1E4)."""
    out = set()
    for m in re.finditer(r'R_68K_\w+\s+(\S+?)(?:([+-])0x([0-9a-fA-F]+))?\s*$',
                         objdump_r, re.M):
        if m.group(1) == sym:
            v = int(m.group(3), 16) if m.group(3) else 0
            out.add(-v if m.group(2) == '-' else v)
    return out


def offset_evidence(used, c_addends):
    """(confirmed, unconfirmed): the unit's A5 offsets that the replaced C
    objects also reach as OWNER+offset, and the rest (whose agreement with
    the C struct rests on ../source's layout and _Static_asserts alone)."""
    return sorted(used & c_addends), sorted(used - c_addends)


def define_mismatches(defines, base_addr, elf_where):
    """The unit's `define'd variables of a block now owned by C
    ({name: offset}) against the linked ELF: a name ../source also defines
    must be at the owner's address + offset.  -> ([error], [name ../source
    does not define])."""
    errs, unnamed = [], []
    for name, off in sorted(defines.items()):
        hits = elf_where.get(name)
        if not hits:
            unnamed.append(name)
        elif len(hits) != 1 or hits[0][0] != base_addr + off:
            errs.append('%s: expected at 0x%X (block + 0x%X), the ELF has %s'
                        % (name, base_addr + off, off,
                           ', '.join('0x%X' % a for a, _ in hits)))
    return errs, unnamed


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


def parse_symtab(text):
    """`objdump -t' output -> [(name, value, binding, section)], binding
    'g' or 'l' (section `*ABS*' for an absolute symbol)."""
    out = []
    for line in text.splitlines():
        left, tab, right = line.partition('\t')
        f = left.split()
        r = right.split()
        if not tab or len(f) < 2 or len(r) != 2:
            continue
        try:
            value = int(f[0], 16)
        except ValueError:
            continue
        out.append((r[1], value, f[1] if f[1] in 'gl' else '', f[-1]))
    return out


def unit_data(symtab, blocks):
    """The data side of a dpc object: its `define'd globals per block
    section ({section: {name: offset}}; a block is its own section, so
    the value is the offset) and the sizes --data-base recorded ({Pascal
    section: bytes}, data_size_symbol)."""
    defines = {s: {} for s in blocks}
    declared = {}
    for name, value, bind, sec in symtab:
        if bind == 'g' and sec in defines:
            defines[sec][name] = value
        if sec == '*ABS*' and name.startswith(DATA_SIZE_SYMBOL):
            declared[name] = value
    return defines, declared


def compile_unit(unit, linkmap, dpc, root, data_bases=()):
    """dpc + m68k-elf-as for one unit -> (object path relative to root,
    {code global: type}, {local symbol: image address} for 0xADDR specs,
    {block section: size} of the module data blocks it allocates, the
    object's symbol table, parse_symtab).
    `data_bases': (Pascal data section, C symbol) pairs for dpc
    --data-base."""
    src = os.path.join(REPO, unit.path)
    routines = conform.unit_routines(unit.specs, linkmap)
    frag = conform.fragment_args(routines, linkmap)
    unit.whole = not frag
    mopts, _ = conform.module_routine_args(unit.mods, routines, linkmap)
    dopts = ['--data-base=%s=%s' % kv for kv in data_bases]
    odir = os.path.join(root, OBJDIR)
    os.makedirs(odir, exist_ok=True)
    asm = conform.compile_pas(src, dpc, odir, frag + unit.dpc_opts + mopts + dopts)
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
    blocks = {s: n for s, n in block_sizes(root, obj).items() if n}
    symtab = parse_symtab(must(['m68k-elf-objdump', '-t', obj]))
    # code globals: what the unit replaces (its `define'd variables are
    # data, handled with its blocks)
    globs = {g: ty for g, ty in object_globals(obj).items() if ty == 'T'}
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
    return os.path.relpath(obj, root), globs, locals_, blocks, symtab


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


def nm_undefined(root, objs):
    refs = {}
    for i in range(0, len(objs), 400):
        refs.update(parse_nm_undefined(must([NM, '-A', '-u'] + objs[i:i + 400], cwd=root)))
    return refs


def runtime_objects(dpc, root):
    """dpc's run-time library (`dpc --print-runtime=m68k', an archive),
    its members extracted into the scratch domain_os/linktest/runtime/:
    -> (their paths relative to `root', their globals)."""
    p = run([dpc, '--print-runtime=m68k'])
    lib = p.stdout.strip()
    if p.returncode or not os.path.isfile(lib):
        raise SetupError('dpc --print-runtime=m68k failed (not built? `make runtime\'):\n%s'
                         % tail(p.stderr))
    rel = os.path.join(OBJDIR, 'runtime')
    out = os.path.join(root, rel)
    shutil.rmtree(out, ignore_errors=True)
    os.makedirs(out)
    members = must([AR, 't', lib]).split()
    if not members:
        raise SetupError('%s is empty' % lib)
    must([AR, 'x', lib], cwd=out)
    objs = [os.path.join(rel, m) for m in members]
    globs = {}
    for o in objs:
        globs.update(object_globals(os.path.join(root, o)))
    return lib, objs, globs


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


def block_sizes(root, obj):
    """{section: size} of the module data block sections of `obj'."""
    out = must(['m68k-elf-objdump', '-h', obj], cwd=root)
    return {f[1]: int(f[2], 16) for f in (l.split() for l in out.splitlines())
            if len(f) >= 7 and f[0].isdigit() and
            f[1].startswith(('.moddata.', '.bss.moddata.'))}


def has_block(root, obj, nonempty=False):
    """The module data block sections of `obj' (non-empty ones only with
    `nonempty')."""
    return [s for s, n in block_sizes(root, obj).items() if n or not nonempty]


def c_layout(root):
    """../source's own placement facts, read from the scratch copy's
    tools/gen_layout_ld.py: ({MODULE_DATA_DEFINE block: declared image
    address}, {DATA_ANCHORS symbol: image address})."""
    tools = os.path.join(root, 'tools')
    sys.path.insert(0, tools)
    try:
        import importlib
        gl = importlib.import_module('gen_layout_ld')
        blocks, errors = gl.scan_sources(root)
        anchors = getattr(gl, 'DATA_ANCHORS', {})
    finally:
        sys.path.remove(tools)
    if errors:
        raise SetupError('gen_layout_ld.scan_sources: ' + '; '.join(errors))
    return {b.name: b.addr for b in blocks}, anchor_symbols(anchors)


def nm_sizes(root, objs):
    out = {}
    for i in range(0, len(objs), 400):
        for k, v in parse_nm_sizes(must([NM, '-A', '-S', '--defined-only', '-g'] +
                                        objs[i:i + 400], cwd=root)).items():
            out.setdefault(k, v)
    return out


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


def unverified_count(text):
    """asm_compare.py's count of relocations it could not resolve to an
    image address (None if its summary is missing)."""
    m = re.search(r'(\d+) unverified;', text)
    return int(m.group(1)) if m else None


def plan_data(u, root, obj, blocks, symtab, ctx, linkmap, dpc):
    """Map each module data block of unit `u''s object onto the C object
    of ../source that owns the same image address, and recompile the unit
    with dpc --data-base so that it allocates none of them.  -> ([(block
    section, image address, C symbol, how found, {define'd name: offset},
    bytes declared)], [error])."""
    errs, plan, bases = [], [], []
    defines, _ = unit_data(symtab, blocks)
    for sec in sorted(blocks):
        addr = u.data_addrs.get(sec)
        if addr is None:
            errs.append('the dpc object has module data %s with no image address '
                        '(manifest: --section-addr %s=0xADDR)' % (sec, sec))
            continue
        owner, how = data_owner(addr, ctx['blocks'], ctx['anchors'],
                                linkmap.names_at(addr), ctx['c_data'])
        if owner is None:
            errs.append('%s: %s' % (sec, how))
            continue
        plan.append([sec, addr, owner, how, defines[sec], None])
        bases.append((pascal_data_section(sec), owner))
    for sec in sorted(set(u.data_addrs) - set(blocks)):
        errs.append('%s: --section-addr names a block the dpc object does not have' % sec)
    if errs:
        return [], errs
    # a block with initial values: its bytes, before --data-base drops it
    ours = {sec: section_bytes(root, obj, sec) for sec in blocks
            if sec.startswith(INIT_DATA_PREFIX)}
    # an address initial value (bead domain-pascal-bjz) is a relocation
    # whose bytes in the object are not final: not comparable either
    for sec in sorted(ours):
        rel = sorted(relocated_offsets(root, obj, sec, 0, len(ours[sec])))
        if rel:
            errs.append('%s: initial bytes: the dpc object has a relocation at +0x%X; '
                        'its bytes cannot be compared' % (sec, rel[0]))
    if errs:
        return [], errs
    obj2, _, _, left, symtab2 = compile_unit(u, linkmap, dpc, root, bases)
    assert obj2 == obj
    if left:
        errs.append('dpc --data-base left module data %s' % ', '.join(sorted(left)))
    _, declared = unit_data(symtab2, {})
    for p in plan:
        sec, addr, owner = p[0], p[1], p[2]
        sym = data_size_symbol(pascal_data_section(sec))
        if sym not in declared:
            errs.append('%s: dpc recorded no %s' % (sec, sym))
            continue
        p[5] = declared[sym]
        cobj, size = ctx['c_data'][owner]
        e = extent_error(sec, addr, p[5], owner, cobj, size)
        if e:
            errs.append(e)
        elif sec in ours:
            theirs, relocated = c_initial_bytes(root, cobj, owner, size)
            e = initial_bytes_mismatch(sec, ours[sec][:p[5]], owner, cobj, theirs, relocated)
            if e:
                errs.append(e)
            else:
                print('linktest: %s: %s: the 0x%X declared initial bytes equal %s\'s (%s)'
                      % (u.path, sec, p[5], owner, cobj))
    return [tuple(p) for p in plan], errs


def section_bytes(root, obj, sec):
    """The contents of section `sec' of relocatable object `obj'."""
    out = os.path.join(root, OBJDIR, '_section.bin')
    must([conform.OBJCOPY, '-O', 'binary', '--only-section=' + sec, obj, out], cwd=root)
    with open(out, 'rb') as f:
        data = f.read()
    os.remove(out)
    return data


def c_initial_bytes(root, cobj, sym, size):
    """The initial bytes of data object `sym' (`size' bytes) of C object
    `cobj': zeros in a NOBITS section, else the section's contents at the
    symbol's offset; and the offsets in it that a relocation fills in."""
    entries = [e for e in parse_symtab(must(['m68k-elf-objdump', '-t', cobj], cwd=root))
               if e[0] == sym and e[3] != '*UND*']
    if not entries:
        raise SetupError('%s: no symbol %s' % (cobj, sym))
    _, value, _, sec = entries[0]
    heads = must(['m68k-elf-objdump', '-h', cobj], cwd=root).splitlines()
    for i, l in enumerate(heads):
        f = l.split()
        if len(f) >= 7 and f[0].isdigit() and f[1] == sec:
            flags = heads[i + 1] if i + 1 < len(heads) else ''
            break
    else:
        raise SetupError('%s: no section %s' % (cobj, sec))
    if 'CONTENTS' not in flags:
        return bytes(size), set()
    data = section_bytes(root, cobj, sec)[value:value + size]
    return data, relocated_offsets(root, cobj, sec, value, size)


def relocated_offsets(root, obj, sec, value, size):
    """The offsets, from `value' and below `size', of section `sec' of
    relocatable object `obj' that a relocation fills in."""
    relocated = set()
    for l in must(['m68k-elf-objdump', '-r', '-j', sec, obj], cwd=root).splitlines():
        m = re.match(r'^([0-9a-fA-F]{8})\s+R_68K_(\w+)', l)
        if m:
            off = int(m.group(1), 16) - value
            n = {'32': 4, '16': 2, '8': 1}.get(re.sub(r'^PC', '', m.group(2)), 4)
            relocated.update(o for o in range(off, off + n) if 0 <= o < size)
    return relocated


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
            obj, globs, locals_, blocks, symtab = compile_unit(u, linkmap, args.dpc, root)
        except (SetupError, conform.SetupError) as e:
            print('linktest: FAIL %s' % e)
            if not args.keep_going:
                return 1
            failed.append(u)
            continue
        compiled.append((u, obj, globs, locals_, blocks, symtab))
        print('linktest: compiled %s -> %s (%s)' % (u.path, obj, ', '.join(sorted(globs))))

    print('linktest: building every object of the scratch tree (make -j%d linktest-objs)'
          % args.jobs)
    must(['make', '-j%d' % args.jobs, 'linktest-objs'], cwd=root,
         log=os.path.join(work, 'objs.log'))
    allobjs = make_var(root, 'LINKTEST_ALL_OBJS')
    asm_objs = {o for o in allobjs if '/sau2/' in o[len('build/sau2/'):]}
    defs = nm_all(root, allobjs)
    refs = None             # nm -u of every object, for whole-module units

    drops, adds = [], []
    ctx = {}
    data_plans = {}         # unit path -> [(section, addr, owner, defines)]
    for u, obj, globs, _, blocks, symtab in compiled:
        whole = None
        if u.whole:
            if refs is None:
                refs = nm_undefined(root, allobjs)
            whole = (refs, lambda n: (lambda m: m is not None and m.name == n)(
                linkmap.lookup(n)))
        d, errs, notes = plan_drops(globs, defs, asm_objs, whole)
        if blocks:
            if not ctx:
                ctx['blocks'], ctx['anchors'] = c_layout(root)
                ctx['c_data'] = nm_sizes(root, allobjs)
            try:
                plan, derrs = plan_data(u, root, obj, blocks, symtab, ctx, linkmap,
                                        args.dpc)
            except (SetupError, conform.SetupError) as e:
                plan, derrs = [], [str(e)]
            errs = derrs + errs
            data_plans[u.path] = plan
            for sec, addr, owner, how, defines, declared in plan:
                print('linktest: %s: %s (0x%X, 0x%X bytes declared) -> %s (%s, %s), '
                      'dpc --data-base=%s=%s' % (u.path, sec, addr, declared, owner, how,
                                                 ctx['c_data'][owner][0],
                                                 pascal_data_section(sec), owner))
            if len(plan) == 1:
                # evidence that the unit's offsets mean the C struct's
                # fields: the C routines it replaces reach OWNER+offset
                # (README "Module data", "Offsets"); informational
                owner = plan[0][2]
                with open(os.path.join(root, OBJDIR, u.stem + '.s')) as f:
                    used = a5_offsets(f.read())
                c_add = set()
                for o in d:
                    c_add |= reloc_addends(must(['m68k-elf-objdump', '-dr', o], cwd=root),
                                           owner)
                ok, unk = offset_evidence(used, c_add)
                fmt = lambda xs: ', '.join('0x%X' % x for x in xs) or 'none'
                print('linktest: %s: A5 offsets used: %s; also reached as %s+offset by '
                      'the replaced C: %s; resting on ../source\'s struct layout '
                      'only: %s' % (u.path, fmt(sorted(used)), owner, fmt(ok), fmt(unk)))
            elif plan:
                print('linktest: note: %s: several data blocks, A5 offsets not '
                      'attributed to one' % u.path)
        if not globs:
            errs.insert(0, 'the unit defines no global symbol: its routines are '
                        'module-local, local symbols in the dpc object, while the C '
                        'callers name the C functions; it would add unreachable code '
                        'and replace nothing')
        blk = has_block(root, obj, nonempty=True)
        if blk and not errs:
            errs.insert(0, 'the dpc object still has module data %s after --data-base'
                        % ', '.join(blk))
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
    # step 4a: dpc's run-time library for ../source's own M$ARITH
    rt_lib, rt_objs, rt_globs = runtime_objects(args.dpc, root)
    rt_drop, errs = plan_runtime(rt_globs, defs)
    for e in errs:
        print('linktest: FAIL runtime %s: %s' % (conform.display_path(rt_lib), e))
    if errs:
        return 1
    print('linktest: runtime %s (%s) replaces %s' % (
        conform.display_path(rt_lib), ', '.join(rt_objs), rt_drop))
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
    drops.append(rt_drop)
    extra += rt_objs
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

    linked = [(u, g) for u, o, g, *_ in compiled if o in adds]
    linked.append((Unit(conform.display_path(rt_lib), [], []), rt_globs))
    bad, where = elf_check(root, linked)
    for b in bad:
        print('linktest: FAIL ' + b)
    if bad:
        return 1
    for u, g in linked:
        print('linktest: linked %s: %s' % (u.path, ', '.join(
            '%s 0x%X' % (n, where[n][0][0]) for n in sorted(g, key=lambda n: where[n][0][0]))))

    local_syms = {}
    for u, o, g, l, *_ in compiled:
        if o in adds:
            local_syms.update(l)
    bad = []
    for u, o, *_ in compiled:
        if o not in adds:
            continue
        for sec, addr, owner, how, defines, _ in data_plans.get(u.path, []):
            # the owner at its image address for asm_compare: a block or a
            # map name it resolves itself; a DATA_ANCHORS name it does not
            local_syms.setdefault(owner, addr)
            base = where.get(owner, [])
            if len(base) != 1:
                bad.append('%s: %s in the ELF: %s' % (u.path, owner, base or 'missing'))
                continue
            errs, unnamed = define_mismatches(defines, base[0][0], where)
            bad += ['%s: %s' % (u.path, e) for e in errs]
            for n in unnamed:
                print('linktest: note: %s: define\'d %s (%s + 0x%X) is not a name in '
                      '../source; nothing there refers to it'
                      % (u.path, n, owner, defines[n]))
            if not errs:
                print('linktest: %s: %s at 0x%X in the ELF; %d define\'d name(s) checked'
                      % (u.path, owner, base[0][0], len(defines) - len(unnamed)))
    for b in bad:
        print('linktest: FAIL ' + b)
    if bad:
        return 1
    p = asm_compare(root, adds + rt_objs, local_syms, work)
    for l in (p.stdout + p.stderr).splitlines():
        print('    ' + l)
    if p.returncode:
        print('linktest: FAIL asm_compare over the dpc objects')
        return 1
    unv = unverified_count(p.stdout + p.stderr)
    if unv != 0:
        # every relocation, the data base's included, must be proved
        # against the image operand
        print('linktest: FAIL asm_compare: %s relocation(s) unverified'
              % ('?' if unv is None else unv))
        return 1
    if failed:
        print('linktest: %d unit(s) not linked: %s'
              % (len(failed), ', '.join(u.path for u in failed)))
        return 1
    print('linktest: OK, %d unit(s) and the runtime linked into the kernel and checked'
          % len(adds))
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
