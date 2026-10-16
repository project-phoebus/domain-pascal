#!/usr/bin/env python3
"""elemupdate.py - array elements updated by a constant in the compiled
code of the SR10.2 image: in place or loaded, changed and stored.

Evidence tool for docs/codegen-patterns.md section 8, "Array elements"
(bead domain-pascal-9pkq; the cause of the split is bead
domain-pascal-1bme).  Compiled routines are split and followed as
tools/cmpsurvey.py does (tools/arithsurvey.routines, a5survey.reached:
only the instructions reachable from the routine's start).  A site is

    inplace  `addq'/`subq'/`addi'/`subi' #c to an element operand: an
             indexed operand `(d,An,Dx)' (form `idx') or `(d,Am)' within
             five instructions after `lea (d,An,Dx),Am' (form `lea');
    lms      `move.<s> E,Dn', one to four instructions changing Dn (the
             first an `addq'/`subq'/`addi'/`subi' #c, for form `idx';
             any, form `lea'), then `move.<s> Dn,E' through the same
             element operand E.

`andi'/`ori'/`bset'/`bclr' to elements are left out (they cannot be told
from stores into packed fields).  For each site: the size, the
operation, the constant, the base (`a5', `fp', `abs' a `lea abs.l,An',
`sym' a `movea.l #imm,An', `reg' set some other way in the statement,
`held' not set in the statement), the index (`absw' a word loaded from
an absolute address, `mem' loaded from other memory, `reg' a copy of a
register or not set in the statement, `calc' computed), whether the
index is `.w' or `.l', and the map segment.

`plain' lists loads, changes (`addq'/`subq'/`addi'/`subi') and stores
back of an operand without an index register, the instructions right
after one another (6: 3 are the `lea' sites above, the others objects
through a register set by an earlier statement).

usage:
    tools/elemupdate.py summary      # counts by kind, form, base, index, size
    tools/elemupdate.py list [KIND]  # one line per site (KIND inplace|lms)
    tools/elemupdate.py plain        # load, change, store of other operands

Python 3 standard library + m68k-elf-objdump.  Read-only.
"""

import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import a5survey  # noqa: E402
import arithsurvey  # noqa: E402
import sau2map  # noqa: E402

IDX = r'%(a\d|fp)@\(([0-9a-f]*),%([ad]\d):([wl])\)'
DSP = r'%(a\d)@\((-?\d+)\)'
UPD = r'(addq|subq|addi|subi)([bwl]) #(-?\d+)'
INPLACE = re.compile(r'^' + UPD + ',(' + IDX + ')$')
INPLACE_DSP = re.compile(r'^' + UPD + ',' + DSP + '$')
LOAD = re.compile(r'^move([bwl]) (' + IDX + r'),%(d\d)$')
LOAD_DSP = re.compile(r'^move([bwl]) (' + DSP + r'),%(d\d)$')
LEA = re.compile(r'^lea ' + IDX + r',%(a\d)$')
STOP = re.compile(r'^(jsr|bsr|b[a-z]+ |rts|dbf|jmp)')
MEM = r'(?:%(?:a[0-6]|fp)@(?:\([^):]*\))?|0x[0-9a-f]+)'
PLAIN_LOAD = re.compile(r'^move([bwl]) (' + MEM + r'),%(d\d)$')


def setter(ins, i, reg):
    """The instruction of the statement before i that last writes reg
    fully, and every instruction between that changes it: [text]."""
    out = []
    for j in range(i - 1, max(-1, i - 12), -1):
        t = ins[j][2]
        if STOP.match(t):
            break
        ops = t.split(None, 1)
        if len(ops) > 1 and ops[1].endswith(',%' + reg):
            out.append(t)
            if t.startswith(('move', 'lea', 'clr', 'moveq')) and not t.startswith('movem'):
                break
    return list(reversed(out))


def base_kind(ins, i, reg):
    if reg in ('a5', 'fp'):
        return reg
    s = setter(ins, i, reg)
    if not s:
        return 'held'
    if re.match(r'^lea 0x[0-9a-f]+,', s[0]):
        return 'abs'
    if s[0].startswith('moveal #'):
        return 'sym'
    return 'reg'


def index_kind(ins, i, reg):
    s = setter(ins, i, reg)
    if not s:
        return 'reg'
    first = s[0]
    if re.match(r'^movew 0x[0-9a-f]+,', first):
        k = 'absw'
    elif re.match(r'^move[bwl] %(d\d|a\d),', first):
        k = 'reg'
    elif first.startswith('move'):
        k = 'mem'
    else:
        return 'calc'
    rest = s[1:]
    if any(not re.match(r'^(addw|addl) %(d\d),%\2$|^ls[lr][wl] #\d+,', t) for t in rest):
        return 'calc'
    return k


