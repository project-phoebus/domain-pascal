#!/usr/bin/env python3
"""conform.py - image-conformance gate (CLAUDE.md section 6, item 3).

Assemble a unit, link its code at the image address of the routine(s) it
reproduces, and compare the bytes with the SR10.2 SAU2 kernel image.

    conform.py check FILE.s --routine NAME [--routine NAME ...]
    conform.py check FILE.s --addr 0xADDR[:LEN] --routine NAME
    conform.py check FILE.s ... --local-sym NAME=0xADDR
    conform.py check FILE.o ...        (a prebuilt object, not assembled)
    conform.py manifest tests/conformance/manifest.txt [--dpc target/debug/dpc]

A manifest line is `FILE SPEC... [--section-addr SEC=0xADDR]... [OPTION]...'
where OPTION is `loop-align=on', `loop-align=off' or
`loop-align-off-from=NAME' (.pas units only), forwarded to dpc as
`--loop-align=...' / `--loop-align-off-from=NAME' (codegen-patterns 7:
loop alignment is on for a leading run of a segment's routines and off for
the rest; `tools/loopalign.py switch SEG' shows where), or one of
`a5-always-from=NAME', `a5-always=NAME,...', `a5-absolute=NAME,...'
(.pas units only, each at most once), forwarded as `--a5-...' (which
routines load the data base A5, codegen-patterns 4; `tools/a5survey.py
switch SEG' shows a segment's modes), or `merge-constant-stores' (.pas
units only, at most once), forwarded as `--merge-constant-stores' (the
unit's segment merges constant pairs, codegen-patterns 13 "Store
merging"; `tools/storesurvey.py units' lists the segments), or
`module-routines=NAME,...' (.pas
fragments only, at most once): routines of the unit's map module outside
the unit, which the unit declares `extern' and the original reached with
`bsr'; each is forwarded as `--module-routine=NAME=0xADDR' with its map
address (codegen-patterns 1.6, 7.4: DISK_$UNASSIGN_ALL 0xE6BE34), or as
`--module-routine=SECTION:NAME=0xADDR' with its map output section when
it lies in another code segment of the module (one of the same name in
another output section; dpc then calls it with `jsr abs.l' from other
sections) or the unit has routines in several.  An
item `NAME=0xADDR' names a module-local routine the map does not name
(test-only, bead domain-pascal-3h9): NAME is the symbol dpc gives the
routine the unit declares `internal; extern' (its Pascal name upper-cased,
a local symbol), and ADDR must lie in the unit's map module, outside the
unit, with no map name; it is forwarded the same way, and the reference is
resolved with `--defsym NAME=0xADDR' at link time (`check --local-sym').
`check --local-sym NAME=0xADDR' is checked the same way.
`enclosing=NAME' (.pas fragments only, at most once; `check --enclosing
NAME'): the unit's routines are nested in routine NAME, dpc's symbol for
it, which follows them; it is compiled with them (their code reaches its
frame), must be linked at the unit's end, where the map has NAME (or no
name, when NAME is not a map name), and is left out of every comparison
(codegen-patterns 6; tests/conformance/README.md, "Nested routines").

FILE may also be an object file (.o), linked and compared as it is: dpc's
prebuilt run-time library (`make runtime', runtime/README.md), so that the
object shipped is the one checked.

`global=NAME[,NAME...]': map names the unit must also define as global
code symbols, at their map addresses, that are not routines of their own:
a routine's second name (M$OIU$LLL, M$OIS$LLL's, both at 0xE0AD7C).

A routine SPEC is `NAME', `NAME:LEN', `0xADDR' or `0xADDR:LEN' (LEN is hex,
`0x' optional).  NAME is looked up in the SAU2 map; an address names a
module-local routine the map does not list ("module-local at 0xADDR").
Without LEN, the length runs to the next map symbol or the end of the map
segment, whichever comes first.  `--len LEN' overrides the length when
exactly one routine is given.

The routines of one unit must be contiguous in the image (e.g. a
module-local worker, then its caller), or, for a module with code in
several map segments (one per output section: TTY in WIRED_PROC,
OS_INIT_PROC and .TEXT), contiguous within each segment ("Fragments of
several code sections" below).  All code sections of the object
without a `--section-addr' are concatenated in object order and linked at
the lowest routine address.  A code section with `--section-addr SEC=0xADDR'
is another code section of the unit's module (a Pascal code section of its
own, dpc's `.text.<UNIT>.<SECTION>', a segment of another output section in
the map: SCSI_ in OS_INIT_PROC and SCSI_WIRED_PROC): it is linked at ADDR and
compared whole, bytes and relocations, ADDR must start a code segment of the
map and the section must be exactly as long, and its global symbols are
checked like the unit's.  Such a unit cannot be a fragment at 2 mod 4.

Fragments.  The compiler's 4-byte alignment (loop heads, module-end
padding with 0x2048; codegen-patterns 7) is relative to the module's
4-aligned code base, i.e. absolute.  A unit that is only part of a module
is compiled with `dpc --fragment-origin LO' (LO = its first routine's
address): dpc then records LO in the absolute local symbol
`__dpc_fragment_origin', starts the code with LO mod 4 bytes of padding
so that section-relative `.balignw 4' is absolute alignment, and emits no
module-end padding (unless also given --fragment-ends-module).  When the
object has that symbol (it must equal LO), the first code section is
linked at LO & ~3 and its leading LO mod 4 bytes, which must be zero, are
left out of every other comparison.  The manifest passes the option for
every .pas unit except one that is a whole map module (starts at its
start, ends at its end), adding --fragment-ends-module when the unit ends
its module; a unit that runs past the end of its module is refused.

Fragments of several code sections.  A unit whose routines lie in several
map segments of one module is compiled as a fragment of each:
`--fragment-origin=SECTION=0xLO' per run of routines (SECTION the map's
output section, LO the run's first address) and
`--fragment-ends-module=SECTION' for a run that ends its segment.  dpc
records each origin in `__dpc_fragment_origin<output section>'; each such
code section is linked at its origin less its pad and compared with the
run that starts there (tests/conformance/README.md, "Fragments of several
code sections").

Steps:
  1. m68k-elf-as -m68010 (a .o file is taken as it is).
  2. Every undefined symbol is resolved from the map (`--defsym', exact
     spelling), or from `--local-sym NAME=0xADDR' (a module-local routine
     outside the unit, which the map does not name); any other symbol is
     an error, and so is a `--local-sym' the object does not reference.
     A common symbol (dpc's overlay section, `var (name)', ref 3.14) is
     resolved to the map's overlay section of that name, a data section
     with no module segment (PCHIST_UNWIRED_D 0xE85718); a common larger
     than that section is a failure.  So is dpc's NOBITS overlay section
     `.bss.overlay.<NAME>' (a unit `define's variables in it): placed
     there unless `--section-addr' says otherwise, and a failure if it is
     placed elsewhere or is larger.  Non-code
     sections are placed with `--section-addr SEC=0xADDR', or else at an
     address outside the image (references to them then fail step 4).
  3. The linked code is compared byte for byte with the image, and so is
     every non-code section with contents (dpc's `.moddata.<UNIT>[.<SECT>]',
     a module data block with initial values), which must start a data
     segment of the map and fit in it; such a section without
     `--section-addr' is a failure.  A zero-filled block (NOBITS,
     `.bss.moddata.') is placed but has no bytes to compare.  A 32-bit field without a relocation
     whose value is a map address (>= 0x10000) is a failure: the reference
     was hard-coded rather than named.
  4. Every relocation of the object is checked symbolically: the image's
     operand at that place must equal the relocation's target (S+A, or
     S+A-P for PC-relative types), so a reference matches only if it names
     a symbol at the right address.  Each named routine must be a global
     code symbol; every global code symbol must be a map name (exact
     spelling) linked at its map address.
  5. Result: one line `OK NAME 0xVA LEN  FILE' per routine (exit 0), or the
     problems and a side-by-side disassembly, ours | image, with differing
     lines marked `*' (exit 1).  Setup errors exit 2.

Relocation matching follows the idea of ../source/domain_os/tools/
asm_compare.py (step 3 there: R_68K_32/16/8 must equal S + A, R_68K_PC*
S + A - P); no code is shared with it.

Python 3 standard library only.  Scratch files go to tmp/conform/<path of
the unit>/, so units with the same file name do not share one.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import sau2map  # noqa: E402

IMAGE = os.environ.get(
    'DOMAIN_OS_IMAGE',
    os.path.expanduser('~/src/domainos-archeology/sr10.2-install/install/'
                       'ri.apollo.os.v.10.2/sau2/domain_os'))
# file offset = VA - IMAGE_BASE, for VA in [IMAGE_LO, IMAGE_HI)  (tools/imgdis.sh)
IMAGE_BASE = 0xDFFC00
IMAGE_LO = 0xE00800
IMAGE_HI = 0xE88834

AS = 'm68k-elf-as'
LD = 'm68k-elf-ld'
OBJDUMP = 'm68k-elf-objdump'
OBJCOPY = 'm68k-elf-objcopy'
TOOLS = (AS, LD, OBJDUMP, OBJCOPY)

SCRATCH = os.path.join(REPO, 'tmp', 'conform')
UNPLACED_BASE = 0x40000000      # non-code sections without --section-addr
CODE_OUT = '.conform_code'


def code_out(k):
    """The output section of the k-th run of code (CODE_OUT for the first)."""
    return CODE_OUT if k == 0 else '%s%d' % (CODE_OUT, k)

FRAGMENT_ORIGIN = '__dpc_fragment_origin'   # dpc_m68k::emit::FRAGMENT_ORIGIN_SYMBOL

RELOC_SIZE = {'R_68K_32': 4, 'R_68K_16': 2, 'R_68K_8': 1,
              'R_68K_PC32': 4, 'R_68K_PC16': 2, 'R_68K_PC8': 1}


class SetupError(Exception):
    """Bad input or environment (exit 2), as opposed to a mismatch."""


def missing_prereqs(need_image=True):
    """A list of what is missing to run the harness (empty if all there)."""
    miss = [t for t in TOOLS if shutil.which(t) is None]
    if not os.path.exists(sau2map.DEFAULT_MAP):
        miss.append(sau2map.DEFAULT_MAP)
    if need_image and not os.path.exists(IMAGE):
        miss.append(IMAGE)
    return miss


def run(cmd, cwd=None):
    p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if p.returncode:
        raise SetupError('%s failed:\n%s%s' % (' '.join(cmd), p.stdout, p.stderr))
    return p.stdout


def display_path(path):
    """`path' relative to the repository when it lies inside it."""
    rel = os.path.relpath(os.path.abspath(path), REPO)
    return path if rel.startswith('..') else rel


def scratch_dir(path):
    """tmp/conform/<path>/ for unit file `path' (relative to the repository
    when inside it), created if needed."""
    rel = display_path(path).lstrip(os.sep)
    work = os.path.join(SCRATCH, rel)
    os.makedirs(work, exist_ok=True)
    return work


def read_image(va, n):
    if va < IMAGE_LO or va + n > IMAGE_HI:
        raise SetupError('range 0x%X..0x%X is outside the image file '
                         '(0x%X..0x%X)' % (va, va + n, IMAGE_LO, IMAGE_HI))
    with open(IMAGE, 'rb') as f:
        f.seek(va - IMAGE_BASE)
        return f.read(n)


# ---- routine specs -------------------------------------------------------

class Routine:
    def __init__(self, name, addr, length):
        self.name = name        # map name, or None for module-local
        self.addr = addr
        self.length = length

    @property
    def label(self):
        return self.name or 'module-local@0x%X' % self.addr

    @property
    def end(self):
        return self.addr + self.length


def parse_hex(s):
    return int(s[2:] if s.lower().startswith('0x') else s, 16)


def parse_spec(spec, linkmap):
    head, _, length = spec.partition(':')
    if head.lower().startswith('0x'):
        name, addr = None, parse_hex(head)
    else:
        sym = linkmap.lookup(head)
        if sym is None:
            raise SetupError('routine %s is not in the map' % head)
        name, addr = sym.name, sym.addr
    n = parse_hex(length) if length else linkmap.extent_at(addr)[1]
    return Routine(name, addr, n)


# ---- object inspection ---------------------------------------------------

def sections_of(obj):
    """[(name, size, flags)] of the object's sections, in object order."""
    out = []
    lines = run([OBJDUMP, '-h', obj]).splitlines()
    for i, line in enumerate(lines):
        f = line.split()
        if len(f) >= 7 and f[0].isdigit() and i + 1 < len(lines):
            flags = {x.strip() for x in lines[i + 1].split(',')}
            out.append((f[1], int(f[2], 16), flags))
    return out


