#!/usr/bin/env python3
"""conform.py - image-conformance gate (CLAUDE.md section 6, item 3).

Assemble a unit, link its code at the image address of the routine(s) it
reproduces, and compare the bytes with the SR10.2 SAU2 kernel image.

    conform.py check FILE.s --routine NAME [--routine NAME ...]
    conform.py check FILE.s --addr 0xADDR[:LEN] --routine NAME
    conform.py check FILE.s ... --local-sym NAME=0xADDR
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
switch SEG' shows a segment's modes), or `module-routines=NAME,...' (.pas
fragments only, at most once): routines of the unit's map module outside
the unit, which the unit declares `extern' and the original reached with
`bsr'; each is forwarded as `--module-routine=NAME=0xADDR' with its map
address (codegen-patterns 1.6, 7.4: DISK_$UNASSIGN_ALL 0xE6BE34).  An
item `NAME=0xADDR' names a module-local routine the map does not name
(test-only, bead domain-pascal-3h9): NAME is the symbol dpc gives the
routine the unit declares `internal; extern' (its Pascal name upper-cased,
a local symbol), and ADDR must lie in the unit's map module, outside the
unit, with no map name; it is forwarded the same way, and the reference is
resolved with `--defsym NAME=0xADDR' at link time (`check --local-sym').

A routine SPEC is `NAME', `NAME:LEN', `0xADDR' or `0xADDR:LEN' (LEN is hex,
`0x' optional).  NAME is looked up in the SAU2 map; an address names a
module-local routine the map does not list ("module-local at 0xADDR").
Without LEN, the length runs to the next map symbol or the end of the map
segment, whichever comes first.  `--len LEN' overrides the length when
exactly one routine is given.

The routines of one unit must be contiguous in the image (e.g. a
module-local worker, then its caller).  All code sections of the object are
concatenated in object order and linked at the lowest routine address.

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

Steps:
  1. m68k-elf-as -m68010.
  2. Every undefined symbol is resolved from the map (`--defsym', exact
     spelling), or from `--local-sym NAME=0xADDR' (a module-local routine
     outside the unit, which the map does not name); any other symbol is
     an error, and so is a `--local-sym' the object does not reference.  Non-code
     sections are placed with `--section-addr SEC=0xADDR', or else at an
     address outside the image (references to them then fail step 4).
  3. The linked code is compared byte for byte with the image, and so is
     every non-code section with contents; such a section without
     `--section-addr' is a failure.  A 32-bit field without a relocation
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
    """The routines of a unit, sorted and checked to be contiguous."""
    routines = sorted((parse_spec(s, linkmap) for s in specs), key=lambda r: r.addr)
    if not routines:
        raise SetupError('no routine given')
    if length is not None:
        if len(routines) != 1:
            raise SetupError('--len needs exactly one routine')
        routines[0].length = length
    for r0, r1 in zip(routines, routines[1:]):
        if r0.end != r1.addr:
            raise SetupError('routines are not contiguous: %s ends at 0x%X, %s starts at 0x%X'
                             % (r0.label, r0.end, r1.label, r1.addr))
    return routines


def check(src, specs, linkmap, section_addrs=None, length=None, quiet=False, out=print,
          work=None, name=None, local_syms=None):
    """Check one assembly unit.  Returns True on a match.  Raises SetupError
    on bad input.  `work' is the scratch directory (default: from `src'),
    `name' the unit's name in the result lines (default: `src').
    `local_syms' maps the symbols of module-local routines outside the
    unit, which the map does not name, to their image addresses
    (`module-routines=NAME=0xADDR')."""
    routines = unit_routines(specs, linkmap, length)
    lo, hi = routines[0].addr, routines[-1].end
    section_addrs = dict(section_addrs or {})
    name = name or display_path(src)

    work = work or scratch_dir(src)
    obj = os.path.join(work, 'unit.o')
    run([AS, '-m68010', '-o', obj, os.path.abspath(src)])

    # a fragment compiled with dpc --fragment-origin: `pad' leading bytes
    # put section-relative alignment on absolute boundaries; they are linked
    # below lo and not compared
    origin = [v for n, v, sec, _ in symbols_of(obj) if n == FRAGMENT_ORIGIN and sec == '*ABS*']
    pad = 0
    if origin:
        if origin[0] != lo:
            raise SetupError('%s was compiled as a fragment at 0x%X, but the unit starts at 0x%X'
                             % (src, origin[0], lo))
        pad = lo % 4

    # 2. place sections, resolve externals from the map
    secs = sections_of(obj)
    code = [s for s, n, fl in secs if 'CODE' in fl and n]
    other = [s for s, n, fl in secs if 'CODE' not in fl and 'ALLOC' in fl and n]
    if not code:
        raise SetupError('%s has no code' % src)
    for s in section_addrs:
        if s not in other:
            raise SetupError('--section-addr %s: no such non-code section with contents' % s)
    markers = {}                # input section -> marker symbol
    script = ['SECTIONS {', '  %s 0x%X : SUBALIGN(2) {' % (CODE_OUT, lo - pad)]
    for i, s in enumerate(code):
        markers[s] = '__conform_%d' % i
        script.append('    %s = .; unit.o(%s)' % (markers[s], s))
    script.append('  }')
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

    undefined = sorted(n for n, _, sec, _ in symbols_of(obj) if sec == '*UND*')
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
    if base.get(code[0]) != lo - pad:
        raise SetupError('first code section linked at 0x%X, not 0x%X'
                         % (base.get(code[0], -1), lo - pad))

    binpath = os.path.join(work, 'code.bin')
    run([OBJCOPY, '-O', 'binary', '-j', CODE_OUT, elf, binpath])
    with open(binpath, 'rb') as f:
        ours = f.read()
    # code.bin keeps the fragment pad; dpc fills it with .space, so anything
    # else there is code that would otherwise escape the comparison
    ours, padbytes = ours[pad:], ours[:pad]

    # 3. bytes
    problems = []
    if padbytes.strip(b'\0'):
        problems.append('fragment pad 0x%X..0x%X is not zero: %s'
                        % (lo - pad, lo, padbytes.hex()))
    img = read_image(lo, hi - lo)
    if len(ours) != len(img):
        problems.append('size: ours 0x%X bytes, image range 0x%X..0x%X is 0x%X'
                        % (len(ours), lo, hi, len(img)))
    for i, (x, y) in enumerate(zip(ours, img)):
        if x != y:
            problems.append('bytes differ first at 0x%X (+0x%X): ours %02x, image %02x'
                            % (lo + i, i, x, y))
            break

    # 4. relocations, symbolically
    relocs = relocs_of(obj)
    nrel = 0
    for sec in code:
        for off, typ, sym, add in relocs.get(sec, []):
            nrel += 1
            p = base[sec] + off
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
            if p < lo or p + size > hi:
                problems.append('0x%X: %s %s%+#x lies outside the compared range'
                                % (p, typ, what, add))
                continue
            have = int.from_bytes(img[p - lo:p - lo + size], 'big')
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
    covered = set()
    for sec in code:
        for off, typ, _, _ in relocs.get(sec, []):
            p = base[sec] + off - lo
            covered.update(range(p, p + RELOC_SIZE.get(typ, 0)))
    map_addrs = {s.addr for s in linkmap.symbols if s.addr >= 0x10000}
    for i in range(0, len(ours) - 3, 2):
        v = int.from_bytes(ours[i:i + 4], 'big')
        if v in map_addrs and not covered & set(range(i, i + 4)):
            problems.append('0x%X: literal 0x%X (%s) with no relocation; reference '
                            'it by name' % (lo + i, v, linkmap.describe(v)))

    # non-code sections with contents: compared byte for byte at their
    # --section-addr; without one they cannot be checked, which is a failure
    flags = {s: fl for s, _, fl in secs}
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
        dimg = read_image(a, len(data))
        bad = [j for j, (x, y) in enumerate(zip(data, dimg)) if x != y]
        if bad:
            problems.append('section %s differs first at 0x%X: ours %02x, image %02x'
                            % (s, a + bad[0], data[bad[0]], dimg[bad[0]]))

    # placement of named routines and other global code symbols: a map name
    # must be global and at its map address; a global the map does not name
    # would be a module-local routine exported by mistake (CLAUDE.md 3.15)
    globals_ = {n for n, _, sec, fl in symbols_of(obj) if fl[0] == 'g' and sec in code}
    for r in routines:
        if r.name and r.name not in globals_:
            problems.append('routine %s is not defined as a global code symbol in %s'
                            % (r.name, src))
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
        return True

    out('FAIL %s (%s, 0x%X..0x%X, %d relocation(s))'
        % (name, ' '.join(r.label for r in routines), lo, hi, nrel))
    for p in problems:
        out('  ' + p)
    n = max(len(ours), hi - lo)
    img_show = read_image(lo, min(n, IMAGE_HI - lo))
    out('%s%-44s | image' % (' ' * 10, 'ours'))
    for line in side_by_side(disassemble(ours, lo, work, 'ours'),
                             disassemble(img_show, lo, work, 'image')):
        out(line)
    return False


