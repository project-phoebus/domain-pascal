#!/usr/bin/env python3
"""nilsurvey.py - longword zeros (`nil' and the integer 0) stored, pushed
and loaded into address registers, in the compiled code of the SR10.2
image.

Evidence tool for docs/codegen-patterns.md section 13 ("Constants
stored", `nil') and 1.5 (zero pushes), bead domain-pascal-j4ip.
Compiled routines are split and followed as tools/boolsurvey.py does
(tools/arithsurvey.py `routines', tools/a5survey.py `reached': map
symbols and `link.w A6' words, hand assembly excluded, only the
instructions reachable from the routine's start).  The image cannot tell
`nil' from the integer 0; the sources in ../source can, per site.
But it can tell an address from a constant: the binder's RELOC table
(after the last loaded segment, VA 0xE88834; ../source
docs/rfc-cold-start.md step 8: a word count, then the load addresses
of the 32-bit cells that hold a VA) lists the immediate of a `move.l
#0' or `movea.l #0' that is the address of a symbol at 0 (the map's
EC_$NIL_EC or TRAP_PAGE).  Such a form is printed with ` [reloc]'.
Kinds:

    store   a longword 0 written to memory other than a push: `clr.l
            <ea>' or `move.l #0,<ea>' (the form), and the operand's kind
            (frame `(d,A6)', unit `(d,A5)', absolute, through an address
            register `(d,An)'/`(An)', `(An)+', indexed).
    push    a longword 0 pushed: `clr.l -(SP)', `move.l #0,-(SP)' or
            `pea (0).w', and the routine the next call reaches (map name,
            or `0xADDR' for a module-local one).
    areg    a 0 loaded into an address register: `suba.l An,An' or
            `movea.l #0,An', and the first instruction after it that
            reads An (`pea (An)' a push of its value, `movea.l An,A0' a
            pointer result, `cmpa' a test, `(d,An)' a base, `copy' a move
            from it), or what comes first: `branch', `call', `rts',
            `overwritten' (An written again).  Only straight-line code
            after the load is looked at.

usage:
    tools/nilsurvey.py summary         # the counts quoted in codegen-patterns
    tools/nilsurvey.py list KIND       # one line per site of KIND

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import struct
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

KINDS = ('store', 'push', 'areg')

_STORE = re.compile(r'^(clrl|movel #0,)\s*(\S+)$')
_PUSH0 = re.compile(r'^(clrl %sp@-|movel #0,%sp@-|pea 0x0)$')
_AREG = re.compile(r'^(subal (%a\d),\2|moveal #0,(%a\d))$')
_CALL = re.compile(r'^(jsr|bsr[sw]?) (0x[0-9a-f]+)$')
_END = re.compile(r'^(b\w+|jmp|jsr|rts|rte|dbf)\b')


RELOC_VA = 0xE88834     # map: I20 E88834 RELOC
LOAD_KERNEL = 0x102000  # load address of VA 0xE00800 (WIRED_PROC)


def reloc_cells(img, off=0xDFFC00, lo=0xE00800):
    """The VAs of the kernel cells the RELOC table of IMG (file offset
    = VA - OFF) lists; cells below the kernel (COLD, DUMP) are left out."""
    o = RELOC_VA - off
    n = struct.unpack('>H', img[o:o + 2])[0]
    cells = struct.unpack('>%dI' % n, img[o + 2:o + 2 + 4 * n])
    return frozenset(c - LOAD_KERNEL + lo for c in cells if c >= LOAD_KERNEL)


def _imm(form, a, relocs):
    """FORM, marked when the immediate at A + 2 is a RELOC cell."""
    return form + ' [reloc]' if form.endswith('#0') and a + 2 in relocs else form


def ea_kind(ea):
    """The kind of a memory operand in objdump's syntax."""
    if re.match(r'^%(a\d|fp)@\(-?[0-9a-f]*,%[ad]\d:[wl]\)$', ea):
        return 'indexed'
    if ea.startswith('%fp@'):
        return 'frame'
    if ea.startswith('%a5@'):
        return 'unit'
    if re.match(r'^0x[0-9a-f]+$', ea):
        return 'abs'
    if re.match(r'^%a\d@(\(-?\d+\))?$', ea):
        return 'areg'
    if re.match(r'^%a\d@\+$', ea):
        return 'postinc'
    return 'other'