_SYM_RE = re.compile(r'^([0-9a-f]{8}) (.{7}) (\S+)\t[0-9a-f]{8} (.*)$')


def symbols_of(path):
    """[(name, value, section, flags)] from objdump -t."""
    out = []
    for line in run([OBJDUMP, '-t', path]).splitlines():
        m = _SYM_RE.match(line)
        if m:
            out.append((m.group(4).strip(), int(m.group(1), 16), m.group(3), m.group(2)))
    return out


_RELOC_RE = re.compile(r'^([0-9a-f]{8}) (R_68K_\S+)\s+(\S+?)(?:([+-])0x([0-9a-f]+))?$')


def relocs_of(obj):
    """{section: [(offset, type, symbol, addend)]} from objdump -r."""
    out, cur = {}, None
    for line in run([OBJDUMP, '-r', obj]).splitlines():
        m = re.match(r'^RELOCATION RECORDS FOR \[(.*)\]:', line)
        if m:
            cur = m.group(1)
            continue
        m = _RELOC_RE.match(line.strip())
        if m and cur:
            add = int(m.group(5), 16) if m.group(5) else 0
            if m.group(4) == '-':
                add = -add
            out.setdefault(cur, []).append((int(m.group(1), 16), m.group(2), m.group(3), add))
    return out


# ---- disassembly ---------------------------------------------------------

_INSN_RE = re.compile(r'^\s*([0-9a-f]+):\t([0-9a-f ]+?)\s*(?:\t(.*))?$')