# ---- manifest ------------------------------------------------------------

def fragment_args(routines, linkmap):
    """The dpc fragment options for a .pas unit made of `routines': none
    for a whole map module (which dpc compiles as a module, module-end
    padding included), `--fragment-origin LO', plus
    `--fragment-ends-module' when the unit is the tail of its module (the
    module-end padding is then part of its last routine in the image)."""
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
    run([dpc, '-o', out] + list(fragment) + [src])
    return out


LOOP_ALIGN_TOKENS = ('loop-align=', 'loop-align-off-from=')


def loop_align_args(tokens):
    """The dpc options for manifest tokens `loop-align=on|off' and
    `loop-align-off-from=NAME' (at most one of them)."""
    if len(tokens) > 1:
        raise SetupError('more than one of loop-align= / loop-align-off-from=: %s'
                         % ' '.join(tokens))
    out = []
    for t in tokens:
        key, _, val = t.partition('=')
        if key == 'loop-align' and val not in ('on', 'off'):
            raise SetupError('%s: want loop-align=on or loop-align=off' % t)
        if not val:
            raise SetupError('%s: needs a routine name' % t)
        out.append('--' + t)
    return out


A5_TOKENS = ('a5-always-from=', 'a5-always=', 'a5-absolute=')


def a5_args(tokens):
    """The dpc options for manifest tokens `a5-always-from=NAME',
    `a5-always=NAME,...' and `a5-absolute=NAME,...' (each at most
    once)."""
    out, seen = [], set()
    for t in tokens:
        key, _, val = t.partition('=')
        if key in seen:
            raise SetupError('%s= given more than once' % key)
        seen.add(key)
        if not val or '' in val.split(','):
            raise SetupError('%s: needs a routine name%s'
                             % (t, '' if key == 'a5-always-from' else ' list'))
        if key == 'a5-always-from' and ',' in val:
            raise SetupError('%s: names one routine' % t)
        out.append('--' + t)
    return out


