#!/usr/bin/env python3
"""conform.py - image-conformance gate (CLAUDE.md section 6, item 3).

Assemble a unit, link its code at the image address of the routine(s) it
reproduces, and compare the bytes with the SR10.2 SAU2 kernel image.

    conform.py check FILE.s --routine NAME [--routine NAME ...]
    conform.py check FILE.s --addr 0xADDR[:LEN] --routine NAME
    conform.py manifest tests/conformance/manifest.txt [--dpc target/debug/dpc]

A routine SPEC is `NAME', `NAME:LEN', `0xADDR' or `0xADDR:LEN' (LEN is hex,
`0x' optional).  NAME is looked up in the SAU2 map; an address names a
module-local routine the map does not list ("module-local at 0xADDR").
Without LEN, the length runs to the next map symbol or the end of the map
segment, whichever comes first.  `--len LEN' overrides the length when
exactly one routine is given.

The routines of one unit must be contiguous in the image (e.g. a
module-local worker, then its caller).  All code sections of the object are
concatenated in object order and linked at the lowest routine address.

Steps:
  1. m68k-elf-as -m68010.
  2. Every undefined symbol is resolved from the map (`--defsym', exact
     spelling); a symbol the map does not name is an error.  Non-code
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
  5. Result: one line `OK NAME 0xVA LEN' per routine (exit 0), or the
     problems and a side-by-side disassembly, ours | image, with differing
     lines marked `*' (exit 1).  Setup errors exit 2.

Relocation matching follows the idea of ../source/domain_os/tools/
asm_compare.py (step 3 there: R_68K_32/16/8 must equal S + A, R_68K_PC*
S + A - P); no code is shared with it.

Python 3 standard library only.  Scratch files go to tmp/conform/.
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

def check(src, specs, linkmap, section_addrs=None, length=None, quiet=False, out=print):
    """Check one assembly unit.  Returns True on a match.  Raises SetupError
    on bad input."""
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
    lo, hi = routines[0].addr, routines[-1].end
    section_addrs = dict(section_addrs or {})

    stem = os.path.splitext(os.path.basename(src))[0]
    work = os.path.join(SCRATCH, stem)
    os.makedirs(work, exist_ok=True)
    obj = os.path.join(work, 'unit.o')
    run([AS, '-m68010', '-o', obj, os.path.abspath(src)])

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
    script = ['SECTIONS {', '  %s 0x%X : SUBALIGN(2) {' % (CODE_OUT, lo)]
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
    defsyms, unknown = [], []
    for n in undefined:
        # exact spelling: external names are upper-cased (CLAUDE.md 3.15),
        # and a link against ../source is case-sensitive
        sym = linkmap.by_name.get(n)
        if sym is None:
            unknown.append(n)
        else:
            defsyms += ['--defsym', '%s=0x%X' % (n, sym.addr)]
    if unknown:
        raise SetupError('%s: external symbol(s) not in the SAU2 map: %s'
                         % (src, ', '.join(unknown)))
    elf = os.path.join(work, 'unit.elf')
    run([LD, '--no-warn-rwx-segments', '-T', 'unit.ld', '-o', 'unit.elf', 'unit.o'] + defsyms,
        cwd=work)
    linked = {}
    for n, v, sec, fl in symbols_of(elf):
        if fl[5] != 'd':
            linked.setdefault(n, v)
    base = {s: linked[m] for s, m in markers.items() if m in linked}
    if base.get(code[0]) != lo:
        raise SetupError('first code section linked at 0x%X, not 0x%X'
                         % (base.get(code[0], -1), lo))

    binpath = os.path.join(work, 'code.bin')
    run([OBJCOPY, '-O', 'binary', '-j', CODE_OUT, elf, binpath])
    with open(binpath, 'rb') as f:
        ours = f.read()

    # 3. bytes
    problems = []
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
                out('OK %s 0x%X 0x%X' % (r.label, r.addr, r.length))
        return True

    out('FAIL %s (%s, 0x%X..0x%X, %d relocation(s))'
        % (src, ' '.join(r.label for r in routines), lo, hi, nrel))
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

def compile_pas(src, dpc):
    if not os.path.exists(dpc):
        raise SetupError('%s not found (run `make build`)' % dpc)
    stem = os.path.splitext(os.path.basename(src))[0]
    work = os.path.join(SCRATCH, stem)
    os.makedirs(work, exist_ok=True)
    out = os.path.join(work, stem + '.s')
    run([dpc, '-o', out, src])
    return out


def run_manifest(path, linkmap, dpc):
    """Each non-comment line: FILE SPEC... [--section-addr SEC=0xADDR]...
    FILE is relative to the repository root; .pas files are compiled with
    dpc first.  Returns (failing entries, entries with setup errors)."""
    failures = errors = 0
    with open(path) as f:
        lines = [(i, l.split('#', 1)[0].split()) for i, l in enumerate(f, 1)]
    for lineno, toks in lines:
        if not toks:
            continue
        src = os.path.join(REPO, toks[0])
        specs, sect = [], {}
        rest = iter(toks[1:])
        for t in rest:
            if t == '--section-addr':
                k, v = parse_section_addr(next(rest, ''))
                sect[k] = v
            else:
                specs.append(t)
        try:
            if src.endswith('.pas'):
                src = compile_pas(src, dpc)
            ok = check(src, specs, linkmap, sect)
        except SetupError as e:
            print('ERROR %s:%d: %s' % (path, lineno, e))
            errors += 1
            continue
        failures += not ok
    return failures, errors


def parse_section_addr(s):
    name, eq, addr = s.partition('=')
    if not eq or not name:
        raise SetupError('--section-addr wants SEC=0xADDR, got %r' % s)
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
            return 0 if check(args.file, args.specs, linkmap, sect, args.len) else 1
        bad, errors = run_manifest(args.file, linkmap, args.dpc)
        return 2 if errors else 1 if bad else 0
    except SetupError as e:
        print('conform: %s' % e, file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