def disassemble(data, va, workdir, tag):
    """[(addr, hex, text)] for raw bytes at va (objdump, 68010)."""
    if not data:
        return []
    path = os.path.join(workdir, tag + '.bin')
    with open(path, 'wb') as f:
        f.write(data)
    out = run([OBJDUMP, '-D', '-b', 'binary', '-m', 'm68k:68010',
               '--adjust-vma=0x%x' % va, path])
    insns = []
    for line in out.splitlines():
        m = _INSN_RE.match(line)
        if not m:
            continue
        if m.group(3) is None and insns:        # continuation of a long insn
            a, h, t = insns[-1]
            insns[-1] = (a, h + ' ' + m.group(2), t)
        else:
            insns.append((int(m.group(1), 16), m.group(2), (m.group(3) or '').strip()))
    return insns


def side_by_side(ours, image, context=4):
    """Lines `M ADDR  ours | image', M = '*' where they differ.  Long
    listings keep only `context' lines around each difference."""
    a = {x[0]: x for x in ours}
    b = {x[0]: x for x in image}
    rows = []
    for addr in sorted(set(a) | set(b)):
        l, r = a.get(addr), b.get(addr)
        diff = l is None or r is None or l[1] != r[1]
        lt = '%-15s %-28s' % (l[1], l[2][:28]) if l else ' ' * 44
        rt = '%-15s %s' % (r[1], r[2]) if r else ''
        rows.append((diff, '%s %06x  %s | %s' % ('*' if diff else ' ', addr, lt, rt)))
    if len(rows) > 40:
        keep = set()
        for i, (d, _) in enumerate(rows):
            if d:
                keep.update(range(i - context, i + context + 1))
        out, last = [], -1
        for i, (_, text) in enumerate(rows):
            if i in keep:
                if last >= 0 and i != last + 1:
                    out.append('  ...')
                out.append(text)
                last = i
        return out
    return [t for _, t in rows]


# ---- the check -----------------------------------------------------------

def unit_routines(specs, linkmap, length=None):
    """The routines of a unit, sorted and checked to be contiguous within
    each map segment (a unit of a module with several code sections has
    one run of routines per segment, see unit_groups)."""
    routines = sorted((parse_spec(s, linkmap) for s in specs), key=lambda r: r.addr)
    if not routines:
        raise SetupError('no routine given')
    if length is not None:
        if len(routines) != 1:
            raise SetupError('--len needs exactly one routine')
        routines[0].length = length
    for r0, r1 in zip(routines, routines[1:]):
        if r0.end != r1.addr and same_segment(r0.addr, r1.addr, linkmap):
            raise SetupError('routines are not contiguous: %s ends at 0x%X, %s starts at 0x%X'
                             % (r0.label, r0.end, r1.label, r1.addr))
    return routines


def same_segment(a, b, linkmap):
    """Do addresses `a' and `b' lie in the same map segment?"""
    return linkmap.segment_containing(a) is linkmap.segment_containing(b)


def unit_groups(routines, linkmap):
    """The routines of a unit (sorted, from unit_routines) split by map
    segment: one contiguous run per code section of the unit.  A unit of
    several segments is one module (README.md, "Fragments of several code
    sections"): its segments have one name, each in its own output
    section."""
    groups = []
    for r in routines:
        if groups and same_segment(groups[-1][-1].addr, r.addr, linkmap):
            groups[-1].append(r)
        else:
            groups.append([r])
    if len(groups) > 1:
        segs = [linkmap.segment_containing(g[0].addr) for g in groups]
        if any(sg is None or sg.is_section for sg in segs):
            raise SetupError('routines in several places, not all in map segments')
        if len({sg.name for sg in segs}) > 1:
            raise SetupError('routines of several modules (%s); one unit is one module'
                             % ', '.join(sorted({sg.name for sg in segs})))
        names = [code_section_of(sg.addr, linkmap) for sg in segs]
        if len(set(names)) != len(names):
            raise SetupError('two runs of routines in one output section (%s)'
                             % ', '.join(names))
    return groups


def code_section_of(addr, linkmap):
    """The name of the map's output code section holding `addr' (WIRED_PROC,
    OS_INIT_PROC, .TEXT): dpc's Pascal code section of a routine there
    (matched case-insensitively; `.text' is the default)."""
    for sec in linkmap.sections:
        if sec.kind == 'I' and sec.size and sec.contains(addr):
            return sec.name
    raise SetupError('0x%X is in no code section of the map' % addr)


# the problem line of a run of the wrong length (tools/srcsurvey.py
# parses it: SIZE_PROBLEM_RE)
SIZE_PROBLEM = 'size: ours 0x%X bytes, image range 0x%X..0x%X is 0x%X'