def store_sites(ins, relocs=frozenset()):
    """[(addr, form, kind)] of the longword-zero stores to memory."""
    out = []
    for a, _n, t in ins:
        m = _STORE.match(t)
        if not m or m.group(2) == '%sp@-' or re.match(r'^%[ad]\d$', m.group(2)):
            continue
        form = 'clr.l' if m.group(1) == 'clrl' else 'move.l #0'
        out.append((a, _imm(form, a, relocs), ea_kind(m.group(2))))
    return out


def push_sites(ins, name=lambda x: '0x%X' % x, relocs=frozenset()):
    """[(addr, form, callee)] of the longword-zero pushes; the callee is
    the next call's target, named with NAME."""
    out = []
    for i, (a, _n, t) in enumerate(ins):
        if not _PUSH0.match(t):
            continue
        form = {'clrl': 'clr.l', 'movel': 'move.l #0', 'pea': 'pea (0).w'}[t.split()[0]]
        callee = '-'
        for _a2, _n2, t2 in ins[i + 1:]:
            c = _CALL.match(t2)
            if c:
                callee = name(int(c.group(2), 16))
                break
        out.append((a, _imm(form, a, relocs), callee))
    return out


def _reader(ins, i, reg):
    """What first reads REG after ins[i] (see the module comment)."""
    for _a, _n, t in ins[i + 1:]:
        if t in ('pea %s@' % reg,):
            return 'pea'
        if t == 'moveal %s,%%a0' % reg:
            return 'result'
        if t.startswith('cmpaw') and t.endswith(reg):
            return 'cmpa'
        if (reg + '@') in t:
            return 'base'
        ops = t.split(None, 1)[1] if ' ' in t else ''
        src = ops.split(',')[0] if ops else ''
        if src == reg:
            return 'copy'
        if ops.endswith(',' + reg):
            return 'overwritten'
        e = _END.match(t)
        if e:
            return {'jsr': 'call', 'rts': 'rts', 'rte': 'rts'}.get(
                e.group(1), 'call' if e.group(1).startswith('bsr') else 'branch')
    return '-'


def areg_sites(ins, relocs=frozenset()):
    """[(addr, form, reader)] of the zero loads into address registers."""
    out = []
    for i, (a, _n, t) in enumerate(ins):
        m = _AREG.match(t)
        if not m:
            continue
        reg = m.group(2) or m.group(3)
        form = 'suba.l' if m.group(2) else 'movea.l #0'
        out.append((a, _imm(form, a, relocs), _reader(ins, i, reg)))
    return out


def scan():
    import a5survey  # noqa: E402
    import arithsurvey  # noqa: E402
    import loopalign  # noqa: E402
    import sau2map  # noqa: E402
    m = sau2map.load()
    relocs = reloc_cells(loopalign.image())

    def name(x):
        n = m.names_at(x)
        return n[0] if n else '0x%X' % x
    res = {k: [] for k in KINDS}
    for r0, r1, rname in arithsurvey.routines():
        ins = a5survey.reached(r0, r1)
        res['store'] += [(r0, rname) + s for s in store_sites(ins, relocs)]
        res['push'] += [(r0, rname) + s for s in push_sites(ins, name, relocs)]
        res['areg'] += [(r0, rname) + s for s in areg_sites(ins, relocs)]
    return res


def first(sites, n=4):
    return ', '.join('0x%X' % s[2] for s in sites[:n])


def counts(title, sites, key):
    print('%s: %d' % (title, len(sites)))
    for k, v in sorted(Counter(key(s) for s in sites).items(), key=lambda kv: -kv[1]):
        print('  %-44s %4d  e.g. %s' % (' '.join(str(x) for x in k), v,
                                        first([s for s in sites if key(s) == k])))


def summary(res):
    counts('longword zero stored to memory (form, operand)', res['store'],
           lambda s: (s[3], s[4]))
    counts('longword zero pushed (form)', res['push'], lambda s: (s[3],))
    counts('longword zero pushed other than by clr.l (form, callee)',
           [s for s in res['push'] if s[3] != 'clr.l'], lambda s: (s[3], s[4]))
    counts('zero into an address register (form, first reader)', res['areg'],
           lambda s: (s[3], s[4]))


def main(argv):
    if not argv or argv[0] not in ('summary', 'list') or (
            argv[0] == 'list' and (len(argv) < 2 or argv[1] not in KINDS)):
        print(__doc__)
        return 2
    res = scan()
    if argv[0] == 'summary':
        summary(res)
    else:
        for s in res[argv[1]]:
            print('0x%06X %-28s %s' % (s[2], '%06X %s' % (s[0], s[1]),
                                      ' '.join(str(x) for x in s[3:])))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