def sites(ins, seg='?'):
    """The element update sites of one routine's instructions ins
    [(addr, len, text)]: [dict]."""
    out = []
    n = len(ins)

    def add(i, kind, form, size, op, c, base, idx, width):
        out.append(dict(addr=ins[i][0], kind=kind, form=form, size=size, op=op,
                        c=int(c), base=base_kind(ins, i, base) if form == 'idx' else base,
                        index=index_kind(ins, i, idx) if form == 'idx' else idx,
                        width=width, seg=seg))

    for i, (a, _, t) in enumerate(ins):
        m = INPLACE.match(t)
        if m:
            add(i, 'inplace', 'idx', m.group(2), m.group(1), m.group(3),
                m.group(5), m.group(7), m.group(8))
            continue
        m = LOAD.match(t)
        if m:
            sz, ea, dn = m.group(1), m.group(2), m.group(7)
            for k in range(1, 5):
                if i + k + 1 >= n:
                    break
                if ins[i + k + 1][2] == 'move%s %%%s,%s' % (sz, dn, ea):
                    u = re.match('^' + UPD + ',%' + dn + '$', ins[i + 1][2])
                    if u:
                        add(i, 'lms', 'idx', sz, u.group(1), u.group(3),
                            m.group(3), m.group(5), m.group(6))
                    break
            continue
        m = LEA.match(t)
        if not m:
            continue
        am = m.group(5)
        base = base_kind(ins, i, m.group(1))
        idx = index_kind(ins, i, m.group(3))
        for j in range(i + 1, min(n, i + 6)):
            u = ins[j][2]
            mm = INPLACE_DSP.match(u)
            if mm and mm.group(4) == am:
                out.append(dict(addr=ins[j][0], kind='inplace', form='lea', size=mm.group(2),
                                op=mm.group(1), c=int(mm.group(3)), base=base, index=idx,
                                width=m.group(4), seg=seg))
                break
            mm = LOAD_DSP.match(u)
            if mm and mm.group(3) == am:
                dn = mm.group(5)
                for k in range(1, 4):
                    if j + k + 1 >= n:
                        break
                    if ins[j + k + 1][2] == 'move%s %%%s,%s' % (mm.group(1), dn, mm.group(2)):
                        ch = [ins[j + q][2] for q in range(1, k + 1)]
                        up = [re.match('^' + UPD + ',%' + dn + '$', x) for x in ch]
                        up = [x for x in up if x]
                        if up:
                            out.append(dict(addr=ins[j][0], kind='lms', form='lea',
                                            size=mm.group(1), op=up[0].group(1),
                                            c=int(up[0].group(3)), base=base, index=idx,
                                            width=m.group(4), seg=seg))
                        break
                break
            if STOP.match(u) or u.endswith(',%' + am):
                break
    return out


def plain_sites(ins, seg='?'):
    """Loads, changes and stores back of an operand without an index
    register, one right after the other: [(addr, text)]."""
    out = []
    for i in range(len(ins) - 2):
        m = PLAIN_LOAD.match(ins[i][2])
        if not m:
            continue
        sz, ea, dn = m.group(1), m.group(2), m.group(3)
        if re.match('^' + UPD + ',%' + dn + '$', ins[i + 1][2]) and \
                ins[i + 2][2] == 'move%s %%%s,%s' % (sz, dn, ea):
            out.append((ins[i][0], seg, ins[i][2] + '; ' + ins[i + 1][2]))
    return out


def scan():
    m = sau2map.load()
    segs = [s for s in m.segments if s.kind == 'I']
    res, plain = [], []
    for r0, r1, name in arithsurvey.routines():
        seg = next((s.name for s in segs if s.addr <= r0 < s.end), '?')
        got = a5survey.reached(r0, r1)
        ins = sorted(got.values()) if isinstance(got, dict) else list(got)
        for s in sites(ins, seg):
            s['routine'] = name
            res.append(s)
        plain.extend(plain_sites(ins, seg))
    return res, plain


def main(argv):
    if not argv or argv[0] not in ('summary', 'list', 'plain'):
        print(__doc__)
        return 2
    res, plain = scan()
    if argv[0] == 'plain':
        for a, seg, t in plain:
            print('0x%06X %-12s %s' % (a, seg, t))
        return 0
    if argv[0] == 'list':
        for s in res:
            if len(argv) > 1 and s['kind'] != argv[1]:
                continue
            print('0x%06X %-8s %-4s %s.%s #%d base=%-4s index=%-4s .%s %-12s %s' % (
                s['addr'], s['kind'], s['form'], s['op'], s['size'], s['c'], s['base'],
                s['index'], s['width'], s['seg'], s['routine']))
        return 0
    kinds = Counter(s['kind'] for s in res)
    print('sites: %s' % ', '.join('%s %d' % kv for kv in sorted(kinds.items())))
    print('constants: %s' % ', '.join('%s #%d: %d' % (k, c, n) for (k, c), n in
                                       sorted(Counter((s['kind'], s['c']) for s in res).items())))
    cells = Counter((s['form'], s['base'], s['index'], s['size'], s['kind']) for s in res)
    print('form base index size: inplace / lms')
    for key in sorted({k[:4] for k in cells}):
        print('  %-4s %-4s %-4s %s: %d / %d' % (key + (cells[key + ('inplace',)],
                                                     cells[key + ('lms',)])))
    print('plain load, change, store: %d' % len(plain))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