def check(src, specs, linkmap, section_addrs=None, length=None, quiet=False, out=print,
          work=None, name=None, local_syms=None, enclosing=None, required_globals=None):
    """Check one assembly unit.  Returns True on a match.  Raises SetupError
    on bad input.  `work' is the scratch directory (default: from `src'),
    `name' the unit's name in the result lines (default: `src').
    `local_syms' maps the symbols of module-local routines outside the
    unit, which the map does not name, to their image addresses
    (`module-routines=NAME=0xADDR').
    `enclosing' is the symbol of the routine the unit's nested routines
    belong to, whose code follows them in `src' and in the image and is
    not compared (`enclosing=NAME'; README.md, "Nested routines").
    `required_globals' are map names the unit must define as global code
    symbols besides its routines' (`global=NAME')."""
    routines = unit_routines(specs, linkmap, length)
    groups = unit_groups(routines, linkmap)
    lo, hi = routines[0].addr, groups[0][-1].end
    if enclosing is not None:
        if len(groups) > 1:
            raise SetupError('enclosing=%s: the unit has routines in several code sections'
                             % enclosing)
        named = linkmap.names_at(hi)
        if named and enclosing not in named:
            raise SetupError('enclosing=%s: the map has %s at the end of the unit, 0x%X'
                             % (enclosing, named[0], hi))
        if not named and linkmap.lookup(enclosing) is not None:
            raise SetupError('enclosing=%s: a map name, but not at the end of the unit, 0x%X'
                             % (enclosing, hi))
    section_addrs = dict(section_addrs or {})
    name = name or display_path(src)

    work = work or scratch_dir(src)
    obj = os.path.join(work, 'unit.o')
    if src.endswith('.o'):
        # a prebuilt object (dpc's runtime, target/runtime/m68k/m_arith.o):
        # checked as built, not assembled again
        if not os.path.isfile(src):
            raise SetupError('%s not found (not built?)' % display_path(src))
        shutil.copyfile(src, obj)
    else:
        run([AS, '-m68010', '-o', obj, os.path.abspath(src)])

    # 2. place sections, resolve externals from the map
    secs = sections_of(obj)
    code = [s for s, n, fl in secs if 'CODE' in fl and n]
    other = [s for s, n, fl in secs if 'CODE' not in fl and 'ALLOC' in fl and n]
    if not code:
        raise SetupError('%s has no code' % src)
    for s in section_addrs:
        if s not in other and s not in code:
            raise SetupError('--section-addr %s: no such section with contents' % s)
    # dpc's overlay section defined by a unit with `define'd variables and
    # no initial values (`.bss.overlay.<NAME>', NOBITS; emit.rs,
    # overlay_area, bead domain-pascal-m15q): placed, like a common
    # symbol, at the map's overlay section NAME unless --section-addr says
    # otherwise (gas relocates the unit's own references against the
    # section, so it must be placed); checked below
    nobits_overlays = {}
    for s, n, fl in secs:
        if s in other and s.startswith('.bss.overlay.'):
            ov = linkmap.overlay(s[len('.bss.overlay.'):])
            if ov is None:
                raise SetupError('%s: section %s names no overlay section of the map'
                                 % (src, s))
            nobits_overlays[s] = (ov, n)
            section_addrs.setdefault(s, ov.addr)
    # code sections with --section-addr: a unit's other code sections
    # (a module with routines in several Pascal code sections, which the
    # map has as segments of different output sections), each linked at
    # its own address and compared whole
    extra = [s for s in code if s in section_addrs]
    code = [s for s in code if s not in section_addrs]
    if not code:
        raise SetupError('%s: every code section has a --section-addr; the unit\'s '
                         'routines must be in one without' % src)

    # a fragment compiled with dpc --fragment-origin: `pad' leading bytes
    # put section-relative alignment on absolute boundaries; they are
    # linked below the fragment's first byte and not compared.  One code
    # section: the symbol __dpc_fragment_origin.  Several (dpc
    # --fragment-origin=SECTION=0xADDR): __dpc_fragment_origin<output
    # section> for each, and each code section is the fragment of one run
    # of the unit's routines (unit_groups), the one starting there.
    syms = symbols_of(obj)
    origin = [v for n, v, sec, _ in syms if n == FRAGMENT_ORIGIN and sec == '*ABS*']
    per_section = {n[len(FRAGMENT_ORIGIN):]: v for n, v, sec, _ in syms
                   if n.startswith(FRAGMENT_ORIGIN + '.') and sec == '*ABS*'}
    # regions: [(code sections, lo, hi, pad)], one per run of routines
    regions = []
    if per_section:
        if extra:
            raise SetupError('%s: a fragment\'s code sections are placed at their origins, '
                             'not with --section-addr (%s)' % (src, ', '.join(extra)))
        for s in code:
            if s not in per_section:
                raise SetupError('%s: code section %s has no fragment origin' % (src, s))
        for s, v in sorted(per_section.items(), key=lambda kv: kv[1]):
            if s not in code:
                raise SetupError('%s: fragment origin for %s, which has no code' % (src, s))
            g = [g for g in groups if g[0].addr == v]
            if not g:
                raise SetupError('%s: code section %s was compiled as a fragment at 0x%X, but '
                                 'no run of the unit\'s routines starts there' % (src, s, v))
            # the run's map output section must be the code section's
            # (dpc's `.text' or `.text.<UNIT>.<SECTION>'): a routine
            # compiled into another section would match here but be
            # linked into the wrong output section in ../source
            want = code_section_of(v, linkmap)
            if s.rsplit('.', 1)[-1].upper() != want.lstrip('.').upper():
                raise SetupError('%s: code section %s holds the routines at 0x%X, which the '
                                 'map has in output section %s' % (src, s, v, want))
            regions.append(([s], v, g[0][-1].end, v % 4))
        # each run in exactly one code section (two sections with one
        # origin would leave another run uncompared)
        starts = [r[1] for r in regions]
        twice = sorted({a for a in starts if starts.count(a) > 1})
        if twice:
            raise SetupError('%s: several code sections compiled as fragments at %s'
                             % (src, ', '.join('0x%X' % a for a in twice)))
        if len(regions) != len(groups):
            missing = [g[0].label for g in groups if g[0].addr not in per_section.values()]
            raise SetupError('%s: no code section for the unit\'s routines from %s'
                             % (src, ', '.join(missing)))
    else:
        if len(groups) > 1:
            raise SetupError('%s: the unit\'s routines are in %d map segments, but it was not '
                             'compiled as a fragment of each (dpc --fragment-origin='
                             'SECTION=0xADDR)' % (src, len(groups)))
        pad = 0
        if origin:
            if origin[0] != lo:
                raise SetupError('%s was compiled as a fragment at 0x%X, but the unit starts '
                                 'at 0x%X' % (src, origin[0], lo))
            pad = lo % 4
        if extra and pad:
            raise SetupError('%s: a fragment has one code section (dpc --fragment-origin)'
                             % src)
        regions.append((code, lo, hi, pad))
    code = [s for r in regions for s in r[0]]

    markers = {}                # input section -> marker symbol
    script = ['SECTIONS {']
    for k, (rsecs, rlo, _, rpad) in enumerate(regions):
        script.append('  %s 0x%X : SUBALIGN(2) {' % (code_out(k), rlo - rpad))
        for s in rsecs:
            markers[s] = '__conform_%d' % len(markers)
            script.append('    %s = .; unit.o(%s)' % (markers[s], s))
        script.append('  }')
    for i, s in enumerate(extra):
        markers[s] = '__conform_c%d' % i
        script.append('  .conform_c%d 0x%X : SUBALIGN(2) { %s = .; unit.o(%s) }'
                      % (i, section_addrs[s], markers[s], s))
    unplaced = UNPLACED_BASE
    for i, s in enumerate(other):
        markers[s] = '__conform_d%d' % i
        addr = section_addrs.get(s)
        if addr is None:
            addr, unplaced = unplaced, unplaced + 0x100000
        script.append('  .conform_d%d 0x%X : SUBALIGN(2) { %s = .; unit.o(%s) }' % (i, addr, markers[s], s))
    script += ['  /DISCARD/ : { *(*) }', '}']
    with open(os.path.join(work, 'unit.ld'), 'w') as f:
        f.write('\n'.join(script) + '\n')

    undefined = sorted(n for n, _, sec, _ in syms if sec == '*UND*')
    local_syms = dict(local_syms or {})
    defsyms, unknown = [], []
    for n in undefined:
        # exact spelling: external names are upper-cased (CLAUDE.md 3.15),
        # and a link against ../source is case-sensitive
        sym = linkmap.by_name.get(n)
        if n in local_syms:
            defsyms += ['--defsym', '%s=0x%X' % (n, local_syms[n])]
        elif sym is None:
            unknown.append(n)
        else:
            defsyms += ['--defsym', '%s=0x%X' % (n, sym.addr)]
    # common symbols: dpc's overlay sections (`var (name)', ref 3.14;
    # dpc-m68k emit.rs, overlay_area), resolved to the map's overlay
    # section of that name (a data section with no module segment)
    # (objdump -t gives a common symbol's size as its value): a unit may
    # declare a prefix of the section, not more than the map's section
    common_problems = []
    for n, size in sorted((n, v) for n, v, sec, _ in syms if sec == '*COM*'):
        ov = linkmap.overlay(n)
        if ov is None:
            unknown.append(n + ' (an overlay section)')
        else:
            defsyms += ['--defsym', '%s=0x%X' % (n, ov.addr)]
            if size > ov.size:
                common_problems.append('overlay section %s: ours 0x%X bytes, map section '
                                       '0x%X..0x%X is 0x%X' % (n, size, ov.addr, ov.end, ov.size))
    if unknown:
        raise SetupError('%s: external symbol(s) not in the SAU2 map: %s'
                         % (src, ', '.join(unknown)))
    # dpc writes a `bsr.s' to a routine outside a fragment as its encoding
    # (`.short 0x61dd | bsr.s NAME', dpc-m68k emit.rs): gas cannot relocate
    # an 8-bit displacement to an undefined symbol.  Such a call names its
    # routine only in the comment, and the byte comparison checks it.
    unused = sorted(set(local_syms) - set(undefined) - encoded_calls(src))
    if unused:
        raise SetupError('%s: module-local symbol(s) given but not referenced: %s'
                         % (src, ', '.join(unused)))
    elf = os.path.join(work, 'unit.elf')
    run([LD, '--no-warn-rwx-segments', '-T', 'unit.ld', '-o', 'unit.elf', 'unit.o'] + defsyms,
        cwd=work)
    linked = {}
    for n, v, sec, fl in symbols_of(elf):
        if fl[5] != 'd':
            linked.setdefault(n, v)
    base = {s: linked[m] for s, m in markers.items() if m in linked}
    for rsecs, rlo, _, rpad in regions:
        if base.get(rsecs[0]) != rlo - rpad:
            raise SetupError('code section %s linked at 0x%X, not 0x%X'
                             % (rsecs[0], base.get(rsecs[0], -1), rlo - rpad))

    # 3. bytes, region by region
    problems = list(common_problems)
    for s, (ov, size) in sorted(nobits_overlays.items()):
        if section_addrs[s] != ov.addr:
            problems.append('overlay section %s at 0x%X: not the map\'s overlay section '
                            '%s 0x%X' % (s, section_addrs[s], ov.name, ov.addr))
        elif size > ov.size:
            problems.append('overlay section %s: ours 0x%X bytes, map section 0x%X..0x%X '
                            'is 0x%X' % (s, size, ov.addr, ov.end, ov.size))
    region_bytes = []           # [(lo, hi, ours, image)]
    for k, (rsecs, rlo, rhi, rpad) in enumerate(regions):
        binpath = os.path.join(work, 'code.bin' if k == 0 else 'code%d.bin' % k)
        run([OBJCOPY, '-O', 'binary', '-j', code_out(k), elf, binpath])
        with open(binpath, 'rb') as f:
            ours = f.read()
        # code.bin keeps the fragment pad; dpc fills it with .space, so
        # anything else there is code that would otherwise escape the
        # comparison
        ours, padbytes = ours[rpad:], ours[:rpad]
        # the enclosing routine of nested routines: compiled after them, at
        # the end of the unit, and left out of every comparison
        if enclosing is not None:
            at = linked.get(enclosing)
            if at is None:
                raise SetupError('enclosing=%s: no such symbol in %s' % (enclosing, src))
            if at != rhi:
                problems.append('enclosing routine %s linked at 0x%X, not at the end of the '
                                'unit, 0x%X' % (enclosing, at, rhi))
            ours = ours[:rhi - rlo]
        if padbytes.strip(b'\0'):
            problems.append('fragment pad 0x%X..0x%X is not zero: %s'
                            % (rlo - rpad, rlo, padbytes.hex()))
        img = read_image(rlo, rhi - rlo)
        if len(ours) != len(img):
            problems.append(SIZE_PROBLEM % (len(ours), rlo, rhi, len(img)))
        for i, (x, y) in enumerate(zip(ours, img)):
            if x != y:
                problems.append('bytes differ first at 0x%X (+0x%X): ours %02x, image %02x'
                                % (rlo + i, i, x, y))
                break
        region_bytes.append((rlo, rhi, ours, img))

    # the other code sections: each compared whole at its --section-addr
    regions_of = {sec: (rlo, rhi, img) for (rsecs, _, _, _), (rlo, rhi, _, img)
                  in zip(regions, region_bytes) for sec in rsecs}
    extra_bytes = {}
    for i, sec in enumerate(extra):
        cpath = os.path.join(work, 'code_c%d.bin' % i)
        run([OBJCOPY, '-O', 'binary', '-j', '.conform_c%d' % i, elf, cpath])
        with open(cpath, 'rb') as f:
            data = f.read()
        a = section_addrs[sec]
        if a < IMAGE_LO or a + len(data) > IMAGE_HI:
            raise SetupError('--section-addr %s=0x%X: outside the image' % (sec, a))
        cimg = read_image(a, len(data))
        regions_of[sec] = (a, a + len(data), cimg)
        extra_bytes[sec] = data
        # compared whole: the section must be exactly one map segment, or
        # a section shorter than its segment would pass on a prefix
        seg = linkmap.segment_containing(a)
        if seg is None or seg.is_section or seg.kind != 'I' or seg.addr != a:
            problems.append('code section %s at 0x%X: not the start of a code segment of '
                            'the map' % (sec, a))
        elif len(data) != seg.size:
            problems.append('code section %s: ours 0x%X bytes, map segment %s 0x%X..0x%X '
                            'is 0x%X' % (sec, len(data), seg.name, seg.addr, seg.end,
                                         seg.size))
        bad = [j for j, (x, y) in enumerate(zip(data, cimg)) if x != y]
        if bad:
            problems.append('code section %s differs first at 0x%X: ours %02x, image %02x'
                            % (sec, a + bad[0], data[bad[0]], cimg[bad[0]]))

    # 4. relocations, symbolically
    relocs = relocs_of(obj)
    nrel = 0
    for sec in code + extra:
        rlo, rhi, rimg = regions_of[sec]
        for off, typ, sym, add in relocs.get(sec, []):
            p = base[sec] + off
            if enclosing is not None and p >= hi:
                continue
            nrel += 1
            size = RELOC_SIZE.get(typ)
            if size is None:
                problems.append('0x%X: relocation type %s not handled' % (p, typ))
                continue
            if sym in markers:
                s, what = base.get(sym), sym
            else:
                s, what = linked.get(sym), sym
            if s is None:
                problems.append('0x%X: %s against %s, which was not linked' % (p, typ, sym))
                continue
            want = s + add - (p if '_PC' in typ else 0)
            mask = (1 << (8 * size)) - 1
            if p < rlo or p + size > rhi:
                problems.append('0x%X: %s %s%+#x lies outside the compared range'
                                % (p, typ, what, add))
                continue
            have = int.from_bytes(rimg[p - rlo:p - rlo + size], 'big')
            if (want & mask) != have:
                if '_PC' in typ:
                    sign = 1 << (8 * size - 1)
                    tgt = p + ((have ^ sign) - sign)
                else:
                    tgt = have
                problems.append('0x%X: %s names %s%s (0x%X); image operand 0x%X is %s'
                                % (p, typ, what, '%+#x' % add if add else '', s + add,
                                   have, linkmap.describe(tgt)))
            if sym in other and sym not in section_addrs:
                problems.append('0x%X: reference into section %s, which has no '
                                '--section-addr' % (p, sym))

    # literal addresses: a 32-bit field with no relocation whose value is a
    # map address (>= 0x10000) means a reference was hard-coded instead of
    # named; it would match here but not survive a relink in ../source
    map_addrs = {s.addr for s in linkmap.symbols if s.addr >= 0x10000}
    for secs_, rlo, data in ([(r[0], rb[0], rb[2]) for r, rb in zip(regions, region_bytes)]
                             + [([s_], section_addrs[s_], extra_bytes[s_]) for s_ in extra]):
        covered = set()
        for sec in secs_:
            for off, typ, _, _ in relocs.get(sec, []):
                p = base[sec] + off - rlo
                covered.update(range(p, p + RELOC_SIZE.get(typ, 0)))
        for i in range(0, len(data) - 3, 2):
            v = int.from_bytes(data[i:i + 4], 'big')
            if v in map_addrs and not covered & set(range(i, i + 4)):
                problems.append('0x%X: literal 0x%X (%s) with no relocation; reference '
                                'it by name' % (rlo + i, v, linkmap.describe(v)))

    # non-code sections with contents (dpc's `.moddata.<UNIT>[.<SECT>]', a
    # module data block with initial values, or `.data.overlay.<NAME>', an
    # overlay section with initial values): compared byte for byte at
    # their --section-addr, which must start a data segment of the map
    # (for an overlay, the map's overlay section) that holds the whole
    # section (a unit may declare a prefix of the block only); without one
    # they cannot be checked, which is a failure
    flags = {s: fl for s, _, fl in secs}
    data_ok = []
    for i, s in enumerate(other):
        if 'CONTENTS' not in flags[s]:
            continue
        if s not in section_addrs:
            problems.append('section %s has contents but no --section-addr, so it '
                            'is not compared with the image' % s)
            continue
        dpath = os.path.join(work, 'data%d.bin' % i)
        run([OBJCOPY, '-O', 'binary', '-j', '.conform_d%d' % i, elf, dpath])
        with open(dpath, 'rb') as f:
            data = f.read()
        a = section_addrs[s]
        if a < IMAGE_LO or a + len(data) > IMAGE_HI:
            raise SetupError('--section-addr %s=0x%X: outside the image' % (s, a))
        seg = linkmap.segment_containing(a)
        # dpc's initialised overlay section (`.data.overlay.<NAME>'): the
        # map's overlay section NAME, which has no module segment
        if s.startswith('.data.overlay.'):
            seg = linkmap.overlay(s[len('.data.overlay.'):])
        elif seg is not None and seg.is_section:
            seg = None
        if seg is None or seg.kind != 'D' or seg.addr != a:
            problems.append('data section %s at 0x%X: not the start of a data segment of '
                            'the map' % (s, a))
        elif len(data) > seg.size:
            problems.append('data section %s: ours 0x%X bytes, map segment %s 0x%X..0x%X '
                            'is 0x%X' % (s, len(data), seg.name, seg.addr, seg.end, seg.size))
        dimg = read_image(a, len(data))
        bad = [j for j, (x, y) in enumerate(zip(data, dimg)) if x != y]
        if bad:
            problems.append('section %s differs first at 0x%X: ours %02x, image %02x'
                            % (s, a + bad[0], data[bad[0]], dimg[bad[0]]))
        data_ok.append((s, a, len(data)))

    # placement of named routines and other global code symbols: a map name
    # must be global and at its map address; a global the map does not name
    # would be a module-local routine exported by mistake (CLAUDE.md 3.15)
    globals_ = {n for n, _, sec, fl in syms
                if fl[0] == 'g' and (sec in code or sec in extra)}
    for r in routines:
        if r.name and r.name not in globals_:
            problems.append('routine %s is not defined as a global code symbol in %s'
                            % (r.name, src))
    # `global=NAME': a map name that is not a routine of its own (the
    # second name of one, M$OIU$LLL = M$OIS$LLL at 0xE0AD7C) must be
    # defined too; its address is checked with the other globals below
    for n in required_globals or ():
        if n not in linkmap.by_name:
            raise SetupError('global=%s: not a map name' % n)
        if n not in globals_:
            problems.append('symbol %s (global=) is not defined as a global code symbol in %s'
                            % (n, src))
    for n in sorted(globals_):
        sym = linkmap.by_name.get(n)
        if sym is None:
            problems.append('global symbol %s is not in the map' % n)
        elif linked.get(n) != sym.addr:
            problems.append('symbol %s linked at 0x%X, map has 0x%X' % (n, linked[n], sym.addr))

    if not problems:
        if not quiet:
            for r in routines:
                out('OK %s 0x%X 0x%X  %s' % (r.label, r.addr, r.length, name))
            for sec in extra:
                out('OK section %s 0x%X 0x%X  %s'
                    % (sec, section_addrs[sec], len(extra_bytes[sec]), name))
            for sec, a, n in data_ok:
                out('OK data %s 0x%X 0x%X  %s' % (sec, a, n, name))
        return True

    out('FAIL %s (%s, %s, %d relocation(s))'
        % (name, ' '.join(r.label for r in routines),
           ' '.join('0x%X..0x%X' % (rlo, rhi) for rlo, rhi, _, _ in region_bytes), nrel))
    for p in problems:
        out('  ' + p)
    for k, (rlo, rhi, ours, _) in enumerate(region_bytes):
        if k:
            out('code at 0x%X:' % rlo)
        n = max(len(ours), rhi - rlo)
        img_show = read_image(rlo, min(n, IMAGE_HI - rlo))
        out('%s%-44s | image' % (' ' * 10, 'ours'))
        tag = '' if k == 0 else str(k)
        for line in side_by_side(disassemble(ours, rlo, work, 'ours' + tag),
                                 disassemble(img_show, rlo, work, 'image' + tag)):
            out(line)
    for sec in extra:
        a, _, cimg = regions_of[sec]
        out('section %s:' % sec)
        for line in side_by_side(disassemble(extra_bytes[sec], a, work, 'ours_' + sec),
                                 disassemble(cimg, a, work, 'image_' + sec)):
            out(line)
    return False