_ENCODED_BSR = re.compile(r'^\s*\.short\s+0x61[0-9a-fA-F]{2}\s*\|\s*bsr\.s\s+(\S+)\s*$')


def encoded_calls(src):
    """The routines that `src' (assembly) calls with a `bsr.s' written as
    its encoding, `.short 0x61dd | bsr.s NAME'."""
    with open(src) as f:
        return {m.group(1) for m in map(_ENCODED_BSR.match, f) if m}


MODULE_ROUTINES_TOKEN = 'module-routines='


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
    `--module-routine=NAME=0xADDR'."""
    if len(tokens) > 1:
        raise SetupError('module-routines= given more than once')
    out, local = [], {}
    for t in tokens:
        items = t[len(MODULE_ROUTINES_TOKEN):].split(',')
        if '' in items:
            raise SetupError('%s: needs a routine name list' % t)
        seg = linkmap.segment_containing(routines[0].addr)
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
                if linkmap.lookup(n) is not None:
                    raise SetupError('%s: %s is a map name; give it without an address' % (t, n))
                named = [x for x in linkmap.names_at(addr)]
                if named:
                    raise SetupError('%s: 0x%X is %s in the map; give that name'
                                     % (t, addr, named[0]))
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
            if seg is None or linkmap.segment_containing(addr) is not seg:
                raise SetupError('%s: %s is not in the module of the unit' % (t, n))
            if any(r.addr <= addr < r.end for r in routines):
                raise SetupError('%s: %s is part of the unit' % (t, n))
            out.append('--module-routine=%s=0x%X' % (n, addr))
    return out, local


def run_manifest(path, linkmap, dpc):
    """Each non-comment line: FILE SPEC... [--section-addr SEC=0xADDR]...
    [loop-align=on|off | loop-align-off-from=NAME] [a5-...=NAME,...]...
    FILE is relative to the repository root; .pas files are compiled with
    dpc first, with the loop-align and a5 tokens forwarded as dpc
    options.  Returns (failing
    entries, entries with setup errors)."""
    failures = errors = 0
    with open(path) as f:
        lines = [(i, l.split('#', 1)[0].split()) for i, l in enumerate(f, 1)]
    for lineno, toks in lines:
        if not toks:
            continue
        src = os.path.join(REPO, toks[0])
        specs, sect, align, a5, mods = [], {}, [], [], []
        rest = iter(toks[1:])
        for t in rest:
            if t == '--section-addr':
                k, v = parse_section_addr(next(rest, ''))
                sect[k] = v
            elif t.startswith(LOOP_ALIGN_TOKENS):
                align.append(t)
            elif t.startswith(A5_TOKENS):
                a5.append(t)
            elif t.startswith(MODULE_ROUTINES_TOKEN):
                mods.append(t)
            else:
                specs.append(t)
        try:
            work = scratch_dir(src)
            asm = src
            opts = loop_align_args(align)
            if opts and not src.endswith('.pas'):
                raise SetupError('%s: loop-align options apply to .pas units only' % toks[0])
            a5opts = a5_args(a5)
            if a5opts and not src.endswith('.pas'):
                raise SetupError('%s: a5 options apply to .pas units only' % toks[0])
            opts += a5opts
            if mods and not src.endswith('.pas'):
                raise SetupError('%s: module-routines applies to .pas units only' % toks[0])
            if src.endswith('.pas'):
                routines = unit_routines(specs, linkmap)
                frag = fragment_args(routines, linkmap)
                mopts, local = module_routine_args(mods, routines, linkmap)
                if mopts and not frag:
                    raise SetupError('%s: module-routines applies to fragments only' % toks[0])
                asm = compile_pas(src, dpc, work, frag + opts + mopts)
            else:
                local = {}
            ok = check(asm, specs, linkmap, sect, work=work, name=toks[0], local_syms=local)
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
            local = dict(parse_section_addr(s, '--local-sym wants NAME=0xADDR')
                         for s in args.local_sym)
            return 0 if check(args.file, args.specs, linkmap, sect, args.len,
                              local_syms=local) else 1
        bad, errors = run_manifest(args.file, linkmap, args.dpc)
        return 2 if errors else 1 if bad else 0
    except SetupError as e:
        print('conform: %s' % e, file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