# ---- manifest ------------------------------------------------------------

def fragment_args(routines, linkmap):
    """The dpc fragment options for a .pas unit made of `routines': none
    for a whole map module (which dpc compiles as a module, module-end
    padding included), `--fragment-origin LO', plus
    `--fragment-ends-module' when the unit is the tail of its module (the
    module-end padding is then part of its last routine in the image).
    A unit with routines in several map segments (one module in several
    code sections, unit_groups) is a fragment of each: per run of routines
    `--fragment-origin=SECTION=0xLO', SECTION being the map's output
    section (dpc matches it with the Pascal section name, any case), and
    `--fragment-ends-module=SECTION' for each run that ends its segment."""
    groups = unit_groups(routines, linkmap)
    if len(groups) > 1:
        args = []
        for g in groups:
            lo, hi = g[0].addr, g[-1].end
            seg = linkmap.segment_containing(lo)
            if hi > seg.end:
                raise SetupError('the unit 0x%X..0x%X runs past the end of module %s '
                                 '(0x%X..0x%X); one .pas unit is one module'
                                 % (lo, hi, seg.name, seg.addr, seg.end))
            sect = code_section_of(lo, linkmap)
            args.append('--fragment-origin=%s=0x%X' % (sect, lo))
            if hi == seg.end:
                args.append('--fragment-ends-module=%s' % sect)
        return args
    lo, hi = routines[0].addr, routines[-1].end
    seg = linkmap.segment_containing(lo)
    if seg is None or seg.is_section:
        return ['--fragment-origin', '0x%X' % lo]
    if hi > seg.end:
        raise SetupError('the unit 0x%X..0x%X runs past the end of module %s (0x%X..0x%X); '
                         'one .pas unit is one module'
                         % (lo, hi, seg.name, seg.addr, seg.end))
    if seg.addr == lo and seg.end == hi:
        return []
    args = ['--fragment-origin', '0x%X' % lo]
    if hi == seg.end:
        args.append('--fragment-ends-module')
    return args


def compile_pas(src, dpc, work, fragment=()):
    if not os.path.exists(dpc):
        raise SetupError('%s not found (run `make build`)' % dpc)
    stem = os.path.splitext(os.path.basename(src))[0]
    out = os.path.join(work, stem + '.s')
    # --strict: conformance never takes a best-effort path (bead
    # domain-pascal-5kgg).
    run([dpc, '--strict', '-o', out] + list(fragment) + [src])
    return out


LOOP_ALIGN_TOKENS = ('loop-align=', 'loop-align-off-from=')


def loop_align_args(tokens):
    """The dpc options for manifest tokens `loop-align=on|off' and
    `loop-align-off-from=NAME,...' (one routine per code section; at most
    one of the tokens)."""
    if len(tokens) > 1:
        raise SetupError('more than one of loop-align= / loop-align-off-from=: %s'
                         % ' '.join(tokens))
    out = []
    for t in tokens:
        key, _, val = t.partition('=')
        if key == 'loop-align' and val not in ('on', 'off'):
            raise SetupError('%s: want loop-align=on or loop-align=off' % t)
        if not val or '' in val.split(','):
            raise SetupError('%s: needs a routine name' % t)
        out.append('--' + t)
    return out


A5_TOKENS = ('a5-always-from=', 'a5-always=', 'a5-absolute=')


def a5_args(tokens):
    """The dpc options for manifest tokens `a5-always-from=NAME,...' (one
    routine per code section), `a5-always=NAME,...' and
    `a5-absolute=NAME,...' (each at most once)."""
    out, seen = [], set()
    for t in tokens:
        key, _, val = t.partition('=')
        if key in seen:
            raise SetupError('%s= given more than once' % key)
        seen.add(key)
        if not val or '' in val.split(','):
            raise SetupError('%s: needs a routine name list' % t)
        out.append('--' + t)
    return out


_ENCODED_BSR = re.compile(r'^\s*\.short\s+0x61[0-9a-fA-F]{2}\s*\|\s*bsr\.s\s+(\S+)\s*$')


def encoded_calls(src):
    """The routines that `src' (assembly) calls with a `bsr.s' written as
    its encoding, `.short 0x61dd | bsr.s NAME'.  An object file has no
    such text: none."""
    if src.endswith('.o'):
        return set()
    with open(src) as f:
        return {m.group(1) for m in map(_ENCODED_BSR.match, f) if m}


MERGE_TOKEN = 'merge-constant-stores'
MODULE_ROUTINES_TOKEN = 'module-routines='
ENCLOSING_TOKEN = 'enclosing='
GLOBAL_TOKEN = 'global='


def module_routine_args(tokens, routines, linkmap):
    """The dpc options for the manifest token `module-routines=ITEM,...'
    (at most once), and the local symbols to resolve at link time:
    (options, {symbol: address}).  An ITEM `NAME' is a map routine;
    `NAME=0xADDR' a module-local routine the map does not name, NAME being
    dpc's symbol for the unit's `internal; extern' declaration of it
    (test-only, bead domain-pascal-3h9; README.md, "Routines outside the
    unit").  Each must lie in the unit's map module and outside the unit;
    an address must name no map symbol (use the map name) and NAME must
    not be a map name.  Each is forwarded as
    `--module-routine=NAME=0xADDR', or, when it lies in another code
    segment of the module than the unit or the unit has routines in
    several (check_in_module), `--module-routine=SECTION:NAME=0xADDR' with
    the map's output section: dpc then calls it with `bsr' from the
    unit's code in that section and with `jsr abs.l' from any other."""
    if len(tokens) > 1:
        raise SetupError('module-routines= given more than once')
    out, local, seen = [], {}, set()
    for t in tokens:
        items = t[len(MODULE_ROUTINES_TOKEN):].split(',')
        if '' in items:
            raise SetupError('%s: needs a routine name list' % t)
        for item in items:
            n, eq, a = item.partition('=')
            if eq:
                if not n or not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_$]*', n):
                    raise SetupError('%s: %r: want NAME=0xADDR' % (t, item))
                try:
                    addr = parse_hex(a)
                except ValueError:
                    raise SetupError('%s: %r: want NAME=0xADDR' % (t, item))
                if not a.lower().startswith('0x'):
                    raise SetupError('%s: %r: want NAME=0xADDR' % (t, item))
                sect = check_local_sym(n, addr, routines, linkmap, t)
                # dpc's symbols are upper-cased (CLAUDE.md 3.15)
                n = n.upper()
                if n in local:
                    raise SetupError('%s: %s given more than once' % (t, n))
                local[n] = addr
            else:
                sym = linkmap.lookup(n)
                if sym is None:
                    raise SetupError('%s: routine %s is not in the map' % (t, n))
                n, addr = sym.name, sym.addr
                sect = check_in_module(n, addr, routines, linkmap, t)
            if n in seen:
                raise SetupError('%s: %s given more than once' % (t, n))
            seen.add(n)
            out.append('--module-routine=%s%s=0x%X' % (sect + ':' if sect else '', n, addr))
    return out, local


def check_in_module(n, addr, routines, linkmap, what):
    """Routine `n' at `addr' must lie in the map module of the unit made
    of `routines', outside the unit: in one of the unit's map segments, or
    in another code segment of the same name (the module's code in another
    output section: TTY in WIRED_PROC, OS_INIT_PROC and .TEXT), but not in
    another segment of an output section the unit has routines in (that
    would be another compilation unit).  `what' prefixes the error.
    Returns the routine's code section for `--module-routine=SECTION:...':
    None when the unit is one run of routines in the routine's own
    segment (dpc's default, the unit's only code section), else the map's
    output section."""
    groups = unit_groups(routines, linkmap)
    segs = [linkmap.segment_containing(g[0].addr) for g in groups]
    there = linkmap.segment_containing(addr)
    if there is None or segs[0] is None or (
            there not in segs and (there.is_section or segs[0].is_section
                                   or there.kind != 'I' or there.name != segs[0].name)):
        raise SetupError('%s: %s is not in the module of the unit' % (what, n))
    if any(r.addr <= addr < r.end for r in routines):
        raise SetupError('%s: %s is part of the unit' % (what, n))
    if there in segs:
        return None if len(segs) == 1 else code_section_of(addr, linkmap)
    sect = code_section_of(addr, linkmap)
    if sect in [code_section_of(sg.addr, linkmap) for sg in segs]:
        raise SetupError('%s: %s is in another segment of output section %s than the unit'
                         % (what, n, sect))
    return sect


def check_local_sym(n, addr, routines, linkmap, what):
    """A module-local routine the map does not name, `n' at `addr'
    (`module-routines=NAME=0xADDR', `check --local-sym NAME=0xADDR'):
    `n' is not a map name, the map names nothing at `addr', which lies in
    the unit's map module outside the unit (bead domain-pascal-3h9;
    README.md, "Routines outside the unit").  Returns check_in_module's
    code section."""
    if linkmap.lookup(n) is not None:
        raise SetupError('%s: %s is a map name; give it without an address' % (what, n))
    named = [x for x in linkmap.names_at(addr)]
    if named:
        raise SetupError('%s: 0x%X is %s in the map; give that name'
                         % (what, addr, named[0]))
    return check_in_module(n, addr, routines, linkmap, what)


class Unit:
    """A manifest line: the unit's file (`file' as written, `src' its
    path), its routine specs, placed sections, local symbols, enclosing
    routine and required globals, and for a .pas unit the dpc options it
    is compiled with besides `--strict' (`dpc_opts'; None for an
    assembly or object unit)."""

    def __init__(self, file):
        self.file = file
        self.src = os.path.join(REPO, file)
        self.specs, self.sect, self.globs = [], {}, []
        self.dpc_opts, self.local, self.enclosing = None, {}, None


def manifest_lines(path):
    """The non-comment lines of manifest `path': [(line number, tokens)]."""
    with open(path) as f:
        lines = [(i, l.split('#', 1)[0].split()) for i, l in enumerate(f, 1)]
    return [(i, toks) for i, toks in lines if toks]


def manifest_unit(toks, linkmap):
    """The Unit of a manifest line's tokens (run_manifest; SetupError for
    a bad line)."""
    u = Unit(toks[0])
    sect, align, a5, mods, encl, merge = {}, [], [], [], [], []
    rest = iter(toks[1:])
    for t in rest:
        if t == '--section-addr':
            k, v = parse_section_addr(next(rest, ''))
            sect[k] = v
        elif t.startswith(LOOP_ALIGN_TOKENS):
            align.append(t)
        elif t.startswith(A5_TOKENS):
            a5.append(t)
        elif t == MERGE_TOKEN:
            merge.append(t)
        elif t.startswith(MODULE_ROUTINES_TOKEN):
            mods.append(t)
        elif t.startswith(ENCLOSING_TOKEN):
            encl.append(t[len(ENCLOSING_TOKEN):])
        elif t.startswith(GLOBAL_TOKEN):
            u.globs += t[len(GLOBAL_TOKEN):].split(',')
        else:
            u.specs.append(t)
    u.sect = sect
    pas = u.src.endswith('.pas')
    opts = loop_align_args(align)
    if opts and not pas:
        raise SetupError('%s: loop-align options apply to .pas units only' % u.file)
    a5opts = a5_args(a5)
    if a5opts and not pas:
        raise SetupError('%s: a5 options apply to .pas units only' % u.file)
    opts += a5opts
    if len(merge) > 1:
        raise SetupError('%s: %s given more than once' % (u.file, MERGE_TOKEN))
    if merge and not pas:
        raise SetupError('%s: %s applies to .pas units only' % (u.file, MERGE_TOKEN))
    opts += ['--' + MERGE_TOKEN] if merge else []
    if mods and not pas:
        raise SetupError('%s: module-routines applies to .pas units only' % u.file)
    if len(encl) > 1 or encl and not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_$]*', encl[0]):
        raise SetupError('%s: want one enclosing=NAME' % u.file)
    if encl and not pas:
        raise SetupError('%s: enclosing applies to .pas units only' % u.file)
    # dpc's symbols are upper-cased (CLAUDE.md 3.15)
    u.enclosing = encl[0].upper() if encl else None
    if pas:
        routines = unit_routines(u.specs, linkmap)
        frag = fragment_args(routines, linkmap)
        mopts, u.local = module_routine_args(mods, routines, linkmap)
        if mopts and not frag:
            raise SetupError('%s: module-routines applies to fragments only' % u.file)
        if u.enclosing and not frag:
            raise SetupError('%s: enclosing applies to fragments only' % u.file)
        u.dpc_opts = frag + opts + mopts
    return u


def run_manifest(path, linkmap, dpc):
    """Each non-comment line: FILE SPEC... [--section-addr SEC=0xADDR]...
    [loop-align=on|off | loop-align-off-from=NAME] [a5-...=NAME,...]...
    [merge-constant-stores]
    FILE is relative to the repository root; .pas files are compiled with
    dpc first, with the loop-align, a5 and merge-constant-stores tokens
    forwarded as dpc options.  Returns (failing
    entries, entries with setup errors)."""
    failures = errors = 0
    for lineno, toks in manifest_lines(path):
        try:
            u = manifest_unit(toks, linkmap)
            work = scratch_dir(u.src)
            asm = u.src
            if u.dpc_opts is not None:
                asm = compile_pas(u.src, dpc, work, u.dpc_opts)
            ok = check(asm, u.specs, linkmap, u.sect, work=work, name=u.file,
                       local_syms=u.local, enclosing=u.enclosing, required_globals=u.globs)
        except SetupError as e:
            print('ERROR %s:%d: %s' % (path, lineno, e))
            errors += 1
            continue
        failures += not ok
    return failures, errors


def parse_section_addr(s, what='--section-addr wants SEC=0xADDR'):
    name, eq, addr = s.partition('=')
    if not eq or not name:
        raise SetupError('%s, got %r' % (what, s))
    return name, parse_hex(addr)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    sub = ap.add_subparsers(dest='cmd', required=True)
    c = sub.add_parser('check', help='check one assembly file')
    c.add_argument('file')
    c.add_argument('--routine', '--addr', action='append', default=[], dest='specs',
                   metavar='SPEC', help='NAME[:LEN] or 0xADDR[:LEN]')
    c.add_argument('--len', type=parse_hex, help='length override (one routine)')
    c.add_argument('--section-addr', action='append', default=[], metavar='SEC=0xADDR',
                   help='place a non-code section')
    c.add_argument('--local-sym', action='append', default=[], metavar='NAME=0xADDR',
                   help='resolve module-local symbol NAME (not in the map) to ADDR')
    c.add_argument('--enclosing', default=None, metavar='NAME',
                   help="the routine after the unit that its nested routines belong to; "
                        "compiled, not compared")
    c.add_argument('--map', default=None)
    m = sub.add_parser('manifest', help='check every entry of a manifest')
    m.add_argument('file')
    m.add_argument('--dpc', default=os.path.join(REPO, 'target', 'debug', 'dpc'))
    m.add_argument('--map', default=None)
    args = ap.parse_args(argv)

    miss = missing_prereqs()
    if miss:
        print('conform: missing: %s' % ', '.join(miss), file=sys.stderr)
        return 2
    linkmap = sau2map.load(args.map)
    try:
        if args.cmd == 'check':
            sect = dict(parse_section_addr(s) for s in args.section_addr)
            local = {}
            if args.local_sym:
                # the checks of `module-routines=NAME=0xADDR' (bead
                # domain-pascal-500)
                routines = unit_routines(args.specs, linkmap, args.len)
                for s in args.local_sym:
                    n, addr = parse_section_addr(s, '--local-sym wants NAME=0xADDR')
                    if not re.fullmatch(r'[A-Za-z_][A-Za-z0-9_$]*', n):
                        raise SetupError('--local-sym %s: want NAME=0xADDR' % s)
                    if n in local:
                        raise SetupError('--local-sym %s: %s given more than once' % (s, n))
                    check_local_sym(n, addr, routines, linkmap, '--local-sym ' + s)
                    local[n] = addr
            return 0 if check(args.file, args.specs, linkmap, sect, args.len,
                              local_syms=local, enclosing=args.enclosing) else 1
        bad, errors = run_manifest(args.file, linkmap, args.dpc)
        return 2 if errors else 1 if bad else 0
    except SetupError as e:
        print('conform: %s' % e, file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
